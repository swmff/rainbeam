//! Almost Snowflake
//!
//! Random IDs which include timestamp information (like Twitter Snowflakes)
//!
//! IDs are generated with 6 bytes of randomly generated numbers and then a unix epoch timestamp.
use serde::{Serialize, Deserialize};
use crate::unix_epoch_timestamp;
use rand::Rng;

static ID_LEN: usize = 6;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AlmostSnowflake(String);

impl AlmostSnowflake {
    /// Create a new [`AlmostSnowflake`]
    pub fn new() -> Self {
        // generate random bytes
        let mut bytes = String::new();

        let mut rng = rand::thread_rng();
        for _ in 1..=6 {
            bytes.push_str(&rng.gen_range(0..10).to_string())
        }

        // return
        Self(format!("{bytes}{}", unix_epoch_timestamp()))
    }

    /// Get both parts of the ID
    pub fn parts(&self) -> (String, u128) {
        (
            self.0.chars().take(ID_LEN).collect(),
            self.0
                .chars()
                .skip(ID_LEN)
                .take(self.0.len() - ID_LEN)
                .collect::<String>()
                .parse()
                .unwrap(),
        )
    }
}
