#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use extension_manager::{EmitContent, ListenContent, ListenType};
use std::{collections::HashMap, sync::Arc};
use tauri::{async_runtime::spawn, EventHandler, Manager, Result};
use tokio::sync::mpsc::unbounded_channel;
mod extension_manager;
mod tool;

#[tokio::main]
async fn main() -> Result<()> {
    const DEV: bool = false;
    let (e_sd, mut e_rc) = unbounded_channel::<EmitContent>();
    let (l_sd, mut l_rc) = unbounded_channel::<ListenContent>();
    let mut event_handler_map: HashMap<&str, EventHandler> = HashMap::new();

    let app = tauri::Builder::default()
        .setup(|app| {
            let app_name = app.package_info().name.clone();
            
            spawn(async move {
                let e_sender = Arc::new(e_sd);
                let l_sender = Arc::new(l_sd);
                // 拓展管理器
                // 透传 emit & listen 发送端
                let mut manager = extension_manager::Manager::new(
                    app_name.as_str(),
                    e_sender,
                    l_sender,
                ).await;

                // todo 监听安装卸载事件
                // l_sd.send(ListenContent {
                //     event: "install_extension",
                //     content_type: ListenType::Listen(Box::new(|x| {
                //         manager.install_extension(x.payload().unwrap());
                //     })),
                // })
                // .unwrap_err();
                // l_sd.send(ListenContent {
                //     event: "remove_extension",
                //     content_type: ListenType::Listen(Box::new(|_x| {})),
                // })
                // .unwrap_err();
               
                // 开发模式启用模块热更新
                if DEV {manager.hot_reload(2).await;}
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("初始化失败！");

    let app_handle1 = app.handle();
    let app_handle2 = app.handle();

    spawn(async move {
        while let Some(e_content) = e_rc.recv().await {
            app_handle1
                .emit_all(e_content.event, e_content.payload)
                .expect(format!("[失败] 发送 {} 事件", e_content.event).as_str());
            println!("[成功] 发送 {} 事件", e_content.event)
        }
    });

    spawn(async move {
        while let Some(l_content) = l_rc.recv().await {
            match l_content.content_type {
                ListenType::Listen(handler) => {
                    event_handler_map.insert(
                        l_content.event,
                        app_handle2.listen_global(l_content.event, handler),
                    );
                    println!("[成功] 监听 {} 事件", l_content.event)
                }
                ListenType::Unlisten => {
                    if let Some(handler) = event_handler_map.remove(l_content.event) {
                        app_handle2.unlisten(handler);
                        println!("[成功] 删除 {} 事件", l_content.event)
                    }
                }
            }
        }
    });

    let _ = &app.run(move |_app_handle, event| match event {
        tauri::RunEvent::ExitRequested { api, .. } => {
            api.prevent_exit();
            _app_handle.exit(0);
        }
        _ => {}
    });

    Ok(())
}
