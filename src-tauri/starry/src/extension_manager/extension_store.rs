use super::extension_error::{CustomError, Result};
use starry_extension_interface::{ArcEmitSender, ArcListenSender, Extension, EmitContent};

use dynamic_reload::{Lib, Symbol};
use std::{collections::HashMap, sync::Arc};

pub struct Context {
    pub id: String,
}

impl Context {
    pub fn default() -> Self {
        Self { id: "".to_string() }
    }
}

pub struct Extensions {
    running: HashMap<String, Box<dyn Extension + Send>>,
    e_sd: ArcEmitSender,
    l_sd: ArcListenSender,
}

impl Extensions {
    pub fn new(e_sd: ArcEmitSender, l_sd: ArcListenSender) -> Self {
        Self {
            running: HashMap::new(),
            e_sd,
            l_sd,
        }
    }

    pub fn get_service(&mut self, ext_lib: &Arc<Lib>) -> Result<Box<dyn Extension + Send>> {
        unsafe {
            match ext_lib
                .lib
                .get::<Symbol<fn() -> Box<dyn Extension + Send>>>(b"new")
            {
                Ok(new) => Ok(new()),
                Err(e) => Err(CustomError::LibLoadError(dynamic_reload::Error::Load(e))),
            }
        }
    }

    pub fn load_extension(
        &mut self,
        ext_lib: &Arc<Lib>,
        context: &Context,
    ) -> Result<(String, String)> {
        let service = self.get_service(&ext_lib)?;
        let id = String::from(service.id());
        let info = String::from(service.info());
        service.load(&self.e_sd, &self.l_sd);
        let payload = serde_json::to_string(&serde_json::json!({ id.clone(): info })).unwrap();
        // 推送插件加载信息
        self.e_sd.send(EmitContent {
            id: context.id.clone(),
            event: "loaded_extension".to_string(),
            payload,
        })?;
        // 将插件加载到内存记录
        self.running.insert(id.clone(), service);
        Ok((id, info))
    }

    pub fn unload_extension_by_id(&mut self, ext_id: &str, context: &Context) -> Result<()> {
        if let Some(service) = self.running.remove(ext_id) {
            service.unload();
            // 推送插件移除信息
            self.e_sd.send(EmitContent {
                id: context.id.clone(),
                event: "unloaded_extension".to_string(),
                payload: ext_id.to_string(),
            })?;
        };
        Ok(())
    }

    pub fn unload_extension(&mut self, ext_lib: &Arc<Lib>, context: &Context) -> Result<String> {
        let service = self.get_service(&ext_lib)?;
        let id = service.id();
        self.unload_extension_by_id(id, context)?;
        let id = String::from(id);
        Ok(id)
    }
}
