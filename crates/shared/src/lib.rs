//! Xsu Utilities
#![doc = include_str!("../README.md")]
#![doc(issue_tracker_base_url = "https://github.com/hkauso/xsu/issues/")]
pub mod fs;
pub mod hash;
pub mod process;
pub mod ui;

// ...
use std::time::{SystemTime, UNIX_EPOCH};

/// Get a [`u128`] timestamp
pub fn unix_epoch_timestamp() -> u128 {
    let right_now = SystemTime::now();
    let time_since = right_now
        .duration_since(UNIX_EPOCH)
        .expect("Time travel is not allowed");

    return time_since.as_millis();
}
