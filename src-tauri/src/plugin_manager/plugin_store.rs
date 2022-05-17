use std::sync::Arc;
use dynamic_reload::{Lib, Symbol};
use tokio::sync::mpsc::UnboundedSender;
use super::{EmitContent, ListenContent};

type PluginType = dyn crate::plugin_manager::plugin_interface::Plugin + Send;

pub struct Plugins(Vec<Box<PluginType>>);

impl Plugins {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn get_service(plugin: &Arc<Lib>) -> Box<PluginType> {
        unsafe {
            plugin
                .lib
                .get::<Symbol<fn() -> Box<PluginType>>>(b"new")
                .unwrap()()
        }
    }

    pub fn load_plugin(
        &mut self,
        plugin: &Arc<Lib>,
        e_sd: &Arc<UnboundedSender<EmitContent>>,
        l_sd: &Arc<UnboundedSender<ListenContent>>,
    ) {
        let service = Self::get_service(&plugin);
        service.load(e_sd, l_sd);
        self.0.push(service);
    }

    pub fn unload_plugin(&mut self, plugin: &Arc<Lib>) {
        let service = Self::get_service(&plugin);
        let id = service.id();
        for i in (0..self.0.len()).rev() {
            let plug = &self.0[i];
            if plug.id() == id {
                plug.unload();
                self.0.swap_remove(i);
            }
        }
    }
}
