//! Application config manager
use serde::{Deserialize, Serialize};
use std::io::Result;

use authbeam::database::HCaptchaConfig;
use shared::fs;

/// Configuration file
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Config {
    /// The port to serve the server on
    pub port: u16,
    /// The name of the site
    pub name: String,
    /// The description of the site
    pub description: String,
    /// The location of the static directory, should not be supplied manually as it will be overwritten with `./.config/static`
    #[serde(default)]
    pub static_dir: String,
    /// HCaptcha configuration
    pub captcha: HCaptchaConfig,
    /// The name of the header used for reading user IP address
    pub real_ip_header: Option<String>,
    /// If new profile registration is enabled
    #[serde(default)]
    pub registration_enabled: bool,
    /// The origin of the public server (ex: "<https://rainbeam.net>")
    ///
    /// Used in embeds and links.
    #[serde(default)]
    pub host: String,
    /// A list of image hosts that are blocked
    #[serde(default)]
    pub blocked_hosts: Vec<String>,
    /// If a migration should be run
    #[serde(default)]
    pub migration: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 8080,
            name: "Rainbeam".to_string(),
            description: "Ask, share, socialize!".to_string(),
            static_dir: String::new(),
            captcha: HCaptchaConfig::default(),
            real_ip_header: Option::None,
            registration_enabled: true,
            host: String::new(),
            blocked_hosts: Vec::new(),
            migration: false,
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
        let c = fs::canonicalize(".").unwrap();
        let here = c.to_str().unwrap();

        match fs::read(format!("{here}/.config/config.toml")) {
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
            format!("{here}/.config/config.toml"),
            toml::to_string_pretty::<Self>(&contents).unwrap(),
        )
    }
}
