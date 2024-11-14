use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HttpProtocol {
    Http,
    Https,
}

impl ToString for HttpProtocol {
    fn to_string(&self) -> String {
        match self {
            Self::Http => "http:".to_string(),
            Self::Https => "https:".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HttpMethod {
    GET,
    HEAD,
    POST,
    PUT,
    OPTIONS,
}

/// A simple identifier to identify resources from other servers
///
/// ```
/// {server_id}:{uuid/hash}
/// ```
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CitrusID(String);

impl CitrusID {
    /// Get the number of bytes that make up both parts of a CitrusID
    pub fn bytes_lens(&self) -> (usize, usize) {
        let mut server_id: usize = 0;
        let mut hash: usize = 0;

        let mut hit_hash: bool = false;

        for char in self.0.chars() {
            if char == ':' {
                hit_hash = true;
                continue;
            }

            if !hit_hash {
                server_id += 1;
                continue;
            }

            hash += 1;
        }

        if hit_hash == false {
            // done going through chars, but we never hit hash
            // this means there is no server_id, so everything should go to the hash
            hash = server_id.clone();
            server_id = 0;
        }

        (server_id, hash)
    }

    /// Get the `server_id` section of the ID
    pub fn server_id(&self) -> String {
        let (len, _) = self.bytes_lens();
        self.0.chars().take(len).collect()
    }

    /// Get the `hash` section of the ID
    pub fn hash(&self) -> String {
        let (server_id, len) = self.bytes_lens();
        self.0.chars().skip(server_id + 1).take(len).collect()
    }
}

/// A representation of a server (`/.well-known/citrus/citrus.toml`)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerRepresentation {
    pub server: ServerInfo,
    pub schemas: Vec<SchemaPointer>,
}

impl ServerRepresentation {
    /// Get the location of a schema file based on its ID
    pub fn get_schema(&self, schema: &str) -> Option<String> {
        if let Some(schema) = self.schemas.iter().find(|s| s.id == schema) {
            return Some(schema.location.to_owned());
        }

        None
    }

    /// Check if the server has the given schema by its ID
    pub fn has_schema(&self, schema: &str) -> bool {
        self.get_schema(schema).is_some()
    }
}

/// Information about a server
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerInfo {
    /// Public server name
    pub name: String,
    /// Server hostname
    pub id: String,
}

/// A description of where a schema is located based on its ID
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SchemaPointer {
    /// Schema ID
    pub id: String,
    /// Schema location (relative to `/.well-known/citrus`)
    pub location: String,
    /// The schema's API endpoints
    #[serde(default)]
    pub api: HashMap<String, SchemaAPI>,
}

/// A description of a schema's API
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SchemaAPI {
    /// The method to send with the API request
    pub method: HttpMethod,
    /// The URL to send the API request to
    pub url: String,
    /// The body of the API request
    ///
    /// Supports template values as defined in the specification.
    pub body: String,
}

/// A schema
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Schema {
    /// Schema ID
    pub id: String,
    /// Schema data
    pub r#struct: HashMap<String, String>,
}
