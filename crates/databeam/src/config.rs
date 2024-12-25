//! Application config manager
use serde::{Deserialize, Serialize};
use rainbeam_shared::{fs, path::current_dir};
use std::io::Result;

/// Configuration file
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Config {
    pub connection: crate::sql::DatabaseOpts,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            connection: crate::sql::DatabaseOpts::default(),
        }
    }
}

impl Config {
    /// Read configuration file into [`Config`]
    pub fn read(contents: String) -> Self {
        toml::from_str::<Self>(&contents).unwrap()
    }

    /// Pull configuration file
    pub fn get_config() -> Self {
        let path = current_dir()
            .unwrap()
            .join(".config")
            .join("databeam")
            .join("config.toml");

        match fs::read(path) {
            Ok(c) => Config::read(c),
            Err(_) => {
                Self::update_config(Self::default()).expect("failed to write default config");
                Self::default()
            }
        }
    }

    /// Update configuration file
    pub fn update_config(contents: Self) -> Result<()> {
        let c = fs::canonicalize(".").unwrap();
        let here = c.to_str().unwrap();

        fs::write(
            format!("{here}/.config/databeam/config.toml"),
            toml::to_string_pretty::<Self>(&contents).unwrap(),
        )
    }
}
