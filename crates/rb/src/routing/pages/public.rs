//! Public pages (homepage, auth)
use axum::{
    extract::State,
    response::{IntoResponse, Html},
    http::StatusCode,
};

use crml::template;

use crate::ToHtml;
use crate::config::Config;
use crate::database::Database;
use crate::model::DatabaseError;

use langbeam::LangFile;
use authbeam::model::Profile;

#[template("public/error")]
pub struct ErrorTemplate {
    pub config: Config,
    pub lang: LangFile,
    pub profile: Option<Box<Profile>>,
    pub message: String,
    pub head: String,
}

pub async fn not_found(State(database): State<Database>) -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Html(DatabaseError::NotFound.to_html(database)),
    )
}
