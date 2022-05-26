#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use extension_manager::start_manager;
use tauri::{async_runtime::spawn, Result};
mod extension_manager;
mod tool;

#[tokio::main]
async fn main() -> Result<()> {
    tauri::Builder::default()
        .setup(|app| {
            Ok({
                let app_handle = app.handle();
                spawn(async {
                    start_manager(app_handle).await;
                });
            })
        })
        .run(tauri::generate_context!())
        .expect("初始化失败！");

    Ok(())
}
