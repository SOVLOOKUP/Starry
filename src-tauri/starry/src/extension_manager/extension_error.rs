use starry_extension_interface::EmitContent;
use std::fmt::{Display, Formatter};
use std::result::Result as StdResult;
use tokio::sync::mpsc::error::SendError;

pub type Result<T> = StdResult<T, CustomError<EmitContent>>;

#[derive(Debug)]
pub enum CustomError<T> {
    EmitSendError(SendError<T>),
    LibLoadError(dynamic_reload::Error),
    TauriError(tauri::Error),
    ExtRemoveError(String),
}

impl<T> From<SendError<T>> for CustomError<T> {
    fn from(s: tokio::sync::mpsc::error::SendError<T>) -> Self {
        CustomError::EmitSendError(s)
    }
}

impl<T> From<dynamic_reload::Error> for CustomError<T> {
    fn from(s: dynamic_reload::Error) -> Self {
        CustomError::LibLoadError(s)
    }
}

impl<T> From<tauri::Error> for CustomError<T> {
    fn from(s: tauri::Error) -> Self {
        CustomError::TauriError(s)
    }
}

impl<T> From<serde_json::Error> for CustomError<T> {
    fn from(s: serde_json::Error) -> Self {
        CustomError::TauriError(tauri::Error::Json(s))
    }
}

impl<T> From<std::io::Error> for CustomError<T> {
    fn from(s: std::io::Error) -> Self {
        CustomError::TauriError(tauri::Error::Io(s))
    }
}

impl<T> From<String> for CustomError<T> {
    fn from(s: String) -> Self {
        CustomError::ExtRemoveError(s)
    }
}

impl<T> Display for CustomError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            CustomError::EmitSendError(ref e) => e.fmt(f),
            CustomError::LibLoadError(ref e) => e.fmt(f),
            CustomError::TauriError(ref e) => e.fmt(f),
            CustomError::ExtRemoveError(ref e) => e.fmt(f),
        }
    }
}
