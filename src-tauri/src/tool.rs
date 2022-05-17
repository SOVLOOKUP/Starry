use std::path::PathBuf;
use tokio::fs::create_dir_all;

pub async fn ensure_dir_exists(path: &PathBuf) {
    if !path.exists() {
        create_dir_all(&path).await.unwrap();
    }
}