use std::path::{Path, PathBuf};
use std::env::current_dir as std_current_dir;
use std::io::Result;
use std::fmt::Display;

use serde::{Deserialize, Serialize};

/// [`PathBuf`] wrapper
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PathBufD(pub PathBuf);

impl PathBufD {
    pub fn push<P>(&mut self, path: P) -> ()
    where
        P: AsRef<Path>,
    {
        self.0.push(path)
    }

    pub fn join<P>(self, path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self(self.0.join(path))
    }
}

impl Default for PathBufD {
    fn default() -> Self {
        Self(PathBuf::default())
    }
}

impl Display for PathBufD {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_str().unwrap_or(""))
    }
}

impl AsRef<Path> for PathBufD {
    fn as_ref(&self) -> &Path {
        self.0.as_path()
    }
}

impl Into<PathBufD> for PathBuf {
    fn into(self) -> PathBufD {
        PathBufD(self)
    }
}

impl From<PathBufD> for PathBuf {
    fn from(value: PathBufD) -> Self {
        value.0
    }
}

/// Get the current directory from env
pub fn current_dir() -> Result<PathBufD> {
    Ok(PathBufD(match std_current_dir() {
        Ok(p) => p,
        Err(e) => return Err(e),
    }))
}
