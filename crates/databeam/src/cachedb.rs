//! Redis connection manager
use redis::Commands;

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

    // GET
    /// Get a cache object by its identifier
    ///
    /// # Arguments:
    /// * `id` - `String` of the object's id
    pub async fn get(&self, id: String) -> Option<String> {
        // fetch from database
        let mut c = self.get_con().await;
        let res = c.get(id);

        if res.is_err() {
            return Option::None;
        }

        // return
        Option::Some(res.unwrap())
    }

    // SET
    /// Set a cache object by its identifier and content
    ///
    /// # Arguments:
    /// * `id` - `String` of the object's id
    /// * `content` - `String` of the object's content
    pub async fn set(&self, id: String, content: String) -> bool {
        // set
        let mut c = self.get_con().await;
        let res: Result<String, redis::RedisError> = c.set(id, content);

        if res.is_err() {
            return false;
        }

        // return
        true
    }

    /// Update a cache object by its identifier and content
    ///
    /// # Arguments:
    /// * `id` - `String` of the object's id
    /// * `content` - `String` of the object's content
    pub async fn update(&self, id: String, content: String) -> bool {
        self.set(id, content).await
    }

    /// Remove a cache object by its identifier
    ///
    /// # Arguments:
    /// * `id` - `String` of the object's id
    pub async fn remove(&self, id: String) -> bool {
        // remove
        let mut c = self.get_con().await;
        let res: Result<String, redis::RedisError> = c.del(id);

        if res.is_err() {
            return false;
        }

        // return
        true
    }

    /// Remove a cache object by its identifier('s start)
    ///
    /// # Arguments:
    /// * `id` - `String` of the object's id('s start)
    pub async fn remove_starting_with(&self, id: String) -> bool {
        let mut c = self.get_con().await;

        // get keys
        let mut cmd = redis::cmd("DEL");
        let keys: Result<Vec<String>, redis::RedisError> = c.keys(id);

        for key in keys.unwrap() {
            cmd.arg(key);
        }

        // remove
        let res: Result<String, redis::RedisError> = cmd.query(&mut c);

        if res.is_err() {
            return false;
        }

        // return
        true
    }

    /// Increment a cache object by its identifier
    ///
    /// # Arguments:
    /// * `id` - `String` of the object's id
    pub async fn incr(&self, id: String) -> bool {
        // remove
        let mut c = self.get_con().await;
        let res: Result<String, redis::RedisError> = c.incr(id, 1);

        if res.is_err() {
            return false;
        }

        // return
        true
    }

    /// Decrement a cache object by its identifier
    ///
    /// # Arguments:
    /// * `id` - `String` of the object's id
    pub async fn decr(&self, id: String) -> bool {
        // remove
        let mut c = self.get_con().await;
        let res: Result<String, redis::RedisError> = c.decr(id, 1);

        if res.is_err() {
            return false;
        }

        // return
        true
    }
}
