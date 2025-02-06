//! Hashing and IDs
use hex_fmt::HexFmt;
use rand::{distr::Alphanumeric, rng, Rng};
use sha2::{Digest, Sha256};
use uuid::Uuid;

// ids
pub fn uuid() -> String {
    let uuid = Uuid::new_v4();
    return uuid.to_string();
}

pub fn hash(input: String) -> String {
    let mut hasher = <Sha256 as Digest>::new();
    hasher.update(input.into_bytes());

    let res = hasher.finalize();
    return HexFmt(res).to_string();
}

pub fn hash_salted(input: String, salt: String) -> String {
    let mut hasher = <Sha256 as Digest>::new();
    hasher.update(format!("{salt}{input}").into_bytes());

    let res = hasher.finalize();
    return HexFmt(res).to_string();
}

pub fn salt() -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}

pub fn random_id() -> String {
    return hash(uuid());
}
