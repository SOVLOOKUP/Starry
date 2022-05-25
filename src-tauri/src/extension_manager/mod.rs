mod extension_error;
mod extension_interface;
mod extension_store;

use crate::tool::ensure_dir_exists;
use dynamic_reload::{DynamicReload, PlatformName, Search, UpdateState};
use extension_error::Result;
use extension_interface::{ArcEmitSender, ArcListenSender};
pub use extension_interface::{EmitContent, ListenContent, ListenType};
use extension_store::Extensions;
use serde_json::{self, json, Value};
use sled::{self, Db};
use std::{collections::HashMap, sync::Arc};
use std::{
    path::{Path, PathBuf},
    vec,
};
use tauri::api::path::{cache_dir, data_dir};
use tauri::{async_runtime::spawn, AppHandle, EventHandler, Manager};
use tokio::sync::mpsc::unbounded_channel;
use tokio::{fs::remove_file, time::Duration};

use self::{extension_error::CustomError, extension_store::Context};

#[derive(Debug)]
struct EventContent {
    id: String,
    payload: String,
}

#[derive(Debug)]
enum ManagerEvent {
    InstallExtension(EventContent),
    RemoveExtension(EventContent),
    ListAllExtension(EventContent),
}

fn new_manager_event(install_or_remove: bool, id: String, payload: &str) -> ManagerEvent {
    if install_or_remove {
        ManagerEvent::InstallExtension(EventContent {
            id,
            payload: payload.to_string(),
        })
    } else {
        ManagerEvent::RemoveExtension(EventContent {
            id,
            payload: payload.to_string(),
        })
    }
}

pub struct ExtensionManager {
    extension_store: Extensions,
    handler: DynamicReload,
    db: Db,
    ext_install_path: PathBuf,
    e_sd: ArcEmitSender,
}

impl ExtensionManager {
    pub async fn new(
        dir_name: &str,
        e_sd: ArcEmitSender,
        l_sd: ArcListenSender,
    ) -> ExtensionManager {
        let mut app_data_path = data_dir().unwrap();
        let mut cache_path = cache_dir().unwrap();
        app_data_path.push(dir_name);
        cache_path.push(dir_name);

        // 拓展安装目录
        let mut ext_install_path = app_data_path.clone();
        ext_install_path.push("extension");

        // 拓展缓存目录
        let mut ext_cache_path = cache_path.clone();
        ext_cache_path.push("extension");

        println!("ext_install_path: {:?}", ext_install_path);

        // 内置数据库持久化目录
        let mut db_persist_path = app_data_path.clone();
        db_persist_path.push("db_data");

        // 确保所有目录存在
        ensure_dir_exists(&ext_install_path).await.unwrap();
        ensure_dir_exists(&ext_cache_path).await.unwrap();
        ensure_dir_exists(&db_persist_path).await.unwrap();

        // 内置数据库
        let db = sled::open(&db_persist_path).unwrap();

        let mut manager = Self {
            extension_store: Extensions::new(e_sd.clone(), l_sd),
            handler: DynamicReload::new(
                Some(vec![ext_install_path.as_os_str().to_str().unwrap()]),
                Some(ext_cache_path.as_os_str().to_str().unwrap()),
                Search::Default,
                Duration::from_secs(2),
            ),
            db,
            ext_install_path,
            e_sd,
        };

        // 获取安装的拓展并启动
        for r in manager.db.iter() {
            if let Ok((_, info)) = r {
                let info: Value =
                    serde_json::from_str(String::from_utf8(info.to_vec()).unwrap().as_str())
                        .unwrap();
                if let Some(name) = info["__file_name"].as_str() {
                    // 从 info 中解析拓展文件名称并启动
                    manager
                        .load_extension_by_file_name(name, &Context::default())
                        .unwrap();
                };
            }
        }

        manager
    }

    // 热更新
    pub async fn try_hot_reload(&mut self) {
        unsafe {
            // 更新拓展
            self.handler.update(
                &|extension_store: &mut Extensions, state, ext_lib| match state {
                    UpdateState::Before => {
                        extension_store
                            .unload_extension(ext_lib.unwrap(), &Context::default())
                            .expect("failed to unload extension");
                        ()
                    }
                    UpdateState::After => {
                        extension_store
                            .load_extension(ext_lib.unwrap(), &Context::default())
                            .expect("failed to load extension");
                        ()
                    }
                    UpdateState::ReloadFailed(_) => println!("Failed to reload"),
                },
                &mut self.extension_store,
            );
        };
    }

    fn load_extension_by_file_name(
        &mut self,
        extension_name: &str,
        context: &Context,
    ) -> extension_error::Result<(String, String)> {
        unsafe {
            match self.handler.add_library(extension_name, PlatformName::No) {
                Ok(lib) => self.extension_store.load_extension(&lib, context),
                Err(e) => Err(extension_error::CustomError::LibLoadError(e)),
            }
        }
    }

    // 从文件安装新插件
    pub async fn install_extension(
        &mut self,
        extension_path: &str,
        context: Context,
    ) -> Result<()> {
        // 复制拓展库到目标文件夹
        tokio::fs::copy(
            extension_path,
            self.ext_install_path.as_os_str().to_str().unwrap(),
        )
        .await?;
        let extension_path = Path::new(extension_path);

        if !extension_path.is_file() {
            self.e_sd.send(EmitContent {
                id: context.id,
                event: "error".to_string(),
                payload: "该路径不是一个文件".to_string(),
            })?;
        } else {
            let file_name = extension_path.file_name().unwrap();
            // 启动插件
            let (id, info) =
                self.load_extension_by_file_name(file_name.to_str().unwrap(), &context)?;
            // 向 info 中添加文件名称字段
            let mut v: Value = serde_json::from_str(info.as_str())?;
            v["__file_name"] = json!(file_name);
            // 将插件信息附加到内置数据库
            self.db.insert(id, v.to_string().as_str()).unwrap();
        };
        Ok(())
    }

    // 移除插件
    pub async fn remove_extension(&mut self, id: &str, context: Context) -> Result<()> {
        // 关闭插件
        self.extension_store.unload_extension_by_id(id, &context)?;
        // 从内置数据库删除插件信息
        if let Ok(Some(info)) = self.db.remove(id) {
            if let Ok(info) = String::from_utf8(info.to_vec()) {
                let info: Value = serde_json::from_str(info.as_str()).unwrap();
                // 从 info 中解析拓展文件名称从文件夹删除插件
                remove_file(info["__file_name"].to_string()).await?;
            }
            Ok(())
        } else {
            Err(CustomError::ExtRemoveError(format!(
                "没有安装 id 为 {} 的拓展",
                id
            )))
        }
    }

    pub async fn list_all_extension(&mut self, context: Context) -> Result<()> {
        for r in self.db.iter() {
            if let Ok((id, info)) = r {
                let info = String::from_utf8(info.to_vec()).unwrap();
                let id = String::from_utf8(id.to_vec()).unwrap();
                let payload = serde_json::to_string(&serde_json::json!({ id: info })).unwrap();
                self.e_sd.send(EmitContent {
                    id: context.id.clone(),
                    event: "loaded_extension".to_string(),
                    payload,
                })?;
            }
        }
        Ok(())
    }
}

pub async fn start_manager(app: AppHandle) {
    let app_name = app.package_info().name.clone();
    let (e_sd, mut e_rc) = unbounded_channel::<EmitContent>();
    let (l_sd, mut l_rc) = unbounded_channel::<ListenContent>();
    let (m_sd, mut m_rc) = unbounded_channel::<ManagerEvent>();
    let m_sd2 = m_sd.clone();
    let m_sd3 = m_sd.clone();

    let mut event_handler_map: HashMap<String, EventHandler> = HashMap::new();
    let e_sender = Arc::new(e_sd);
    let l_sender = Arc::new(l_sd);

    // 拓展管理器
    // 透传 emit & listen 发送端
    let mut manager = ExtensionManager::new(app_name.as_str(), e_sender, l_sender).await;

    // 监听安装卸载事件
    app.listen_global("install_extension", move |event| {
        if let Some(path) = event.payload() {
            m_sd.send(new_manager_event(true, event.id().to_string(), path))
                .unwrap();
        };
    });

    app.listen_global("remove_extension", move |event| {
        if let Some(id) = event.payload() {
            m_sd2
                .send(new_manager_event(false, event.id().to_string(), id))
                .unwrap();
        };
    });

    app.listen_global("list_all_extension", move |event| {
        if let Some(id) = event.payload() {
            m_sd3
                .send(ManagerEvent::ListAllExtension(EventContent {
                    id: id.to_string(),
                    payload: event.payload().unwrap().to_string(),
                }))
                .unwrap();
        };
    });

    let app_handle1 = app.clone();
    let app_handle2 = app.clone();

    let ext_event_emiter = spawn(async move {
        while let Some(e_content) = e_rc.recv().await {
            app_handle1
                .emit_all(&e_content.event, e_content.payload)
                .expect(format!("[失败] 发送 {} 事件", e_content.event).as_str());
            println!("[成功] 发送 {} 事件", e_content.event)
        }
    });

    let ext_event_listener = spawn(async move {
        while let Some(l_content) = l_rc.recv().await {
            match l_content.content_type {
                ListenType::Listen(handler) => {
                    let k = l_content.event.clone();
                    let v = app_handle2.listen_global(l_content.event, handler);
                    println!("[成功] 监听 {} 事件", k);
                    event_handler_map.insert(k, v);
                }
                ListenType::Unlisten => {
                    if let Some(handler) = event_handler_map.remove(l_content.event.as_str()) {
                        app_handle2.unlisten(handler);
                        println!("[成功] 删除 {} 事件", l_content.event)
                    }
                }
            }
        }
    });

    let manager_event_excuter = spawn(async move {
        while let Some(m_content) = m_rc.recv().await {
            match m_content {
                ManagerEvent::InstallExtension(content) => {
                    let path = content.payload;
                    match manager
                        .install_extension(
                            &path,
                            Context {
                                id: content.id.clone(),
                            },
                        )
                        .await
                    {
                        Ok(_) => println!("[成功] 安装 {} 拓展", path),
                        Err(e) => {
                            println!("[失败] 安装 {} 拓展", path);
                            manager
                                .e_sd
                                .send(EmitContent {
                                    id: content.id,
                                    event: "error".to_string(),
                                    payload: e.to_string(),
                                })
                                .unwrap();
                        }
                    }
                }
                ManagerEvent::RemoveExtension(content) => {
                    let id = content.payload;
                    match manager
                        .remove_extension(
                            &id,
                            Context {
                                id: content.id.clone(),
                            },
                        )
                        .await
                    {
                        Ok(_) => println!("[成功] 移除 {} 拓展", id),
                        Err(e) => {
                            println!("[失败] 移除 {} 拓展", id);
                            manager
                                .e_sd
                                .send(EmitContent {
                                    id: content.id,
                                    event: "error".to_string(),
                                    payload: e.to_string(),
                                })
                                .unwrap();
                        }
                    }
                }
                ManagerEvent::ListAllExtension(content) => {
                    manager
                        .list_all_extension(Context {
                            id: content.id.clone(),
                        })
                        .await
                        .unwrap();
                }
            }
        }
    });

    manager_event_excuter
        .await
        .expect("无法正常监听安装移除拓展事件！");
    ext_event_listener
        .await
        .expect("无法正常挂载拓展事件监听器！");
    ext_event_emiter
        .await
        .expect("无法正常挂载拓展事件广播器！");
}
