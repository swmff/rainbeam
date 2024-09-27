//! Fs access utilities
//!
//! This is essentially a wrapper around standard fs, so it's just to keep things similar.
use std::path::Path;
pub use std::{
    fs::{
        create_dir, read_dir, read_to_string, remove_dir_all, remove_file, write as std_write,
        read as std_read, canonicalize, metadata, Metadata,
    },
    io::Result,
};

/// Get a path's metadata
///
/// # Arguments
/// * `path`
pub fn fstat<P>(path: P) -> Result<Metadata>
where
    P: AsRef<Path>,
{
    metadata(path)
}

/// Create a directory if it does not already exist
///
/// # Arguments
/// * `path`
pub fn mkdir<P>(path: P) -> Result<()>
where
    P: AsRef<Path>,
{
    if let Err(_) = read_dir(&path) {
        create_dir(path)?
    }

    Ok(())
}

/// `rm -r` for only directories
///
/// # Arguments
/// * `path`
pub fn rmdirr<P: AsRef<Path>>(path: P) -> Result<()>
where
    P: AsRef<Path>,
{
    if let Err(_) = read_dir(&path) {
        return Ok(()); // doesn't exist, return ok since there was nothing to remove
    }

    remove_dir_all(path)
}

/// `rm` for only files
///
/// # Arguments
/// * `path`
pub fn rm<P: AsRef<Path>>(path: P) -> Result<()>
where
    P: AsRef<Path>,
{
    remove_file(path)
}

/// Write to a file given its path and data
///
/// # Arguments
/// * `path`
/// * `data`
pub fn write<P, D>(path: P, data: D) -> Result<()>
where
    P: AsRef<Path>,
    D: AsRef<[u8]>,
{
    std_write(path, data)
}

/// `touch`
///
/// # Arguments
/// * `path`
pub fn touch<P>(path: P) -> Result<()>
where
    P: AsRef<Path>,
{
    std_write(path, "")
}

/// Append to a file given its path and data
///
/// # Arguments
/// * `path`
/// * `data`
pub fn append<P, D>(path: P, data: D) -> Result<()>
where
    P: AsRef<Path>,
    D: AsRef<[u8]>,
{
    let mut bytes = std_read(&path)?; // read current data as bytes
    bytes = [&bytes, data.as_ref()].concat(); // append
    std_write(path, bytes) // write
}

/// `cat`
///
/// # Arguments
/// * `path`
///
/// # Returns
/// * [`String`]
pub fn read<P: AsRef<Path>>(path: P) -> Result<String>
where
    P: AsRef<Path>,
{
    read_to_string(path)
}
