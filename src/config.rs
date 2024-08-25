//! Application config manager
use serde::{Deserialize, Serialize};
use std::{env, io::Result};

use xsu_authman::database::HCaptchaConfig;
use xsu_util::fs;

/// Configuration file
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Config {
    /// The port to serve the server on
    pub port: u16,
    /// The name of the site
    pub name: String,
    /// The description of the site
    pub description: String,
    /// The location of the static directory, should not be supplied manually as it will be overwritten with `$HOME/.config/xsu-apps/sparkler/static`
    #[serde(default)]
    pub static_dir: String,
    /// HCaptcha configuration
    pub captcha: HCaptchaConfig,
    /// If new profile registration is enabled
    #[serde(default)]
    pub registration_enabled: bool,
    /// The origin of the public server (ex: "<https://sparkler.cc>")
    ///
    /// Used in embeds and links.
    #[serde(default)]
    pub host: String,
    /// If a migration should be run
    #[serde(default)]
    pub migration: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 8080,
            name: "Sparkler".to_string(),
            description: "Simple Q&A".to_string(),
            static_dir: String::new(),
            captcha: HCaptchaConfig::default(),
            registration_enabled: true,
            host: String::new(),
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
        let home = env::var("HOME").expect("failed to read $HOME");

        if let Err(_) = fs::read_dir(format!("{home}/.config/xsu-apps/sparkler")) {
            // make sure .config exists
            fs::mkdir(format!("{home}/.config")).expect("failed to create .config directory");

            // make sure .config/xsu-apps exists
            fs::mkdir(format!("{home}/.config/xsu-apps"))
                .expect("failed to create xsu-apps directory");

            // create .config/xsu-apps/slime
            fs::mkdir(format!("{home}/.config/xsu-apps/sparkler"))
                .expect("failed to create application directory")
        }

        match fs::read(format!("{home}/.config/xsu-apps/sparkler/config.toml")) {
            Ok(c) => Config::read(c),
            Err(_) => {
                Self::update_config(Self::default()).expect("failed to write default config");
                Self::default()
            }
        }
    }

    /// Update configuration file
    pub fn update_config(contents: Self) -> Result<()> {
        let home = env::var("HOME").expect("failed to read $HOME");

        fs::write(
            format!("{home}/.config/xsu-apps/sparkler/config.toml"),
            toml::to_string_pretty::<Self>(&contents).unwrap(),
        )
    }
}
