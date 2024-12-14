//! 🌈 Rainbeam!
#![doc = include_str!("../../../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/swmff/rainbeam/issues")]
#![doc(html_favicon_url = "https://rainbeam.net/static/favicon.svg")]
#![doc(html_logo_url = "https://rainbeam.net/static/favicon.svg")]
use askama_axum::Template;

pub use rainbeam::database;
pub use rainbeam::config;
pub use rainbeam::model;
pub mod routing;

/// Trait to convert errors into HTML
pub(crate) trait ToHtml {
    fn to_html(&self, database: database::Database) -> String;
}

impl ToHtml for model::DatabaseError {
    fn to_html(&self, database: database::Database) -> String {
        crate::routing::pages::ErrorTemplate {
            config: database.config.clone(),
            lang: database.lang("net.rainbeam.langs:en-US"),
            profile: None,
            message: self.to_string(),
        }
        .render()
        .unwrap()
    }
}
