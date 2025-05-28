#[macro_export]
macro_rules! cache_sync {
    (|$row:ident, $id:ident| $key:ident->($update:ident in $self:ident){1}) => {{
        let as_str = stringify!($key);

        let row_count = match $row.get(as_str) {
            Some(s) => s,
            None => "0",
        }
        .parse::<usize>()
        .unwrap_or(0);

        let count = $self
            .base
            .cache
            .get(format!("rbeam.app.{}:{}", as_str, &$id))
            .await
            .unwrap_or_default()
            .parse::<usize>()
            .unwrap_or(0);

        if count == 0 {
            row_count
        } else {
            // ensure values sync (update the lesser value)
            if row_count > count {
                $self
                    .base
                    .cache
                    .set(
                        format!("rbeam.app.{}:{}", as_str, &$id),
                        row_count.to_string(),
                    )
                    .await;
            } else {
                $self.$update(&$id, count).await.unwrap();
            };

            // ...
            count
        }
    }};
}

#[macro_export]
macro_rules! from_row {
    ($row:ident->$name:ident(ref)) => {
        $row.get(stringify!($name)).unwrap()
    };

    ($row:ident->$name:ident()) => {
        $row.get(stringify!($name)).unwrap().to_string()
    };

    ($row:ident->$name:ident(ref)) => {
        $row.get(stringify!($name)).unwrap()
    };

    ($row:ident->$name:ident()) => {
        $row.get(stringify!($name)).unwrap().to_string()
    };

    ($row:ident->$name:ident(ref); $v:literal) => {
        $row.get(stringify!($name)).unwrap_or($v)
    };

    ($row:ident->$name:ident(); $v:literal) => {
        $row.get(stringify!($name)).unwrap_or($v).to_string()
    };

    ($row:ident->$name:ident(ref); $v:expr) => {
        $row.get(stringify!($name)).unwrap_or($v)
    };

    ($row:ident->$name:ident(); $v:expr) => {
        $row.get(stringify!($name)).unwrap_or($v).to_string()
    };

    ($row:ident->$name:ident($t:tt); $v:literal) => {
        $row.get(stringify!($name))
            .unwrap()
            .parse::<$t>()
            .unwrap_or($v)
    };

    ($row:ident->$name:ident($t:expr); $v:literal) => {
        $row.get(stringify!($name))
            .unwrap()
            .parse::<$t>()
            .unwrap_or($v)
    };

    ($row:ident->$name:ident(json); $e:expr) => {
        match serde_json::from_str($row.get(stringify!($name)).unwrap()) {
            Ok(de) => de,
            Err(_) => return Err($e),
        }
    };

    ($row:ident->$name:ident(json); $e:tt) => {
        match serde_json::from_str($row.get(stringify!($name)).unwrap()) {
            Ok(de) => de,
            Err(_) => return Err($e),
        }
    };

    ($row:ident->$name:ident(toml); $e:expr) => {
        match toml::from_str($row.get(stringify!($name)).unwrap()) {
            Ok(de) => de,
            Err(_) => return Err($e),
        }
    };

    ($row:ident->$name:ident(toml); $e:tt) => {
        match toml::from_str($row.get(stringify!($name)).unwrap()) {
            Ok(de) => de,
            Err(_) => return Err($e),
        }
    };
}

#[macro_export]
macro_rules! update_profile_count {
    ($name:tt, $col:ident) => {
        /// Update a profile count value.
        ///
        /// # Arguments
        /// * `id`
        /// * `count`
        pub async fn $name(&self, id: &str, count: usize) -> Result<()> {
            // update profile
            let col_name = stringify!($col);
            let query: String =
                if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                    format!("UPDATE \"xprofiles\" SET \"{col_name}\" = ? WHERE \"id\" = ?")
                } else {
                    format!("UPDATE \"xprofiles\" SET \"{col_name}\" = ? WHERE \"id\" = ?")
                };

            let c = &self.base.db.client;
            match sqlquery(&query)
                .bind::<&i64>(&(count as i64))
                .bind::<&str>(&id)
                .execute(c)
                .await
            {
                Ok(_) => {
                    let username = self.get_profile_username(id).await;

                    self.base
                        .cache
                        .remove(format!("rbeam.auth.profile:{id}"))
                        .await;

                    self.base
                        .cache
                        .get(format!("rbeam.auth.profile:{}", username))
                        .await;

                    Ok(())
                }
                Err(_) => Err(DatabaseError::Other),
            }
        }
    };
}

/// Simplfy an error-forwarding match block.
#[macro_export]
macro_rules! simplify {
    ($e:expr; Result) => {
        match $e {
            Ok(x) => x,
            Err(e) => return Err(e),
        }
    };

    ($e:expr; Err) => {
        if let Err(e) = $e {
            return Err(e);
        }
    };

    ($e:expr; Option) => {
        match $e {
            Some(x) => x,
            None => return None,
        }
    };

    ($e:expr; None) => {
        if let None = $e {
            return None;
        }
    };

    ($e:expr; Result; $v:expr) => {
        match $e {
            Ok(x) => x,
            Err(e) => $v,
        }
    };

    ($e:expr; Err; $v:expr) => {
        if let Err(_) = $e {
            return $v;
        }
    };

    ($e:expr; Option; $v:expr) => {
        match $e {
            Some(x) => x,
            None => return $v,
        }
    };

    ($e:expr; None; $v:expr) => {
        if let None = $e {
            return $v;
        }
    };
}

/// Ignore (`let _ = ...`) something.
#[macro_export]
macro_rules! ignore {
    ($e:expr) => {
        let _ = $e;
    };
}

pub fn serde_json_to_string<T: serde::Serialize>(value: T) -> Result<String, serde_json::Error> {
    serde_json::to_string(&value)
}
