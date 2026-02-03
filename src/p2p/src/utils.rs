//! # Utility Functions
//!
//! Helper functions for the P2P crate, primarily focused on key management.

use iroh::SecretKey;
use std::path::PathBuf;

/// Loads a secret key from a file, or generates a new one if it doesn't exist.
///
/// If the file at `path_buf` exists, it attempts to read 32 bytes and convert them into a `SecretKey`.
/// If the file does not exist, it generates a new random `SecretKey`, creates the necessary parent directories,
/// and saves the key to the file for future use.
///
/// # Arguments
///
/// * `path_buf` - The path to the file where the secret key is stored or should be stored.
///
/// # Returns
///
/// * `Result<SecretKey>` - The loaded or generated secret key.
///
/// # Errors
///
/// This function will return an error if it failed to read/write/access the path or if it fails to parse the key in the existing file
pub async fn load_secret_key(path_buf: PathBuf) -> anyhow::Result<SecretKey> {
    if path_buf.exists() {
        let secret_key_bytes = tokio::fs::read(&path_buf).await?;
        match SecretKey::try_from(&secret_key_bytes[0..32]) {
            Ok(secret_key) => Ok(secret_key),
            Err(_) => generate_key(path_buf).await,
        }
    } else {
        let secret_key = SecretKey::generate(&mut rand::rng());
        let secret_key_bytes = secret_key.to_bytes();
        if let Some(parent) = path_buf.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path_buf, &secret_key_bytes).await?;
        Ok(secret_key)
    }
}

async fn generate_key(path_buf: PathBuf) -> anyhow::Result<SecretKey> {
    let secret_key = SecretKey::generate(&mut rand::rng());
    let secret_key_bytes = secret_key.to_bytes();
    if let Some(parent) = path_buf.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(path_buf, &secret_key_bytes).await?;
    Ok(secret_key)
}
