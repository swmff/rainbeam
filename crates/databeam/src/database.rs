//! Database handler
use super::{
    cachedb::CacheDB,
    sql::{create_db, Database, DatabaseOpts},
};

use serde::{Deserialize, Serialize};
use sqlx::{Column, Row};
use std::collections::BTreeMap;

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
pub struct DatabaseReturn(pub BTreeMap<String, String>);

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
    pub fn textify_row(&self, row: sqlx::sqlite::SqliteRow) -> DatabaseReturn {
        // get all columns
        let columns = row.columns();

        // create output
        let mut out: BTreeMap<String, String> = BTreeMap::new();

        for column in columns {
            let name = column.name().to_string();
            let value = row.get(&name.as_str());
            out.insert(name, value);
        }

        // return
        DatabaseReturn(out)
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
        let mut out: BTreeMap<String, String> = BTreeMap::new();

        for column in columns {
            let name = column.name().to_string();
            let value = row.get(&name.as_str());
            out.insert(name, value);
        }

        // return
        DatabaseReturn(out)
    }

    /// Convert all columns into a [`HashMap`].
    ///
    /// # Arguments
    /// * `row`
    /// * `as_bytes` - a [`Vec`] containing all the columns that we want to read in their original `Vec<u8>` form
    #[cfg(feature = "mysql")]
    pub fn textify_row(&self, row: sqlx::mysql::MySqlRow) -> DatabaseReturn {
        // get all columns
        let columns = row.columns();

        // create output
        let mut out: BTreeMap<String, String> = BTreeMap::new();

        for column in columns {
            let name = column.name().to_string();

            match row.try_get::<Vec<u8>, _>(&name.as_str()) {
                Ok(value) => {
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
        DatabaseReturn(out)
    }
}
