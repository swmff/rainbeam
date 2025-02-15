//! Moka connection manager
use moka::future::Cache as MokaCache_;
use serde::{de::DeserializeOwned, Serialize};

use super::{Cache, TimedObject, EXPIRE_AT};
pub const ENTRIES: u64 = 50_000_u64;

#[derive(Clone)]
pub struct MokaCache {
    pub client: MokaCache_<String, String>,
}

impl Cache for MokaCache {
    type Item = String;
    type Client = MokaCache_<String, String>;

    async fn new() -> Self {
        return Self {
            client: MokaCache_::new(ENTRIES),
        };
    }

    async fn get_con(&self) -> Self::Client {
        self.client.clone()
    }

    async fn get(&self, id: Self::Item) -> Option<String> {
        self.get_con().await.get(&id).await
    }

    async fn set(&self, id: Self::Item, content: Self::Item) -> bool {
        let c = self.get_con().await;
        c.insert(id, content).await;
        true
    }

    async fn update(&self, id: Self::Item, content: Self::Item) -> bool {
        self.set(id, content).await
    }

    async fn remove(&self, id: Self::Item) -> bool {
        let c = self.get_con().await;
        c.invalidate(&id).await;
        true
    }

    async fn remove_starting_with(&self, id: Self::Item) -> bool {
        let c = self.get_con().await;
        match c.invalidate_entries_if(move |k, _| k.starts_with(&id)) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    async fn incr(&self, id: Self::Item) -> bool {
        let c = self.get_con().await;
        let v = match c.get(&id).await {
            Some(v) => v,
            None => return false,
        };

        let num = match v.parse::<i64>() {
            Ok(i) => i,
            Err(_) => return false,
        };

        c.insert(id, (num + 1).to_string()).await;
        true
    }

    async fn decr(&self, id: Self::Item) -> bool {
        let c = self.get_con().await;
        let v = match c.get(&id).await {
            Some(v) => v,
            None => return false,
        };

        let num = match v.parse::<i64>() {
            Ok(i) => i,
            Err(_) => return false,
        };

        c.insert(id, (num - 1).to_string()).await;
        true
    }

    async fn get_timed<T: Serialize + DeserializeOwned>(
        &self,
        id: Self::Item,
    ) -> Option<TimedObject<T>> {
        let c = self.get_con().await;
        match c.get(&id).await {
            Some(d) => match serde_json::from_str::<TimedObject<T>>(&d) {
                Ok(d) => {
                    // check time
                    let now = rainbeam_shared::epoch_timestamp(2024);

                    if now - d.0 >= EXPIRE_AT {
                        // expired key, remove and return None
                        self.remove(id).await;
                        return None;
                    }

                    // return
                    Some(d)
                }
                Err(_) => None,
            },
            None => None,
        }
    }

    async fn set_timed<T: Serialize + DeserializeOwned>(&self, id: Self::Item, content: T) -> bool {
        let c = self.get_con().await;

        c.insert(
            id,
            match serde_json::to_string::<TimedObject<T>>(&(
                rainbeam_shared::epoch_timestamp(2024),
                content,
            )) {
                Ok(s) => s,
                Err(_) => return false,
            },
        )
        .await;

        true
    }
}
