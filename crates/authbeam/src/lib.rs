//! Authentication manager with user accounts and simple group-based permissions.
#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/swmff/rainbeam/issues/")]
pub mod api;
pub mod avif;
pub mod database;
pub mod layout;
pub mod macros;
pub mod model;
pub mod permissions;

pub use database::{Database, ServerOptions};
pub use databeam::DatabaseOpts;
