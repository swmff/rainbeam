#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/hkauso/xsu/issues/")]
pub mod cachedb;
pub mod config;
pub mod database;
pub mod sql;
pub mod utility;

pub use cachedb::CacheDB;
pub use database::{DefaultReturn, StarterDatabase};
pub use sql::DatabaseOpts;
pub use sqlx::query;
