//! Shared SQL types
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseOpts {
    /// The type of the database
    pub r#type: Option<String>,
    /// The host to connect to (`None` for SQLite)
    pub host: Option<String>,
    /// The database user's name
    pub user: String,
    /// The database user's password
    pub pass: String,
    /// The name of the database
    #[serde(default = "default_database_name")]
    pub name: String,
}

fn default_database_name() -> String {
    "main".to_string()
}

impl Default for DatabaseOpts {
    fn default() -> Self {
        Self {
            r#type: Some("sqlite".to_string()),
            host: None,
            user: String::new(),
            pass: String::new(),
            name: default_database_name(),
        }
    }
}

// ...
#[derive(Clone)]
pub struct Database<T> {
    pub client: T,
    pub r#type: String,
}

// ...
#[cfg(feature = "mysql")]
/// Create a new "mysql" database
pub async fn create_db(options: DatabaseOpts) -> Database<sqlx::MySqlPool> {
    // mysql
    let opts = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(25)
        .acquire_timeout(std::time::Duration::from_millis(2000))
        .idle_timeout(Some(std::time::Duration::from_secs(60 * 5)));

    let client = opts
        .connect(&format!(
            "mysql://{}:{}@{}/{}",
            options.user,
            options.pass,
            if options.host.is_some() {
                options.host.unwrap()
            } else {
                "localhost".to_string()
            },
            options.name
        ))
        .await;

    if client.is_err() {
        panic!("failed to connect to database: {}", client.err().unwrap());
    }

    return Database {
        client: client.unwrap(),
        r#type: String::from("mysql"),
    };
}

#[cfg(feature = "postgres")]
/// Create a new "postgres" database
pub async fn create_db(options: DatabaseOpts) -> Database<sqlx::PgPool> {
    // postgres
    let opts = sqlx::postgres::PgPoolOptions::new()
        .max_connections(25)
        .acquire_timeout(std::time::Duration::from_millis(2000))
        .idle_timeout(Some(std::time::Duration::from_secs(60 * 5)));

    let client = opts
        .connect(&format!(
            "postgres://{}:{}@{}/{}",
            options.user,
            options.pass,
            if options.host.is_some() {
                options.host.unwrap()
            } else {
                "localhost".to_string()
            },
            options.name
        ))
        .await;

    if client.is_err() {
        panic!("failed to connect to database: {}", client.err().unwrap());
    }

    return Database {
        client: client.unwrap(),
        r#type: String::from("postgres"),
    };
}

#[cfg(feature = "sqlite")]
/// Create a new "sqlite" database (named "main.db")
pub async fn create_db(options: DatabaseOpts) -> Database<sqlx::SqlitePool> {
    // sqlite
    let client = sqlx::SqlitePool::connect(&format!("sqlite://{}.db", options.name)).await;

    if client.is_err() {
        panic!("Failed to connect to database!");
    }

    return Database {
        client: client.unwrap(),
        r#type: String::from("sqlite"),
    };
}
