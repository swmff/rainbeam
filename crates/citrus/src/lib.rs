//! Citrus client library
use reqwest::Client as HttpClient;
use serde::{de::DeserializeOwned, Serialize};
use std::io::{Result, Error, ErrorKind};

pub mod model;
use model::{HttpProtocol, ServerRepresentation};

/// Citrus API body template builder
pub struct TemplateBuilder(pub String);

impl TemplateBuilder {
    /// Build template
    pub fn build(mut self, values: Vec<&str>) -> Self {
        for value in values {
            self.0 = self.0.replacen("<field>", value, 1);
        }

        self
    }
}

/// Core Citrus manager
#[derive(Clone)]
pub struct CitrusClient {
    pub http: HttpClient,
    pub protocol: HttpProtocol,
}

impl CitrusClient {
    /// Create a new [`CitrusClient`]
    pub fn new(protocol: HttpProtocol) -> Self {
        Self {
            http: HttpClient::new(),
            protocol,
        }
    }

    /// Get the [`ServerRepresentation`] of a server
    ///
    /// # Arguments
    /// * `hostname` - the hostname of the server (ex: `https://neospring.org`)
    pub async fn server(&self, hostname: String) -> Result<ServerRepresentation> {
        // send request to address
        let body = match self
            .http
            .get(format!(
                "{}//{hostname}/.well-known/citrus/citrus.toml",
                self.protocol.to_string()
            ))
            .send()
            .await
        {
            Ok(b) => match b.text().await {
                Ok(t) => t,
                Err(_) => return Err(Error::new(ErrorKind::InvalidData, "Body is invalid")),
            },
            Err(_) => return Err(Error::new(ErrorKind::Other, "Failed to request")),
        };

        // return
        Ok(toml::from_str::<ServerRepresentation>(&body).unwrap())
    }

    /// Get data(`T`) from the given server which must support `schema`
    ///
    /// # Arguments
    /// * `server` - [`ServerRepresentation`]
    /// * `schema` - the ID of the schema the remote server must support
    /// * `url` - the URL to fetch the data from (starting with forward slash)
    pub async fn get<T: Serialize + DeserializeOwned>(
        &self,
        server: ServerRepresentation,
        schema: &str,
        url: &str,
    ) -> Result<T> {
        if !server.has_schema(schema) {
            return Err(Error::new(
                ErrorKind::Other,
                "Server does not support schema",
            ));
        }

        let address = format!("{}//{}", self.protocol.to_string(), server.server.id);
        let body = match self.http.get(format!("{address}{url}")).send().await {
            Ok(b) => match b.text().await {
                Ok(t) => t,
                Err(_) => return Err(Error::new(ErrorKind::InvalidData, "Body is invalid")),
            },
            Err(_) => return Err(Error::new(ErrorKind::Other, "Failed to request")),
        };

        // we're going to assume most things return json
        Ok(serde_json::from_str::<T>(&body).unwrap())
    }
}
