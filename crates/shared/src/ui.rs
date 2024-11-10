//! Ui utilities
use std::collections::HashSet;
use ammonia::Builder;
use comrak::{markdown_to_html, Options};

/// Render markdown input into HTML
pub fn render_markdown(input: &str) -> String {
    let mut options = Options::default();

    options.extension.table = true;
    options.extension.superscript = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options.extension.header_ids = Option::Some(String::new());
    options.extension.tagfilter = true;
    options.render.unsafe_ = true;
    // options.render.escape = true;
    options.parse.smart = false;

    let html = markdown_to_html(input, &options);

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
        .replace("src=\"", "src=\"/api/v0/util/ext/image?img=")
}
