//! Almost Snowflake
//!
//! Random IDs which include timestamp information (like Twitter Snowflakes)
//!
//! IDs are generated with 41 bits of an epoch timestamp, 10 bits of a machine/server ID, and 12 bits of randomly generated numbers.
//!
//! ```
//! tttttttttttttttttttttttttttttttttttttttttiiiiiiiiiirrrrrrrrrrrr...
//! Timestamp                                ID        Seed
//! ```
use serde::{Serialize, Deserialize};
use crate::epoch_timestamp;

use num_bigint::BigInt;
use rand::Rng;

static SEED_LEN: usize = 12;
// static ID_LEN: usize = 10;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AlmostSnowflake(String);

pub fn bigint(input: usize) -> BigInt {
    BigInt::from(input)
}

impl AlmostSnowflake {
    /// Create a new [`AlmostSnowflake`]
    pub fn new(server_id: usize) -> Self {
        // generate random bytes
        let mut bytes = String::new();

        let mut rng = rand::rng();
        for _ in 1..=SEED_LEN {
            bytes.push_str(&rng.random_range(0..10).to_string())
        }

        // build id
        let mut id = bigint(epoch_timestamp(2024) as usize) << 22 as u128;
        id = id | bigint((server_id % 1024) << 12);
        id = id | bigint((bytes.parse::<usize>().unwrap() + 1) % 4096);

        // return
        Self(id.to_string())
    }
}

impl std::fmt::Display for AlmostSnowflake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
