use crate::{config::PageConfig, confluence::ConfluenceClient, error::Result, FQDN, SECRET, USER};
use comrak::{
    arena_tree::Node,
    format_html,
    nodes::{Ast, NodeValue},
    parse_document, Arena, Options,
};
use normalize_path::NormalizePath;
use serde::Deserialize;
use std::{cell::RefCell, collections::HashSet, path::Path};
use tracing::{debug, instrument, warn, Level};

type NodeRef<'a> = &'a Node<'a, RefCell<Ast>>;

// ###################################################### //
//                   Image Align Enum                     //
// ###################################################### //

enum Align {
    Left,
    Right,
    Center,
}

impl Align {
    fn from_str(value: &str) -> Align {
        match value {
            "align-left" => Align::Left,
            "align-right" => Align::Right,
            "align-center" => Align::Center,
            _ => Align::Left,
        }
    }
}

impl std::fmt::Display for Align {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let align = match self {
            Align::Left => "left",
            Align::Right => "right",
            Align::Center => "center",
        };
        write!(f, "{align}")
    }
}

// ###################################################### //
//                    HTML Page Struct                    //
// ###################################################### //

#[derive(Deserialize, Debug)]
pub struct HtmlPage {
    pub image_paths: Vec<String>,
    pub page_header: Option<String>,
    pub html: String,
}

impl HtmlPage {
    #[instrument(skip_all, ret(level = Level::TRACE), err(Debug, level = Level::DEBUG))]
    pub async fn new(page_config: &PageConfig) -> Result<HtmlPage> {
        let md_file = std::fs::read_to_string(&page_config.file_path)?;
        let arena = Arena::new();

        let mut options = Options::default();
        options.extension.superscript = true;
        options.extension.table = true;

        let root_node = parse_document(&arena, &md_file, &options);

        let title = get_and_remove_h1_header(root_node);
        let image_paths = get_image_paths(root_node, &page_config.file_path);

        replace_codeblock_with_html(root_node);
        replace_image_node_with_html(root_node);
        replace_page_link(root_node).await?;

        if let Some(sup) = &page_config.superscript_header {
            let super_string = format!("^{sup}^");
            let super_node = parse_document(&arena, &super_string, &options);
            root_node.prepend(super_node);
        }

        let mut html = String::new();
        format_html(root_node, &options, &mut html)?;

        Ok(HtmlPage {
            image_paths,
            page_header: title,
            html,
        })
    }
}

// ###################################################### //
//       Helper functions for processesing markdown       //
// ###################################################### //

// Gets all filesystem paths for images for later uploading.
fn get_image_paths(root_node: NodeRef<'_>, md_file_path: &str) -> Vec<String> {
    let get_image_node_link = |node: NodeRef<'_>| match &node.data().value {
        NodeValue::Image(node_link) => Some(node_link.url.clone()),
        _ => None,
    };

    let is_relative_path = |path: String| match Path::new(&path).has_root() {
        true => {
            warn!("image path is not a relative path, skipping. [{}]", path);
            None
        }
        false => Some(path),
    };

    let is_valid_path = |path: String| match Path::new(&path).is_file() {
        true => Some(path),
        false => {
            warn!("image path not valid file path, skipping: [{}]", path);
            None
        }
    };

    // Join the *.md file path with the image path to convert the relative path to an absolut.
    let convert_to_full_path = |path: String| {
        Path::new(md_file_path)
            .with_file_name(&path)
            .normalize()
            .to_str()
            .map(|x| x.to_string())
    };

    root_node
        .descendants()
        .filter(|node| matches!(node.data().value, NodeValue::Image(_)))
        .filter_map(get_image_node_link)
        .filter(|x| !x.starts_with("https://") && !x.starts_with("http://"))
        .filter_map(is_relative_path)
        .filter_map(convert_to_full_path)
        .filter_map(is_valid_path)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<String>>()
}

// Retrieves the NodeValue::Text if the first child is a h1 header, and then removes it.
fn get_and_remove_h1_header(root_node: NodeRef<'_>) -> Option<String> {
    let first_child = root_node.first_child()?;
    let child_value = &first_child.data().value;

    if !matches!(child_value, NodeValue::Heading(heading) if heading.level == 1) {
        return None;
    }

    let mut title = String::new();

    for child in first_child.children() {
        if let NodeValue::Text(text) = &child.data().value {
            title.push_str(text);
        }
    }

    debug!("first child of root node is h1, detaching node.");
    first_child.detach();

    Some(title)
}

// Replaces all Image Nodes with custom HTML in Confluence storage format.
fn replace_image_node_with_html(root_node: NodeRef<'_>) {
    let image_nodes = root_node
        .descendants()
        .filter(|node| matches!(node.data().value, NodeValue::Image(_)))
        .collect::<Vec<_>>();

    for image_node in image_nodes {
        let mut image_node_data = image_node.data_mut();

        if let NodeValue::Image(node_link) = &image_node_data.value {
            if node_link.url.starts_with("https://") || node_link.url.starts_with("http://") {
                continue;
            }

            let file_name = Path::new(&node_link.url)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");

            let mut alignment = Align::Left;

            if let Some(child) = image_node.first_child() {
                if let NodeValue::Text(image_alt_text) = &child.data().value {
                    alignment = Align::from_str(image_alt_text);
                }
            }

            let raw_html_node = format!(
                r#"
                    <p>
                    <ac:image ac:align="{alignment}">
                        <ri:attachment ri:filename="{file_name}" />
                    </ac:image>
                    </p>
                "#
            )
            .trim()
            .to_string();

            image_node_data.value = NodeValue::Raw(raw_html_node);

            // Remove all children of the NodeValue::Image node.
            let children: Vec<_> = image_node.children().collect();
            for child in children {
                child.detach();
            }
        };
    }
}

// Replaces all CodeBlock nodes with custom HTML in Confluence storage format.
fn replace_codeblock_with_html(root_node: NodeRef<'_>) {
    let codeblock_nodes: Vec<_> = root_node
        .descendants()
        .filter(|node| matches!(node.data().value, NodeValue::CodeBlock(_)))
        .collect();

    for node in codeblock_nodes {
        let mut node_data = node.data_mut();

        if let NodeValue::CodeBlock(codeblock) = &node_data.value {
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

            node_data.value = NodeValue::Raw(raw_html_node)
        };
    }
}

// If a markdown link title contains a Confluence page id, this replaces it url with the Confluence Cloud WebUI url.
async fn replace_page_link(root_node: NodeRef<'_>) -> Result<()> {
    let link_nodes: Vec<_> = root_node
        .descendants()
        .filter(|node| matches!(node.data().value, NodeValue::Link(_)))
        .collect();

    let fqdn = FQDN.get().unwrap().to_string();
    let user = USER.get().unwrap().to_string();
    let secret = SECRET.get().unwrap().to_string();

    for node in link_nodes {
        // Borrowing node value in its own scope to prevent holding refcell across await point
        let link_title = {
            let node_value = &node.data().value;
            match node_value {
                NodeValue::Link(link) if !link.title.is_empty() => link.title.to_string(),
                _ => continue,
            }
        };

        let parts: Vec<&str> = link_title.split(":").collect();

        if parts.len() != 2 || parts[0] != "pid" {
            debug!("link title found but not a page id match: {}", link_title);
            continue;
        }

        let page_id = parts[1];
        debug!(page_id, "found page id match");

        let client = ConfluenceClient::new(&fqdn, &user, &secret)?;
        let page_url = client.get_page_link(page_id).await?;

        let node_value = &mut node.data_mut().value;
        if let NodeValue::Link(ref mut link) = node_value {
            link.url = page_url;
        }
    }
    Ok(())
}
