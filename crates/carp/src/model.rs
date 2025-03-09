pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub enum Error {
    DeserializeError(String),
    Custom(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{:?}", self))
    }
}

/// A simple carpgraph renderer.
pub trait CarpGraph {
    /// Serialize an image to bytes.
    fn to_bytes(&self) -> Vec<u8>;
    /// Deserialize an image from bytes.
    fn from_bytes(bytes: Vec<u8>) -> Self;
    /// Convert an image to svg format.
    fn to_svg(&self) -> String;
}
