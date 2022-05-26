use starry_extension_interface::{Extension,EmitSender, ListenSender};

#[no_mangle]
pub fn new() -> Box<dyn Extension> {
    Box::new(PluginSayHello { id: "3a90790e".to_string()})
}

pub struct PluginSayHello {
    id: String,
}

impl Extension for PluginSayHello {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn info(&self) -> &str {
        todo!()
    }

    fn load(
        &self,
        emit_sender: &EmitSender,
        listen_sender: &ListenSender,
    ) {
        todo!()
    }

    fn unload(&self) {
        println!("[{}] Unload instance!", self.id);
    }    
}
