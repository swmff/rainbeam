//! Ui utilities
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use ammonia::Builder;
use pulldown_cmark::{Options, Parser};

/// Render markdown input into HTML
pub fn render_markdown(input: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options.insert(Options::ENABLE_GFM);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);

    let parser = Parser::new_ext(input, options);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);

    let mut allowed_attributes = HashSet::new();
    allowed_attributes.insert("id");
    allowed_attributes.insert("class");
    allowed_attributes.insert("ref");
    allowed_attributes.insert("aria-label");
    allowed_attributes.insert("lang");
    allowed_attributes.insert("title");
    allowed_attributes.insert("align");

    Builder::default()
        .generic_attributes(allowed_attributes)
        .clean(&html)
        .to_string()
        .replace("src=\"", "src=\"/api/util/ext/image?img=")
}

// ...
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum BlockType {
    /// Embedded markdown
    Markdown,
    /// An embedded DocShare document
    Document,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Block {
    /// The block header
    pub title: String,
    /// The type of this block
    pub r#type: BlockType,
    /// The content this block will use
    pub content: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BlockList {
    /// The version of the block list definition file
    #[serde(default)]
    pub version: i32,
    /// A list of blocks
    #[serde(default)]
    pub blocks: Vec<Block>,
}

impl Default for BlockList {
    fn default() -> Self {
        Self {
            version: 1,
            blocks: Vec::new(),
        }
    }
}

/// Render a [`BlockList`] into HTML
pub fn render_blocklist(list: BlockList) -> String {
    let mut out = String::new();

    for block in list.blocks {
        match block.r#type {
            BlockType::Markdown => out.push_str(&format!(
                "<fieldset><legend>{}</legend>\n{}</fieldset>",
                block.title,
                render_markdown(&block.content)
            )),
            BlockType::Document => out.push_str(&format!(
                "<fieldset>
                    <legend>{}</legend>
                    <iframe src=\"/doc/~{}\" frameborder=\"0\" sandbox=\"allow-forms allow-scripts allow-same-origin\" onload=\"trigger('app:possess_iframe', [event.target])\"></iframe>
                </fieldset>",
                block.title, block.content
            )),
        }
    }

    // return
    out
}
