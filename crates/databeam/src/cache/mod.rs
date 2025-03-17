#![allow(async_fn_in_trait)]
use serde::{de::DeserializeOwned, Serialize};

pub const EXPIRE_AT: i64 = 3_600_000;

#[allow(type_alias_bounds)]
pub type TimedObject<T: Serialize + DeserializeOwned> = (i64, T);

#[cfg(feature = "redis")]
pub mod redis;

#[cfg(feature = "moka")]
pub mod moka;

#[cfg(feature = "oysters")]
pub mod oysters;

/// A simple cache "database".
pub trait Cache {
    type Item;
    type Client;

    /// Create a new [`Cache`].
    async fn new() -> Self;
    /// Get a connection to the cache.
    async fn get_con(&self) -> Self::Client;

    /// Get a cache object by its identifier
    ///
    /// # Arguments
    /// * `id` - `String` of the object's id
    async fn get(&self, id: Self::Item) -> Option<String>;
    /// Set a cache object by its identifier and content
    ///
    /// # Arguments
    /// * `id` - `String` of the object's id
    /// * `content` - `String` of the object's content
    async fn set(&self, id: Self::Item, content: Self::Item) -> bool;
    /// Update a cache object by its identifier and content
    ///
    /// # Arguments
    /// * `id` - `String` of the object's id
    /// * `content` - `String` of the object's content
    async fn update(&self, id: Self::Item, content: Self::Item) -> bool;
    /// Remove a cache object by its identifier
    ///
    /// # Arguments
    /// * `id` - `String` of the object's id
    async fn remove(&self, id: Self::Item) -> bool;
    /// Remove a cache object by its identifier('s start)
    ///
    /// # Arguments
    /// * `id` - `String` of the object's id('s start)
    async fn remove_starting_with(&self, id: Self::Item) -> bool;
    /// Increment a cache object by its identifier
    ///
    /// # Arguments
    /// * `id` - `String` of the object's id
    async fn incr(&self, id: Self::Item) -> bool;
    /// Decrement a cache object by its identifier
    ///
    /// # Arguments
    /// * `id` - `String` of the object's id
    async fn decr(&self, id: Self::Item) -> bool;

    /// Get a cache object by its identifier
    ///
    /// # Arguments
    /// * `id` - `String` of the object's id
    async fn get_timed<T: Serialize + DeserializeOwned>(
        &self,
        id: Self::Item,
    ) -> Option<TimedObject<T>>;
    /// Set a cache object by its identifier and content
    ///
    /// # Arguments
    /// * `id` - `String` of the object's id
    /// * `content` - `String` of the object's content
    async fn set_timed<T: Serialize + DeserializeOwned>(&self, id: Self::Item, content: T) -> bool;
}
