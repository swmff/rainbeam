//! Database handler
use super::{
    cachedb::CacheDB,
    sql::{create_db, Database, DatabaseOpts},
};

use serde::{Deserialize, Serialize};
use sqlx::{Column, Row};
use std::collections::HashMap;

/// Default API return value
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DefaultReturn<T> {
    pub success: bool,
    pub message: String,
    pub payload: T,
}

/// Basic return type for database output
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseReturn(pub HashMap<String, String>, pub HashMap<String, Vec<u8>>);

/// Basic database
#[derive(Clone)]
#[cfg(feature = "postgres")]
pub struct StarterDatabase {
    pub db: Database<sqlx::PgPool>,
    pub options: DatabaseOpts,
    pub cachedb: CacheDB,
}

/// Basic database
#[derive(Clone)]
#[cfg(feature = "mysql")]
pub struct StarterDatabase {
    pub db: Database<sqlx::MySqlPool>,
    pub options: DatabaseOpts,
    pub cachedb: CacheDB,
}

/// Basic database
#[derive(Clone)]
#[cfg(feature = "sqlite")]
pub struct StarterDatabase {
    pub db: Database<sqlx::SqlitePool>,
    pub options: DatabaseOpts,
    pub cachedb: CacheDB,
}

impl StarterDatabase {
    pub async fn new(options: DatabaseOpts) -> StarterDatabase {
        StarterDatabase {
            db: create_db(options.clone()).await,
            options,
            cachedb: CacheDB::new().await,
        }
    }

    /// Convert all columns into a [`HashMap`].
    ///
    /// # Arguments
    /// * `row`
    /// * `as_bytes` - a [`Vec`] containing all the columns that we want to read in their original `Vec<u8>` form
    #[cfg(feature = "sqlite")]
    pub fn textify_row(
        &self,
        row: sqlx::sqlite::SqliteRow,
        as_bytes: Vec<String>,
    ) -> DatabaseReturn {
        // get all columns
        let columns = row.columns();

        // create output
        let mut out: HashMap<String, String> = HashMap::new();
        let mut out_bytes: HashMap<String, Vec<u8>> = HashMap::new();

        for column in columns {
            let name = column.name().to_string();

            if as_bytes.contains(&name) {
                let value = row.get(&name.as_str());
                out_bytes.insert(name, value);
                continue;
            }

            let value = row.get(&name.as_str());
            out.insert(name, value);
        }

        // return
        DatabaseReturn(out, out_bytes)
    }

    /// Convert all columns into a [`HashMap`].
    ///
    /// # Arguments
    /// * `row`
    /// * `as_bytes` - a [`Vec`] containing all the columns that we want to read in their original `Vec<u8>` form
    #[cfg(feature = "postgres")]
    pub fn textify_row(&self, row: sqlx::postgres::PgRow) -> DatabaseReturn {
        // get all columns
        let columns = row.columns();

        // create output
        let mut out: HashMap<String, String> = HashMap::new();
        let mut out_bytes: HashMap<String, Vec<u8>> = HashMap::new();

        for column in columns {
            let name = column.name().to_string();

            if as_bytes.contains(&name) {
                let value = row.get(&name.as_str());
                out_bytes.insert(name, value);
                continue;
            }

            let value = row.get(&name.as_str());
            out.insert(name, value);
        }

        // return
        DatabaseReturn(out, out_bytes)
    }

    /// Convert all columns into a [`HashMap`].
    ///
    /// # Arguments
    /// * `row`
    /// * `as_bytes` - a [`Vec`] containing all the columns that we want to read in their original `Vec<u8>` form
    #[cfg(feature = "mysql")]
    pub fn textify_row(&self, row: sqlx::mysql::MySqlRow, as_bytes: Vec<String>) -> DatabaseReturn {
        // get all columns
        let columns = row.columns();

        // create output
        let mut out: HashMap<String, String> = HashMap::new();
        let mut out_bytes: HashMap<String, Vec<u8>> = HashMap::new();

        for column in columns {
            let name = column.name().to_string();

            match row.try_get::<Vec<u8>, _>(&name.as_str()) {
                Ok(value) => {
                    // returned bytes instead of text
                    if as_bytes.contains(&name) {
                        out_bytes.insert(name, value);
                        continue;
                    }

                    // we're going to convert this to a string and then add it to the output!
                    out.insert(
                        column.name().to_string(),
                        std::str::from_utf8(value.as_slice()).unwrap().to_string(),
                    );
                }
                Err(_) => {
                    // already text
                    let value = row.get(&name.as_str());
                    out.insert(name, value);
                }
            };
        }

        // return
        DatabaseReturn(out, out_bytes)
    }
}
