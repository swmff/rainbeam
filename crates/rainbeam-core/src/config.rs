//! Application config manager
use serde::{Deserialize, Serialize};
use std::io::Result;

use authbeam::database::HCaptchaConfig;
use shared::fs;

/// Premium features
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Tiers {
    /// Doubled character limits for everything
    ///
    /// * Questions: ~~2048~~ **4096**
    /// * Responses: ~~4096~~ **8192**
    /// * CommentS: ~~2048~~ **4096**
    ///
    /// *\*Carpgraph drawings stay at 32kb maximum*
    #[serde(default)]
    pub double_limits: i32,
    /// Styled profile card in the followers/following/friends section of other users
    #[serde(default)]
    pub stylish_card: i32,
    /// A small little crown shown on the user's profile avatar
    #[serde(default)]
    pub avatar_crown: i32,
    /// A small badge shwon on the user's profile
    #[serde(default)]
    pub profile_badge: i32,
    /// Pages access (super long blog-like posts)
    #[serde(default)]
    pub pages: i32,
}

impl Default for Tiers {
    /// Everything is tier 1 by default
    fn default() -> Self {
        Self {
            double_limits: 1,
            stylish_card: 1,
            avatar_crown: 1,
            profile_badge: 1,
            pages: 1,
        }
    }
}

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
    /// The origin of the public server (ex: "https://rainbeam.net")
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
    /// Tiered benefits
    #[serde(default)]
    pub tiers: Tiers,
    /// A global site announcement shown at the top of the page
    #[serde(default)]
    pub alert: String,
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
            tiers: Tiers::default(),
            alert: String::new(),
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
