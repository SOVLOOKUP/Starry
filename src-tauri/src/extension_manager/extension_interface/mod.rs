use std::sync::Arc;
use tauri::Event;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug)]
pub struct EmitContent<'a> {
    pub event: &'a str,
    pub payload: String,
}

pub enum ListenType {
    Listen(Box<dyn Fn(Event) + Send>),
    Unlisten,
}

pub struct ListenContent<'a> {
    pub event: &'a str,
    pub content_type: ListenType,
}

pub type EmitSender<'a> = UnboundedSender<EmitContent<'a>>;
pub type ListenSender<'a> = UnboundedSender<ListenContent<'a>>;

pub type ArcEmitSender<'a> = Arc<EmitSender<'a>>;
pub type ArcListenSender<'a> = Arc<ListenSender<'a>>;

pub trait Extension: Send {
    // 拓展ID
    fn id(&self) -> &str;
    // 拓展信息 name
    fn info(&self) -> &str;
    // 拓展加载
    fn load(
        &self,
        emit_sender: &EmitSender,
        listen_sender: &ListenSender,
    );
    // 拓展卸载
    fn unload(&self);
}
