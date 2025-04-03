//! Redis connection manager
use redis::Commands;
use serde::{de::DeserializeOwned, Serialize};

use super::{Cache, TimedObject, EXPIRE_AT};

#[derive(Clone)]
pub struct RedisCache {
    pub client: redis::Client,
}

impl Cache for RedisCache {
    type Item = String;
    type Client = redis::Connection;

    async fn new() -> Self {
        Self {
            client: redis::Client::open("redis://127.0.0.1:6379").unwrap(),
        }
    }

    async fn get_con(&self) -> Self::Client {
        self.client.get_connection().unwrap()
    }

    async fn get(&self, id: Self::Item) -> Option<String> {
        self.get_con().await.get(id).ok()
    }

    async fn set(&self, id: Self::Item, content: Self::Item) -> bool {
        let mut c = self.get_con().await;
        let res: Result<String, redis::RedisError> = c.set_ex(id, content, 604800);

        res.is_ok()
    }

    async fn update(&self, id: Self::Item, content: Self::Item) -> bool {
        self.set(id, content).await
    }

    async fn remove(&self, id: Self::Item) -> bool {
        let mut c = self.get_con().await;
        let res: Result<String, redis::RedisError> = c.del(id);

        res.is_ok()
    }

    async fn remove_starting_with(&self, id: Self::Item) -> bool {
        let mut c = self.get_con().await;

        // get keys
        let mut cmd = redis::cmd("DEL");
        let keys: Result<Vec<String>, redis::RedisError> = c.keys(id);

        for key in keys.unwrap() {
            cmd.arg(key);
        }

        // remove
        let res: Result<String, redis::RedisError> = cmd.query(&mut c);

        res.is_ok()
    }

    async fn incr(&self, id: Self::Item) -> bool {
        let mut c = self.get_con().await;
        let res: Result<String, redis::RedisError> = c.incr(id, 1);

        res.is_ok()
    }

    async fn decr(&self, id: Self::Item) -> bool {
        let mut c = self.get_con().await;
        let res: Result<String, redis::RedisError> = c.decr(id, 1);

        res.is_ok()
    }

    async fn get_timed<T: Serialize + DeserializeOwned>(
        &self,
        id: Self::Item,
    ) -> Option<TimedObject<T>> {
        let mut c = self.get_con().await;
        let res: Result<String, redis::RedisError> = c.get(&id);

        match res {
            Ok(d) => match serde_json::from_str::<TimedObject<T>>(&d) {
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
            Err(_) => None,
        }
    }

    async fn set_timed<T: Serialize + DeserializeOwned>(&self, id: Self::Item, content: T) -> bool {
        let mut c = self.get_con().await;
        let res: Result<String, redis::RedisError> = c.set(
            id,
            match serde_json::to_string::<TimedObject<T>>(&(
                rainbeam_shared::epoch_timestamp(2024),
                content,
            )) {
                Ok(s) => s,
                Err(_) => return false,
            },
        );

        res.is_ok()
    }
}
