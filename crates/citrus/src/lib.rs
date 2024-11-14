//! Citrus client library
use reqwest::Client as HttpClient;
use serde::{de::DeserializeOwned, Serialize};
use std::io::{Result, Error, ErrorKind};

pub mod model;
use model::{HttpProtocol, ServerRepresentation};

/// Core Citrus manager
pub struct CitrusClient {
    http: HttpClient,
    protocol: HttpProtocol,
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
    /// * `address` - the origin of the server (ex: `https://neospring.org`)
    pub async fn server(&self, address: String) -> Result<ServerRepresentation> {
        // send request to address
        let body = match self
            .http
            .get(format!("{address}/.well-known/citrus/citrus.toml"))
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

        Ok(toml::from_str::<T>(&body).unwrap())
    }
}
