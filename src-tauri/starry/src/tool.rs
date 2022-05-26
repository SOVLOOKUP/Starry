use std::path::PathBuf;
use tokio::fs::create_dir_all;
use std::io;

pub async fn ensure_dir_exists(path: &PathBuf) -> io::Result<()> {
    if !path.exists() {
        create_dir_all(&path).await?;
    }
    Ok(())
}