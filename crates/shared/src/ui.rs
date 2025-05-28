use ammonia::Builder;
use markdown::{to_html_with_options, Options, CompileOptions, ParseOptions, Constructs};
use std::collections::HashSet;

/// Render markdown input into HTML
pub fn render_markdown(input: &str) -> String {
    let options = Options {
        compile: CompileOptions {
            allow_any_img_src: false,
            allow_dangerous_html: true,
            gfm_task_list_item_checkable: false,
            gfm_tagfilter: false,
            ..Default::default()
        },
        parse: ParseOptions {
            constructs: Constructs {
                gfm_autolink_literal: true,
                ..Default::default()
            },
            gfm_strikethrough_single_tilde: false,
            math_text_single_dollar: false,
            mdx_expression_parse: None,
            mdx_esm_parse: None,
            ..Default::default()
        },
    };

    let html = match to_html_with_options(input, &options) {
        Ok(h) => h,
        Err(e) => e.to_string(),
    };

    let mut allowed_attributes = HashSet::new();
    allowed_attributes.insert("id");
    allowed_attributes.insert("class");
    allowed_attributes.insert("ref");
    allowed_attributes.insert("aria-label");
    allowed_attributes.insert("lang");
    allowed_attributes.insert("title");
    allowed_attributes.insert("align");
    allowed_attributes.insert("src");

    Builder::default()
        .generic_attributes(allowed_attributes)
        .add_tags(&[
            "video", "source", "img", "b", "span", "p", "i", "strong", "em", "a",
        ])
        .rm_tags(&["script", "style", "link", "canvas"])
        .add_tag_attributes("a", &["href", "target"])
        .clean(&html)
        .to_string()
        .replace(
            "src=\"",
            "loading=\"lazy\" src=\"/api/v0/util/ext/image?img=",
        )
        .replace("<video loading=", "<video controls loading=")
        .replace("--&gt;", "<align class=\"right\">")
        .replace("-&gt;", "<align class=\"center\">")
        .replace("&lt;-", "</align>")
}
