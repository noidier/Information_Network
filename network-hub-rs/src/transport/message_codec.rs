use serde::{Serialize, Deserialize};
use std::any::Any;
use crate::hub::{ApiRequest, ApiResponse, Message};
use std::collections::HashMap;

// Simple message enum for network transport
#[derive(Serialize, Deserialize)]
enum TransportMessage {
    Request {
        path: String,
        data: String,
        metadata: HashMap<String, String>,
        sender_id: String,
    },
    Response {
        data: String,
        metadata: HashMap<String, String>,
        status: u8, // 0=success, 1=not found, 2=error, 3=intercepted, 4=approximated
    },
    PubMessage {
        topic: String,
        data: String,
        metadata: HashMap<String, String>,
        sender_id: String,
        timestamp: u64,
    },
}

/// Serialize data to bytes
pub fn serialize<T: Send + Sync + 'static>(data: &T) -> Vec<u8> {
    // Try to convert the data based on its type
    if let Some(req) = (data as &dyn Any).downcast_ref::<ApiRequest>() {
        // Extract string data from Box<dyn Any>
        let str_data = match req.data.downcast_ref::<String>() {
            Some(s) => s.clone(),
            _ => match req.data.downcast_ref::<&str>() {
                Some(s) => s.to_string(),
                _ => "".to_string(),
            }
        };
        
        let message = TransportMessage::Request {
            path: req.path.clone(),
            data: str_data,
            metadata: req.metadata.clone(),
            sender_id: req.sender_id.clone(),
        };
        
        if let Ok(bytes) = serde_json::to_vec(&message) {
            return bytes;
        }
    } 
    else if let Some(resp) = (data as &dyn Any).downcast_ref::<ApiResponse>() {
        // Extract string data from Box<dyn Any>
        let str_data = match resp.data.downcast_ref::<String>() {
            Some(s) => s.clone(),
            _ => match resp.data.downcast_ref::<&str>() {
                Some(s) => s.to_string(),
                _ => "".to_string(),
            }
        };
        
        // Convert status to u8
        let status_code = match resp.status {
            crate::hub::ResponseStatus::Success => 0,
            crate::hub::ResponseStatus::NotFound => 1,
            crate::hub::ResponseStatus::Error => 2,
            crate::hub::ResponseStatus::Intercepted => 3,
            crate::hub::ResponseStatus::Approximated => 4,
        };
        
        let message = TransportMessage::Response {
            data: str_data,
            metadata: resp.metadata.clone(),
            status: status_code,
        };
        
        if let Ok(bytes) = serde_json::to_vec(&message) {
            return bytes;
        }
    }
    else if let Some(msg) = (data as &dyn Any).downcast_ref::<Message<String>>() {
        let message = TransportMessage::PubMessage {
            topic: msg.topic.clone(),
            data: msg.data.clone(),
            metadata: msg.metadata.clone(),
            sender_id: msg.sender_id.clone(),
            timestamp: msg.timestamp,
        };
        
        if let Ok(bytes) = serde_json::to_vec(&message) {
            return bytes;
        }
    }
    else if let Some(msg) = (data as &dyn Any).downcast_ref::<Message<&str>>() {
        let message = TransportMessage::PubMessage {
            topic: msg.topic.clone(),
            data: msg.data.to_string(),
            metadata: msg.metadata.clone(),
            sender_id: msg.sender_id.clone(),
            timestamp: msg.timestamp,
        };
        
        if let Ok(bytes) = serde_json::to_vec(&message) {
            return bytes;
        }
    }
    
    // Default case - return empty data
    println!("Serialization not implemented for type: {}", std::any::type_name::<T>());
    Vec::new()
}

/// Deserialize bytes to data
pub fn deserialize<T: Send + Sync + 'static>(bytes: &[u8]) -> Option<T> {
    // Determine what message type we're deserializing to
    let type_id = std::any::TypeId::of::<T>();
    
    // Try to parse as our transport message
    if let Ok(message) = serde_json::from_slice::<TransportMessage>(bytes) {
        if type_id == std::any::TypeId::of::<ApiRequest>() {
            // Only handle Request message type for ApiRequest
            if let TransportMessage::Request { path, data, metadata, sender_id } = message {
                // Create a new ApiRequest
                let request = ApiRequest {
                    path,
                    data: Box::new(data),
                    metadata,
                    sender_id,
                };
                
                // Convert it to the expected type using any_box cast
                let boxed: Box<dyn Any> = Box::new(request);
                // This is safe because we've verified T is ApiRequest
                return boxed.downcast::<T>().ok().map(|t| *t);
            }
        }
        else if type_id == std::any::TypeId::of::<ApiResponse>() {
            // Only handle Response message type for ApiResponse
            if let TransportMessage::Response { data, metadata, status } = message {
                // Convert status from u8
                let response_status = match status {
                    0 => crate::hub::ResponseStatus::Success,
                    1 => crate::hub::ResponseStatus::NotFound,
                    2 => crate::hub::ResponseStatus::Error,
                    3 => crate::hub::ResponseStatus::Intercepted,
                    4 => crate::hub::ResponseStatus::Approximated,
                    _ => crate::hub::ResponseStatus::Error,
                };
                
                let response = ApiResponse {
                    data: Box::new(data),
                    metadata,
                    status: response_status,
                };
                
                // Convert it to the expected type using any_box cast
                let boxed: Box<dyn Any> = Box::new(response);
                // This is safe because we've verified T is ApiResponse
                return boxed.downcast::<T>().ok().map(|t| *t);
            }
        }
        else if type_id == std::any::TypeId::of::<Message<String>>() {
            // Only handle PubMessage message type for Message<String>
            if let TransportMessage::PubMessage { topic, data, metadata, sender_id, timestamp } = message {
                let pub_message = Message {
                    topic,
                    data,
                    metadata,
                    sender_id,
                    timestamp,
                };
                
                // Convert it to the expected type using any_box cast
                let boxed: Box<dyn Any> = Box::new(pub_message);
                // This is safe because we've verified T is Message<String>
                return boxed.downcast::<T>().ok().map(|t| *t);
            }
        }
    }
    
    println!("Deserialization failed for type: {}", std::any::type_name::<T>());
    None
}