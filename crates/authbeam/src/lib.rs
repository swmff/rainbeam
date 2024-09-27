//! Authentication manager with user accounts and simple group-based permissions.
#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/hkauso/xsu/issues/")]
pub mod api;
pub mod database;
pub mod model;

pub use database::{Database, ServerOptions};
pub use databeam::DatabaseOpts;
