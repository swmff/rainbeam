//! Oysters connection manager
use serde::{de::DeserializeOwned, Serialize};
use oysters_client::Client as OystersClient;

use super::{Cache, TimedObject, EXPIRE_AT};

#[derive(Clone)]
pub struct OystersCache {
    pub client: OystersClient,
}

impl Cache for OystersCache {
    type Item = String;
    type Client = OystersClient;

    async fn new() -> Self {
        Self {
            client: OystersClient::new("http://localhost:5072".to_string()),
        }
    }

    async fn get_con(&self) -> Self::Client {
        OystersClient::new("http://localhost:5072".to_string())
    }

    async fn get(&self, id: Self::Item) -> Option<String> {
        let v = self.client.get(&id).await;

        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    }

    async fn set(&self, id: Self::Item, content: Self::Item) -> bool {
        self.client.insert(&id, &content).await;
        true
    }

    async fn update(&self, id: Self::Item, content: Self::Item) -> bool {
        self.set(id, content).await
    }

    async fn remove(&self, id: Self::Item) -> bool {
        self.client.remove(&id).await;
        true
    }

    async fn remove_starting_with(&self, id: Self::Item) -> bool {
        let keys: Vec<String> = self.client.filter_keys(&id).await;

        for key in keys {
            self.remove(key).await;
        }

        true
    }

    async fn incr(&self, id: Self::Item) -> bool {
        self.client.incr(&id).await;
        true
    }

    async fn decr(&self, id: Self::Item) -> bool {
        self.client.decr(&id).await;
        true
    }

    async fn get_timed<T: Serialize + DeserializeOwned>(
        &self,
        id: Self::Item,
    ) -> Option<TimedObject<T>> {
        let res: String = self.client.get(&id).await;
        match serde_json::from_str::<TimedObject<T>>(&res) {
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
        }
    }

    async fn set_timed<T: Serialize + DeserializeOwned>(&self, id: Self::Item, content: T) -> bool {
        self.client
            .insert(
                &id,
                &match serde_json::to_string::<TimedObject<T>>(&(
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
