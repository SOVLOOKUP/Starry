use std::sync::Arc;
use tauri::Event;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug)]
pub struct EmitContent {
    pub id: String,
    pub event: String,
    pub payload: String,
}

#[allow(dead_code)]
pub enum ListenType {
    Listen(Box<dyn Fn(Event) + Send>),
    Unlisten,
}

pub struct ListenContent {
    pub event: String,
    pub content_type: ListenType,
}

impl std::fmt::Debug for ListenContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListenContent").field("event", &self.event).finish()
    }
}

type EmitSender = UnboundedSender<EmitContent>;
type ListenSender = UnboundedSender<ListenContent>;

pub type ArcEmitSender = Arc<EmitSender>;
pub type ArcListenSender = Arc<ListenSender>;

pub trait Extension: Send {
    // 拓展ID
    fn id(&self) -> &str;
    // 拓展信息 name
    fn info(&self) -> &str;
    // 拓展加载
    fn load(
        &self,
        emit_sender: &ArcEmitSender,
        listen_sender: &ArcListenSender,
    );
    // 拓展卸载
    fn unload(&self);
}
