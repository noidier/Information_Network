use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};

/// Represents a scope level of the hub
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HubScope {
    /// Thread-level scope (within a single thread)
    Thread,
    /// Process-level scope (across threads in a process)
    Process,
    /// Machine-level scope (across processes on a machine)
    Machine,
    /// Network-level scope (across machines on a network)
    Network,
}

/// Message with typed data
pub struct Message<T> {
    /// Topic of the message
    pub topic: String,
    /// Message data
    pub data: T,
    /// Message metadata
    pub metadata: HashMap<String, String>,
    /// Sender ID
    pub sender_id: String,
    /// Timestamp (milliseconds since epoch)
    pub timestamp: u64,
}

/// Request to an API endpoint
pub struct ApiRequest {
    /// API path
    pub path: String,
    /// Request data
    pub data: Box<dyn Any + Send + Sync>,
    /// Request metadata
    pub metadata: HashMap<String, String>,
    /// Sender ID
    pub sender_id: String,
}

/// Response from an API endpoint
pub struct ApiResponse {
    /// Response data
    pub data: Box<dyn Any + Send + Sync>,
    /// Response metadata
    pub metadata: HashMap<String, String>,
    /// Response status
    pub status: ResponseStatus,
}

/// Status of an API response
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseStatus {
    /// Success
    Success,
    /// Not found
    NotFound,
    /// Error
    Error,
    /// Intercepted
    Intercepted,
    /// Approximated
    Approximated,
}

/// A subscription to messages
pub struct Subscription {
    /// Subscription ID
    pub id: String,
    /// Priority (higher priorities are checked first)
    pub priority: i32,
    /// Message handler function
    pub handler: Arc<Mutex<Box<dyn Fn(&Message<Box<dyn Any + Send + Sync>>) -> Option<Box<dyn Any + Send + Sync>> + Send + Sync>>>,
}

/// An interceptor for messages or API requests
pub struct Interceptor<T, R> {
    /// Interceptor ID
    pub id: String,
    /// Priority (higher priorities are checked first)
    pub priority: i32,
    /// Interceptor handler function
    pub handler: Arc<dyn Fn(&T) -> Option<R> + Send + Sync>,
}