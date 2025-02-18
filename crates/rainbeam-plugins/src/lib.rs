use serde::{Serialize, Deserialize};
use std::fs::{read_to_string, read_dir};
use pathbufd::PathBufD;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PluginConfig {
    /// The name of the plugin.
    pub name: String,
    /// The name of the plugin author.
    pub author: String,
    /// The version of the plugin.
    pub version: String,
    /// The homepage of the plugin.
    #[serde(default)]
    pub homepage: String,
    /// The path to the `.wasm` file.
    pub wasm: PathBufD,
    /// The ID of a [Neospring](https://neospring.org) module asset to verify the plugin.
    /// Can be left blank if the server has `plugin_verify` set to `false`.
    #[serde(default)]
    pub asset: String,
}

/// Get all plugins in `.config/plugins`.
pub fn get_plugins() -> Vec<PluginConfig> {
    let mut plugins = Vec::new();

    for plugin in read_dir(PathBufD::current().join("plugins")).unwrap() {
        match plugin {
            Ok(p) => {
                let path = p.path();

                if path.extension().unwrap() != "toml" {
                    // can only load toml files
                    continue;
                }

                match read_to_string(path) {
                    Ok(f) => {
                        let config = toml::from_str(&f).unwrap();
                        // TODO: verify plugin here
                        plugins.push(config)
                    }
                    Err(_) => {
                        panic!("Invalid plugin: {}", p.path().to_str().unwrap());
                        continue;
                    }
                }
            }
            Err(_) => {
                panic!("Invalid plugin: {}", p.path().to_str().unwrap());
                continue;
            }
        }
    }

    plugins
}
