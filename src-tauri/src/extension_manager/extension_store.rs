use super::{extension_interface::{ArcListenSender, Extension, ArcEmitSender}, EmitContent};
use dynamic_reload::{Lib, Symbol};
use std::{sync::Arc, collections::HashMap};

pub struct Extensions<'a> {
    running: HashMap<String, Box<dyn Extension + Send>>,
    e_sd: ArcEmitSender<'a>,
    l_sd: ArcListenSender<'a>,
}

impl<'a> Extensions<'a> {
    pub fn new(e_sd: ArcEmitSender<'a>, l_sd: ArcListenSender<'a>) -> Self {
        Self {
            running: HashMap::new(),
            e_sd,
            l_sd,
        }
    }

    pub fn get_service(ext_lib: &Arc<Lib>) -> Box<dyn Extension + Send> {
        unsafe {
            ext_lib
                .lib
                .get::<Symbol<fn() -> Box<dyn Extension + Send>>>(b"new")
                .unwrap()()
        }
    }

    pub fn load_extension(&mut self, ext_lib: &Arc<Lib>) -> (String, String) {
        let service = Self::get_service(&ext_lib);
        let id = String::from(service.id());
        let info = String::from(service.info());
        let id_copy = id.clone();
        service.load(&self.e_sd, &self.l_sd);
        // 推送插件加载信息
        self.e_sd.send(EmitContent {
            event: "installed_extension",
            payload: format!(
                "{{\"{}\":{}}}",
                id,
                info
            ),
        })
        .unwrap();
        // 将插件加载到内存记录
        self.running.insert(id, service);
        (id_copy, info)
    }

    pub fn unload_extension_by_id(&mut self, ext_id: &str) {
        if let Some(service) = self.running.remove(ext_id) {
            service.unload();
            // 推送插件移除信息
            self.e_sd.send(EmitContent {
                event: "remove_extension",
                payload: ext_id.to_string(),
            })
            .unwrap();
        };
    }   
    
    pub fn unload_extension(&mut self, ext_lib: &Arc<Lib>) -> String {
        let service = Self::get_service(&ext_lib);
        let id = service.id();
        self.unload_extension_by_id(id);
        let id = String::from(id);
        id
    }    
}
