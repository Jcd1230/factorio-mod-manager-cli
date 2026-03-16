use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Message(String),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("TOML decode error: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("TOML encode error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
    #[error("Dialog error: {0}")]
    Dialog(#[from] dialoguer::Error),
}

impl AppError {
    pub fn message<S: Into<String>>(message: S) -> Self {
        Self::Message(message.into())
    }
}
