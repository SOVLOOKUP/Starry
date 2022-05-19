mod extension_interface;
mod extension_store;

use crate::tool::ensure_dir_exists;
use dynamic_reload::{DynamicReload, PlatformName, Search, UpdateState};
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
use tauri::{async_runtime::spawn, AppHandle, EventHandler, Manager, Result};
use tokio::sync::mpsc::unbounded_channel;
use tokio::{
    fs::remove_file,
    time::{sleep, Duration},
};

#[derive(Debug)]
enum ManagerEvent {
    InstallExtension(String),
    RemoveExtension(String),
}

pub struct ExtensionManager<'a> {
    extension_store: Extensions<'a>,
    handler: DynamicReload,
    db: Db,
    ext_install_path: PathBuf,
}

impl<'a> ExtensionManager<'a> {
    pub async fn new(
        dir_name: &str,
        e_sd: ArcEmitSender<'a>,
        l_sd: ArcListenSender<'a>,
    ) -> ExtensionManager<'a> {
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
            extension_store: Extensions::new(e_sd, l_sd),
            handler: DynamicReload::new(
                Some(vec![ext_install_path.as_os_str().to_str().unwrap()]),
                Some(ext_cache_path.as_os_str().to_str().unwrap()),
                Search::Default,
                Duration::from_secs(2),
            ),
            db,
            ext_install_path,
        };

        // 获取安装的拓展并启动
        for r in manager.db.iter() {
            if let Ok((_, info)) = r {
                let info: Value =
                    serde_json::from_str(String::from_utf8(info.to_vec()).unwrap().as_str())
                        .unwrap();
                if let Some(name) = info["__file_name"].as_str() {
                    // 从 info 中解析拓展文件名称并启动
                    manager.load_extension_by_file_name(name);
                };
            }
        }

        manager
    }

    // 热更新
    pub async fn hot_reload(&mut self, sec: u64) {
        unsafe {
            loop {
                // 更新拓展
                self.handler.update(
                    &|extension_store: &mut Extensions, state, ext_lib| match state {
                        UpdateState::Before => {
                            extension_store.unload_extension(ext_lib.unwrap());
                            ()
                        }
                        UpdateState::After => {
                            extension_store.load_extension(ext_lib.unwrap());
                            ()
                        }
                        UpdateState::ReloadFailed(_) => println!("Failed to reload"),
                    },
                    &mut self.extension_store,
                );
                // 2秒运行间隔
                sleep(Duration::from_secs(sec)).await;
            }
        };
    }

    fn load_extension_by_file_name(&mut self, extension_name: &str) -> Option<(String, String)> {
        unsafe {
            match self.handler.add_library(extension_name, PlatformName::No) {
                Ok(lib) => Some(self.extension_store.load_extension(&lib)),
                Err(e) => {
                    println!("Unable to load dynamic lib, err {:?}", e);
                    return None;
                }
            }
        }
    }

    // 从文件安装新插件
    pub async fn install_extension(&mut self, extension_path: &str) -> Result<()> {
        // 复制拓展库到目标文件夹
        tokio::fs::copy(
            extension_path,
            self.ext_install_path.as_os_str().to_str().unwrap(),
        )
        .await?;
        if let Some(file_name) = Path::new(extension_path).file_name() {
            // 启动插件
            if let Some((id, info)) = self.load_extension_by_file_name(file_name.to_str().unwrap())
            {
                // 向 info 中添加文件名称字段
                let mut v: Value = serde_json::from_str(info.as_str())?;
                v["__file_name"] = json!(file_name);
                // 将插件信息附加到内置数据库
                self.db.insert(id, v.to_string().as_str()).unwrap();
            };
        };
        Ok(())
    }

    // 移除插件
    pub async fn remove_extension(&mut self, id: &str) -> Result<()> {
        // 关闭插件
        self.extension_store.unload_extension_by_id(id);
        // 从内置数据库删除插件信息
        if let Ok(Some(info)) = self.db.remove(id) {
            if let Ok(info) = String::from_utf8(info.to_vec()) {
                let info: Value = serde_json::from_str(info.as_str()).unwrap();
                // 从 info 中解析拓展文件名称从文件夹删除插件
                remove_file(info["__file_name"].to_string()).await?;
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

    let mut event_handler_map: HashMap<&str, EventHandler> = HashMap::new();
    let e_sender = Arc::new(e_sd);
    let l_sender = Arc::new(l_sd);

    // 拓展管理器
    // 透传 emit & listen 发送端
    let mut manager = ExtensionManager::new(app_name.as_str(), e_sender, l_sender).await;

    // 监听安装卸载事件
    app.listen_global("install_extension", move |event| {
        if let Some(path) = event.payload() {
            m_sd.send(ManagerEvent::InstallExtension(path.to_string()))
                .expect("安装拓展失败！");
        };
    });

    app.listen_global("remove_extension", move |event| {
        if let Some(id) = event.payload() {
            m_sd2
                .send(ManagerEvent::RemoveExtension(id.to_string()))
                .expect("移除拓展失败！");
        };
    });

    let app_handle1 = app.clone();
    let app_handle2 = app.clone();

    let ext_event_emiter = spawn(async move {
        while let Some(e_content) = e_rc.recv().await {
            app_handle1
                .emit_all(e_content.event, e_content.payload)
                .expect(format!("[失败] 发送 {} 事件", e_content.event).as_str());
            println!("[成功] 发送 {} 事件", e_content.event)
        }
    });

    let ext_event_listener = spawn(async move {
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

    let manager_event_excuter = spawn(async move {
        while let Some(m_content) = m_rc.recv().await {
            match m_content {
                ManagerEvent::InstallExtension(path) => {
                    manager.install_extension(&path).await.unwrap();
                    println!("[成功] 安装 {} 拓展", path)
                }
                ManagerEvent::RemoveExtension(id) => {
                    manager.remove_extension(&id).await.unwrap();
                    println!("[成功] 移除 {} 拓展", id)
                }
            }
        }
    });

    // // 开发模式启用模块热更新
    // spawn(async move {
    //     if true {manager.hot_reload(2).await;}
    // });

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
