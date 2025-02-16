//! Xsu Utilities
#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/swmff/rainbeam/issues/")]
pub mod fs;
pub mod hash;
pub mod process;
pub mod snow;
pub mod ui;
pub mod config;

// ...
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{TimeZone, Utc};

/// Get a [`u128`] timestamp
pub fn unix_epoch_timestamp() -> u128 {
    let right_now = SystemTime::now();
    let time_since = right_now
        .duration_since(UNIX_EPOCH)
        .expect("Time travel is not allowed");

    return time_since.as_millis();
}

/// Get a [`i64`] timestamp from the given `year` epoch
pub fn epoch_timestamp(year: i32) -> i64 {
    let now = Utc::now().timestamp_millis();
    let then = Utc
        .with_ymd_and_hms(year, 1, 1, 0, 0, 0)
        .unwrap()
        .timestamp_millis();

    return now - then;
}
