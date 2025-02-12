use std::{cell::RefCell, collections::HashSet, path::Path, rc::Rc};

use comrak::{
    arena_tree::Node,
    format_html,
    nodes::{Ast, NodeCodeBlock, NodeValue},
    parse_document, Arena, Options,
};
use normalize_path::NormalizePath;
use tracing::warn;

type NodeRef<'a> = &'a Node<'a, RefCell<Ast>>;

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

impl std::fmt::Display for Align {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let align = match self {
            Align::Left => "left",
            Align::Right => "right",
            Align::Center => "center",
        };
        write!(f, "{}", align)
    }
}

use crate::error::{Error, Result};
struct HtmlPage {
    md_file_path: String,
    image_paths: Vec<String>,
    title: Option<String>,
    html: String,
}

//struct RenderMarkdownFile {}

#[allow(clippy::new_ret_no_self)]
impl HtmlPage {
    pub fn new(md_file_path: String, super_string: Option<&String>) -> Result<HtmlPage> {
        let md_file = std::fs::read_to_string(&md_file_path)?;
        let arena = Arena::new();

        let mut options = Options::default();
        options.extension.superscript = true;

        let root_node = parse_document(&arena, &md_file, &options);
        let title = get_h1_header(root_node);

        if let Some(sup) = super_string {
            let super_string = format!("^{}^", sup);
            let super_node = parse_document(&arena, &super_string, &options);
            root_node.prepend(super_node);
        }

        remove_h1_header(root_node);
        replace_codeblock_with_html(root_node);
        replace_image_node_with_html(root_node);
        let image_paths = get_image_paths(root_node, &md_file_path);

        let mut html_bytes = vec![];
        format_html(root_node, &options, &mut html_bytes)?;

        Ok(HtmlPage {
            md_file_path,
            image_paths,
            title,
            html: String::from_utf8(html_bytes)?,
        })
    }
}

// TODO: Should return absolute path from root
fn get_image_paths<'a>(root_node: NodeRef<'a>, md_file_path: &str) -> Vec<String> {
    let is_node_link = |node: NodeRef<'a>| match &node.data.borrow().value {
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
            warn!("image path not valid, skipping: [{}]", path);
            None
        }
    };

    // Join the *.md file path with the image path to convert a relative path to an absolut.
    let convert_to_full_path = |path: String| {
        Path::new(md_file_path)
            .with_file_name(&path)
            .normalize()
            .to_str()
            .map(|x| Some(x.to_string()))
    };

    root_node
        .descendants()
        .filter(|node| matches!(node.data.borrow().value, NodeValue::Image(_)))
        .filter_map(is_node_link)
        .filter_map(is_relative_path)
        .filter_map(is_valid_path)
        .filter_map(convert_to_full_path)
        .flatten()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<String>>()
}

// TODO: Maybe use the type alias NodeRef<'a>
fn remove_h1_header<'a>(root_node: &'a Node<'a, RefCell<Ast>>) {
    if let Some(node) = root_node.first_child() {
        if let NodeValue::Heading(header) = node.data.borrow().value {
            if header.level == 1 {
                node.detach();
            }
        }
    }
}

fn get_h1_header<'a>(root_node: &'a Node<'a, RefCell<Ast>>) -> Option<String> {
    let first_child = root_node.first_child()?;
    let child_value = &first_child.data.borrow().value;

    if !matches!(child_value, NodeValue::Heading(heading) if heading.level == 1) {
        return None;
    }

    let mut title = String::new();

    for child in first_child.children() {
        if let NodeValue::Text(text) = &child.data.borrow().value {
            title.push_str(text);
        }
    }

    Some(title)
}

fn replace_image_node_with_html<'a>(root_node: &'a Node<'a, RefCell<Ast>>) {
    let image_nodes = root_node
        .descendants()
        .filter(|node| matches!(node.data.borrow().value, NodeValue::Image(_)))
        .collect::<Vec<_>>();

    for image_node in image_nodes {
        let mut image_node_data = image_node.data.borrow_mut();

        if let NodeValue::Image(node_link) = &image_node_data.value {
            let path = Path::new(&node_link.url);

            // If it is not a path to a file, it could be a http link to an image. Then skip it.
            if !path.is_file() {
                continue;
            }

            let file_name = path
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default();

            let mut alignment: Option<Align> = None;

            if let Some(child) = image_node.first_child() {
                if let NodeValue::Text(image_alt_text) = &child.data.borrow().value {
                    alignment = Align::from_str(image_alt_text);
                }
            }

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

            image_node_data.value = NodeValue::Raw(raw_html_node)
        };
    }
}

fn replace_codeblock_with_html<'a>(root_node: &'a Node<'a, RefCell<Ast>>) {
    let codeblock_nodes: Vec<_> = root_node
        .descendants()
        .filter(|node| matches!(node.data.borrow().value, NodeValue::CodeBlock(_)))
        .collect();

    for node in codeblock_nodes {
        let mut node_data = node.data.borrow_mut();

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
