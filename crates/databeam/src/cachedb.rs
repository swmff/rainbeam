//! Redis connection manager
use redis::{Commands, ToRedisArgs};
use serde::{de::DeserializeOwned, Serialize};

#[allow(type_alias_bounds)]
pub type TimedObject<T: Serialize + DeserializeOwned> = (i64, T);

pub const EXPIRE_AT: i64 = 3_600_000;

#[derive(Clone)]
pub struct CacheDB {
    pub client: redis::Client,
}

impl CacheDB {
    pub async fn new() -> CacheDB {
        return CacheDB {
            client: redis::Client::open("redis://127.0.0.1:6379").unwrap(),
        };
    }

    pub async fn get_con(&self) -> redis::Connection {
        self.client.get_connection().unwrap()
    }

    /// Get a cache object by its identifier
    ///
    /// # Arguments:
    /// * `id` - `String` of the object's id
    pub async fn get<I>(&self, id: I) -> Option<String>
    where
        I: ToRedisArgs,
    {
        match self.get_con().await.get(id) {
            Ok(d) => Some(d),
            Err(_) => None,
        }
    }

    /// Set a cache object by its identifier and content
    ///
    /// # Arguments:
    /// * `id` - `String` of the object's id
    /// * `content` - `String` of the object's content
    pub async fn set<I>(&self, id: String, content: I) -> bool
    where
        I: ToRedisArgs,
    {
        let mut c = self.get_con().await;
        let res: Result<String, redis::RedisError> = c.set(id, content);

        match res {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Update a cache object by its identifier and content
    ///
    /// # Arguments:
    /// * `id` - `String` of the object's id
    /// * `content` - `String` of the object's content
    pub async fn update<I>(&self, id: String, content: I) -> bool
    where
        I: ToRedisArgs,
    {
        self.set(id, content).await
    }

    /// Remove a cache object by its identifier
    ///
    /// # Arguments:
    /// * `id` - `String` of the object's id
    pub async fn remove<I>(&self, id: I) -> bool
    where
        I: ToRedisArgs,
    {
        let mut c = self.get_con().await;
        let res: Result<String, redis::RedisError> = c.del(id);

        match res {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Remove a cache object by its identifier('s start)
    ///
    /// # Arguments:
    /// * `id` - `String` of the object's id('s start)
    pub async fn remove_starting_with<I>(&self, id: I) -> bool
    where
        I: ToRedisArgs,
    {
        let mut c = self.get_con().await;

        // get keys
        let mut cmd = redis::cmd("DEL");
        let keys: Result<Vec<String>, redis::RedisError> = c.keys(id);

        for key in keys.unwrap() {
            cmd.arg(key);
        }

        // remove
        let res: Result<String, redis::RedisError> = cmd.query(&mut c);

        match res {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Increment a cache object by its identifier
    ///
    /// # Arguments:
    /// * `id` - `String` of the object's id
    pub async fn incr<I>(&self, id: I) -> bool
    where
        I: ToRedisArgs,
    {
        let mut c = self.get_con().await;
        let res: Result<String, redis::RedisError> = c.incr(id, 1);

        match res {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Decrement a cache object by its identifier
    ///
    /// # Arguments:
    /// * `id` - `String` of the object's id
    pub async fn decr<I>(&self, id: I) -> bool
    where
        I: ToRedisArgs,
    {
        let mut c = self.get_con().await;
        let res: Result<String, redis::RedisError> = c.decr(id, 1);

        match res {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Get a cache object by its identifier
    ///
    /// # Arguments:
    /// * `id` - `String` of the object's id
    pub async fn get_timed<T, I>(&self, id: I) -> Option<TimedObject<T>>
    where
        T: Serialize + DeserializeOwned,
        I: ToRedisArgs,
    {
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

    /// Set a cache object by its identifier and content
    ///
    /// # Arguments:
    /// * `id` - `String` of the object's id
    /// * `content` - `String` of the object's content
    pub async fn set_timed<T, I>(&self, id: I, content: T) -> bool
    where
        T: Serialize + DeserializeOwned,
        I: ToRedisArgs,
    {
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

        match res {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}
