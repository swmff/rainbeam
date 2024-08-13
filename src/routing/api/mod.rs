pub mod comments;
pub mod profiles;
pub mod questions;
pub mod responses;

use crate::database::Database;
use axum::Router;

pub fn routes(database: Database) -> Router {
    Router::new()
        .nest("/questions", questions::routes(database.clone()))
        .nest("/responses", responses::routes(database.clone()))
        .nest("/comments", comments::routes(database.clone()))
        .nest("/profiles", profiles::routes(database.clone()))
    // .nest("/comments", comments::routes(database.clone()))
}
