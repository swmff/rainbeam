pub mod chats;
pub mod circles;
pub mod comments;
pub mod messages;
pub mod pages;
pub mod profiles;
pub mod questions;
pub mod reactions;
pub mod responses;
pub mod util;

use crate::database::Database;
use axum::Router;
use hcaptcha::Hcaptcha;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Hcaptcha)]
pub struct CreateReport {
    content: String,
    #[captcha]
    token: String,
}

pub fn routes(database: Database) -> Router {
    Router::new()
        .nest("/util", util::routes(database.clone()))
        .nest("/questions", questions::routes(database.clone()))
        .nest("/responses", responses::routes(database.clone()))
        .nest("/comments", comments::routes(database.clone()))
        .nest("/reactions", reactions::routes(database.clone()))
        .nest("/circles", circles::routes(database.clone()))
        .nest("/profiles", profiles::routes(database.clone()))
        .nest("/chats", chats::routes(database.clone()))
        .nest("/messages", messages::routes(database.clone()))
        .nest("/pages", pages::routes(database.clone()))
}
