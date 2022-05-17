use crate::tool::ensure_dir_exists;
use dynamic_reload::{DynamicReload, Lib, PlatformName, Search, Symbol, UpdateState};
pub use plugin_interface::{EmitContent, ListenContent, ListenType};
use std::sync::Arc;
use tauri::api::path::{cache_dir, data_dir};
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::{sleep, Duration};
use self::plugin_store::Plugins;
mod plugin_interface;
mod plugin_store;

pub struct Manager<'a> {
    plugin_store: Plugins,
    handler: DynamicReload,
    e_sd: Arc<UnboundedSender<EmitContent<'a>>>,
    l_sd: Arc<UnboundedSender<ListenContent<'a>>>,
}

impl<'a> Manager<'a> {
    pub async fn new(
        dir_name: &str,
        e_sd: Arc<UnboundedSender<EmitContent<'a>>>,
        l_sd: Arc<UnboundedSender<ListenContent<'a>>>,
    ) -> Manager<'a> {
        let mut plugin_install_path = data_dir().unwrap();
        let mut cache_path = cache_dir().unwrap();
        plugin_install_path.push(dir_name);
        cache_path.push(dir_name);
        ensure_dir_exists(&plugin_install_path).await;
        ensure_dir_exists(&cache_path).await;
        Self {
            plugin_store: Plugins::new(),
            handler: DynamicReload::new(
                Some(vec![plugin_install_path.as_os_str().to_str().unwrap()]),
                Some(cache_path.as_os_str().to_str().unwrap()),
                Search::Default,
                Duration::from_secs(2),
            ),
            e_sd,
            l_sd,
        }
    }

    // 热更新
    pub async fn hot_reload(&mut self, sec: u64) {
        unsafe {
            loop {
                // 拓展更新
                self.handler.update(
                    &|plugin_store: &mut Plugins, state, plugin| match state {
                        UpdateState::Before => plugin_store.unload_plugin(plugin.unwrap()),
                        UpdateState::After => {
                            plugin_store.load_plugin(plugin.unwrap(), &self.e_sd, &self.l_sd)
                        }
                        UpdateState::ReloadFailed(_) => println!("Failed to reload"),
                    },
                    &mut self.plugin_store,
                );

                // todo 查看并与内存比对 index.json 列表 如果有不同则执行load/unload
                sleep(Duration::from_millis(sec)).await;
            }
        };
    }

    fn load_plugin_by_name(&mut self, plugin_name: &str) {
        unsafe {
            match self.handler.add_library(plugin_name, PlatformName::Yes) {
                Ok(lib) => self.plugin_store.load_plugin(&lib, &self.e_sd, &self.l_sd),
                Err(e) => {
                    println!("Unable to load dynamic lib, err {:?}", e);
                    return;
                }
            };
        }
    }

    fn unload_plugin_by_name(&mut self, plugin_name: &str) {
        unsafe {
            match self.handler.add_library(plugin_name, PlatformName::Yes) {
                Ok(lib) => self.plugin_store.unload_plugin(&lib),
                Err(e) => {
                    println!("Unable to load dynamic lib, err {:?}", e);
                    return;
                }
            };
        }
    }
}
