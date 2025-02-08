use crate::confluence::{ConfluenceClient, ConfluenceClientTrait};
use crate::error::{Error, Result};
use crate::{FQDN, SECRET, USER};
use tokio::task;

use comrak::{
    self, format_html,
    nodes::{NodeCodeBlock, NodeLink, NodeValue},
    parse_document, Arena, Options,
};
use tracing::warn;

#[derive(Debug)]
struct PageLink {
    id: u64,
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

pub fn render_markdown_file(file_path: &str, super_string: Option<&String>) -> Result<String> {
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
