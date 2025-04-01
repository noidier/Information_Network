use thiserror::Error;
use std::io;

/// Error types for the network hub system
#[derive(Error, Debug)]
pub enum HubError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    
    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    /// TLS error
    #[error("TLS error: {0}")]
    Tls(String),
    
    /// Network error
    #[error("Network error: {0}")]
    Network(String),
    
    /// API not found
    #[error("API not found: {0}")]
    ApiNotFound(String),
    
    /// Hub error
    #[error("Hub error: {0}")]
    Hub(String),
    
    /// Invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),
}

pub type Result<T> = std::result::Result<T, HubError>;