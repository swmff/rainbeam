pub use crate::cache::Cache;
pub use crate::database::{DefaultReturn, StarterDatabase};

#[cfg(feature = "redis")]
pub use crate::cache::redis::RedisCache;

#[cfg(feature = "moka")]
pub use crate::cache::moka::MokaCache;

#[cfg(feature = "oysters")]
pub use crate::cache::oysters::OystersCache;
