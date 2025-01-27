use crate::error::{Error, Result};
use comrak::{self, format_html, nodes::NodeValue, parse_document, Arena, Options};

pub fn render_markdown_file(file_path: &str) -> Result<String> {
    let arena = Arena::new();

    let md_file = std::fs::read_to_string(file_path)?;
    let root = parse_document(&arena, &md_file, &Options::default());

    // Remove the first child node if it's a header level 1.
    if let Some(node) = root.first_child() {
        if let NodeValue::Heading(header) = node.data.borrow().value {
            if header.level == 1 {
                node.detach();
            }
        }
    }

    // Replacing codeblock nodes with a raw output nodes incapsulating the code value with
    // HTML that creates a CodeBlock macro in Confluence.
    for node in root.descendants() {
        let mut node_value = node.data.borrow_mut();

        if let NodeValue::CodeBlock(codeblock) = &node_value.value {
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
                codeblock.literal.trim()
            );

            node_value.value = NodeValue::Raw(raw_html_node);
        };
    }

    let mut html_bytes = vec![];
    format_html(root, &Options::default(), &mut html_bytes)?;
    let html = String::from_utf8(html_bytes)?;

    Ok(html)
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
