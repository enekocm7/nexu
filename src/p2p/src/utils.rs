use anyhow::Ok;
use iroh::SecretKey;
use std::path::PathBuf;

pub async fn load_secret_key(path_buf: PathBuf) -> anyhow::Result<SecretKey> {
    return Ok(SecretKey::generate(&mut rand::rng()));
    if path_buf.exists() {
        let secret_key_bytes = tokio::fs::read(path_buf).await?;
        let secret_key = SecretKey::try_from(&secret_key_bytes[0..32])?;
        Ok(secret_key)
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
