use std::sync::Arc;
use tauri::Event;
use tokio::sync::mpsc::UnboundedSender;

pub struct EmitContent<'a> {
    pub event: &'a str,
    pub payload: &'a str,
}

pub enum ListenType {
    Listen(Box<dyn Fn(Event) + Send + 'static>),
    Unlisten,
}

pub struct ListenContent<'a> {
    pub event: &'a str,
    pub content_type: ListenType,
}

pub trait Plugin {
    // 拓展ID
    fn id(&self) -> &str;
    // 拓展加载
    fn load(
        &self,
        emit_sender: &Arc<UnboundedSender<EmitContent>>,
        listen_sender: &Arc<UnboundedSender<ListenContent>>,
    );
    // 拓展卸载
    fn unload(&self);
}
