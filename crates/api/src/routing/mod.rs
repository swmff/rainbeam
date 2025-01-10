pub mod api;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct PaginatedQuery {
    #[serde(default)]
    pub page: i32,
}
