use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use rainbeam_shared::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LangFile {
    pub name: String,
    pub version: String,
    pub data: HashMap<String, String>,
}

impl Default for LangFile {
    fn default() -> Self {
        Self {
            name: "net.rainbeam.langs.testing:aa-BB".to_string(),
            version: "0.0.0".to_string(),
            data: HashMap::new(),
        }
    }
}

impl LangFile {
    /// Check if a value exists in `data` (and isn't empty)
    pub fn exists(&self, key: &str) -> bool {
        if let Some(ref value) = self.data.get(key) {
            if value.is_empty() {
                return false;
            }

            return true;
        }

        false
    }

    /// Get a value from `data`, returns an empty string if it doesn't exist
    pub fn get(&self, key: &str) -> String {
        if !self.exists(key) {
            return key.to_string();
        }

        self.data.get(key).unwrap().to_owned()
    }
}

/// Read the `langs` directory and return a [`Hashmap`] containing all files
pub fn read_langs() -> HashMap<String, LangFile> {
    let mut out = HashMap::new();

    // read directory
    let c = fs::canonicalize(".").unwrap();
    let here = c.to_str().unwrap();

    if let Ok(files) = fs::read_dir(format!("{here}/langs")) {
        for file in files.into_iter() {
            if file.is_err() {
                continue;
            }

            let de: LangFile =
                match serde_json::from_str(&match fs::read_to_string(file.unwrap().path()) {
                    Ok(f) => f,
                    Err(_) => continue,
                }) {
                    Ok(de) => de,
                    Err(_) => continue,
                };

            out.insert(de.name.clone(), de);
        }
    }

    // return
    out
}
