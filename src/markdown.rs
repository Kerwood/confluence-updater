use core::fmt;
use std::cell::RefCell;
use std::path::Path;

use crate::confluence::{ConfluenceClient, ConfluenceClientTrait};
use crate::error::{Error, Result};
use crate::{FQDN, SECRET, USER};
use comrak::{
    self, format_html,
    nodes::{NodeCodeBlock, NodeLink, NodeValue},
    parse_document, Arena, Options,
};
use comrak::{arena_tree::Node, nodes::Ast};
use normalize_path::NormalizePath;
use tokio::task;
use tracing::{info, warn};

enum Align {
    Left,
    Right,
    Center,
}

impl Align {
    fn from_str(value: &str) -> Option<Align> {
        match value {
            "align-left" => Some(Align::Left),
            "align-right" => Some(Align::Right),
            "align-center" => Some(Align::Center),
            _ => None,
        }
    }
}

impl fmt::Display for Align {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let align = match self {
            Align::Left => "left",
            Align::Right => "right",
            Align::Center => "center",
        };
        write!(f, "{}", align)
    }
}

struct Client {}

impl ConfluenceClientTrait for Client {
    fn fqdn(&self) -> String {
        FQDN.get().unwrap().to_string()
    }

    fn username(&self) -> String {
        USER.get().unwrap().to_string()
    }

    fn secret(&self) -> String {
        SECRET.get().unwrap().to_string()
    }
}

#[derive(Debug)]
struct PageLink {
    id: u64,
}

impl PageLink {
    fn try_option_into(input: &str) -> Option<Self> {
        let parts: Vec<&str> = input.split(":").collect();
        if parts.len() != 2 || parts[0] != "pid" {
            return None;
        }

        match parts[1].trim().parse::<u64>() {
            Ok(id) => Some(PageLink { id }),
            Err(_) => {
                let error = Error::LinkIdMissing(input.to_string());
                warn!(%error);
                None
            }
        }
    }
}

pub fn render_markdown_file(
    page_id: &str,
    file_path: &str,
    super_string: Option<&String>,
) -> Result<String> {
    let arena = Arena::new();

    let md_file = std::fs::read_to_string(file_path)?;

    let mut options = Options::default();
    options.extension.superscript = true;

    let root = parse_document(&arena, &md_file, &options);

    // Remove the first child node if it's a header level 1.
    if let Some(node) = root.first_child() {
        if let NodeValue::Heading(header) = node.data.borrow().value {
            if header.level == 1 {
                node.detach();
            }
        }
    }

    if let Some(sup) = super_string {
        let super_string = format!("^{}^", sup);
        let super_node = parse_document(&arena, &super_string, &options);
        root.prepend(super_node);
    }

    let image_nodes: Vec<_> = root
        .descendants()
        .filter(|node| matches!(node.data.borrow().value, NodeValue::Image(_)))
        .collect();

    process_image_nodes(page_id, file_path, image_nodes)?;

    for node in root.descendants() {
        let mut node_value = node.data.borrow_mut();

        // Replacing the url value of a NodeLink with the Confluence page url if the link title matches "pid:<id>"
        if let NodeValue::Link(ref mut node_link) = node_value.value {
            replace_node_link(node_link)?;
        }

        // Replacing codeblock nodes with a raw output nodes incapsulating the code value with
        // HTML that creates a CodeBlock macro in Confluence.
        if let NodeValue::CodeBlock(codeblock) = &node_value.value {
            node_value.value = replace_with_codeblock_macro(codeblock);
        };
    }

    let mut html_bytes = vec![];
    format_html(root, &Options::default(), &mut html_bytes)?;
    let html = String::from_utf8(html_bytes)?;

    Ok(html)
}

fn replace_node_link(node_link: &mut NodeLink) -> Result<()> {
    if let Some(page_link) = PageLink::try_option_into(&node_link.title) {
        let client = ConfluenceClient::new(&Client {})?;
        let id = page_link.id.to_string();
        let page_link_res = task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(client.get_page_link(&id))
        })?;
        node_link.url = page_link_res;
    }
    Ok(())
}

fn process_image_nodes<'a>(
    page_id: &str,
    file_path: &str,
    image_nodes: Vec<&'a Node<'a, RefCell<Ast>>>,
) -> Result<()> {
    for node in image_nodes {
        let mut node_link_path = match &node.data.borrow().value {
            NodeValue::Image(node_link) => node_link.url.clone(),
            _ => return Err(Error::ImageLinkMissing),
        };

        if Path::new(&node_link_path).has_root() {
            warn!(
                "image path is not a relative path, skipping. [{}]",
                node_link_path
            );
            continue;
        }

        node_link_path = match Path::new(file_path)
            .with_file_name(&node_link_path)
            .normalize()
            .to_str()
        {
            Some(s) => s.to_string(),
            None => {
                warn!(
                    "image path is not valid UTF-8, skipping. [{}]",
                    node_link_path
                );
                continue;
            }
        };

        if !Path::new(&node_link_path).is_file() {
            warn!("image path not valid, skipping: [{}]", &node_link_path);
            continue;
        }

        let client = ConfluenceClient::new(&Client {})?;
        task::block_in_place(|| {
            info!("uploading attachment [{}]", &node_link_path);
            tokio::runtime::Handle::current()
                .block_on(client.upload_attachment(page_id, &node_link_path))
        })?;

        // Get image alignment property from the NodeValue::Text child.
        let mut align: Option<Align> = None;
        if let Some(child) = node.first_child() {
            if let NodeValue::Text(text) = &child.data.borrow().value {
                align = Align::from_str(text);
            }
        }

        // Replace the NodeValue::Image with NodeValue::Raw.
        node.data.borrow_mut().value = confluence_image_block(&node_link_path, align);

        // Remove all children of the NodeValue::Image node.
        let children: Vec<_> = node.children().collect();
        for child in children {
            child.detach();
        }
    }

    Ok(())
}

fn confluence_image_block(file_path: &str, alignment: Option<Align>) -> NodeValue {
    let get_file_name = match Path::new(file_path).file_name() {
        Some(name) => name.to_str(),
        None => None,
    };

    let file_name = match get_file_name {
        Some(name) => name,
        None => {
            warn!("could not get file name for image: {}", file_path);
            ""
        }
    };

    let align = match alignment {
        Some(align) => align.to_string(),
        None => "left".to_string(),
    };

    let raw_html_node = format!(
        r#"
             <ac:image ac:align="{}">
               <ri:attachment ri:filename="{}" />
             </ac:image>
        "#,
        align, file_name
    )
    .trim()
    .to_string();

    NodeValue::Raw(raw_html_node)
}

fn replace_with_codeblock_macro(codeblock: &NodeCodeBlock) -> NodeValue {
    let language = match codeblock.info.is_empty() {
        true => "plaintext",
        false => &codeblock.info,
    };

    let raw_html_node = format!(
        r#"
            <ac:structured-macro ac:name="code" ac:schema-version="1">
            <ac:parameter ac:name="language">{}</ac:parameter>
            <ac:plain-text-body><![CDATA[{}]]></ac:plain-text-body>
            </ac:structured-macro>
        "#,
        language,
        codeblock.literal.trim_end()
    );

    NodeValue::Raw(raw_html_node)
}

pub fn get_page_title(file_path: &str) -> Result<String> {
    let arena = Arena::new();
    let md_file = std::fs::read_to_string(file_path)?;
    let root = parse_document(&arena, &md_file, &Options::default());

    let first_child = match root.first_child() {
        Some(node) => node,
        None => return Err(Error::PageHeaderMissing),
    };

    let header = match first_child.data.borrow().value {
        NodeValue::Heading(header) => header,
        _ => return Err(Error::PageHeaderMissing),
    };

    if header.level != 1 {
        return Err(Error::PageHeaderMissing);
    }

    let mut title = String::new();

    for child in first_child.children() {
        if let NodeValue::Text(text) = &child.data.borrow().value {
            title.push_str(text);
        }
    }

    Ok(title)
}
