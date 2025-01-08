pub mod api;
pub mod pages;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct PaginatedQuery {
    #[serde(default)]
    pub page: i32,
}
