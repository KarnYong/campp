use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    Process(String),
    Config(String),
    Download(String),
    Database(String),
    Settings(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "IO error: {}", e),
            AppError::Process(msg) => write!(f, "Process error: {}", msg),
            AppError::Config(msg) => write!(f, "Config error: {}", msg),
            AppError::Download(msg) => write!(f, "Download error: {}", msg),
            AppError::Database(msg) => write!(f, "Database error: {}", msg),
            AppError::Settings(msg) => write!(f, "Settings error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}

impl From<AppError> for tauri::ipc::InvokeError {
    fn from(e: AppError) -> Self {
        tauri::ipc::InvokeError::from(e.to_string())
    }
}

impl From<AppError> for serde_json::Value {
    fn from(e: AppError) -> Self {
        serde_json::json!({ "error": e.to_string() })
    }
}
