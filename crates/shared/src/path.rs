use std::path::{Path, PathBuf};
use std::env::current_dir as std_current_dir;
use std::io::Result;

/// [`PathBuf`] wrapper
pub struct PathBufD(pub PathBuf);

impl PathBufD {
    pub fn push<P>(&mut self, path: P) -> ()
    where
        P: AsRef<Path>,
    {
        self.0.push(path)
    }

    pub fn join<P>(self, path: P) -> PathBufD
    where
        P: AsRef<Path>,
    {
        Self(self.0.join(path))
    }
}

impl ToString for PathBufD {
    fn to_string(&self) -> String {
        return self.0.to_str().unwrap_or("").to_string();
    }
}

impl AsRef<Path> for PathBufD {
    fn as_ref(&self) -> &Path {
        self.0.as_path()
    }
}

/// Get the current directory from env
pub fn current_dir() -> Result<PathBufD> {
    Ok(PathBufD(match std_current_dir() {
        Ok(p) => p,
        Err(e) => return Err(e),
    }))
}
