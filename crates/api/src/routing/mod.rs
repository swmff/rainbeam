pub mod api;
pub mod pages;
pub use pages::PaginatedQuery;

use serde::Serialize;
use serde_json::Value;

pub fn into_some_serde_value<T>(s: T) -> Option<Value>
where
    T: Serialize,
{
    Some(serde_json::to_value(s).unwrap())
}
