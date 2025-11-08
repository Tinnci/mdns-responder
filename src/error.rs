use std::io;
use thiserror::Error;
use windows::core::Error as WinError;

#[derive(Error, Debug)]
pub enum MdnsError {
    #[error("Configuration validation error: {0}")]
    ConfigValidation(String),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("mDNS service error: {0}")]
    Service(String),

    #[error("Windows error: {0}")]
    Windows(#[from] WinError),

    #[error("Thread error: {0}")]
    Thread(String),

    #[error("Service error: {0}")]
    ServiceDispatcher(String),

    #[error("Network adapter error: {0}")]
    IpConfig(String),
}

pub type Result<T> = std::result::Result<T, MdnsError>;

impl From<windows_service::Error> for MdnsError {
    fn from(err: windows_service::Error) -> Self {
        MdnsError::ServiceDispatcher(err.to_string())
    }
}

impl From<ipconfig::error::Error> for MdnsError {
    fn from(err: ipconfig::error::Error) -> Self {
        MdnsError::IpConfig(err.to_string())
    }
}
