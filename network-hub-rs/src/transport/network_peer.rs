use std::net::SocketAddr;
use std::sync::{Mutex, Arc};
use std::io::{Read, Write};

use crate::error::{HubError, Result};
use crate::hub::{ApiRequest, ApiResponse, Message};
use crate::transport::TlsStream;
use crate::transport::message_codec::{serialize, deserialize};

/// A connected network peer
pub struct NetworkPeer {
    /// Peer ID
    pub id: String,
    /// Peer address
    pub address: SocketAddr,
    /// TLS stream for communication
    stream: Arc<Mutex<TlsStream>>,
    /// Last seen timestamp
    pub last_seen: u64,
}

impl Clone for NetworkPeer {
    fn clone(&self) -> Self {
        NetworkPeer {
            id: self.id.clone(),
            address: self.address,
            stream: Arc::clone(&self.stream),
            last_seen: self.last_seen,
        }
    }
}

impl NetworkPeer {
    /// Create a new network peer
    pub fn new(id: String, address: SocketAddr, stream: TlsStream) -> Self {
        NetworkPeer {
            id,
            address,
            stream: Arc::new(Mutex::new(stream)),
            last_seen: 0,
        }
    }
    
    /// Send a request to the peer
    pub fn send_request(&self, request: ApiRequest) -> Result<ApiResponse> {
        // Serialize request
        let request_data = serialize(&request);
        
        // Lock the stream for the duration of this operation
        let mut stream = self.stream.lock().unwrap();
        
        // Send message type (1 = API request) and data
        stream.write(&[1])?;
        stream.write(&request_data)?;
        
        // Read response
        let mut buffer = [0u8; 8192];
        let size = stream.read(&mut buffer)?;
        
        if size == 0 {
            return Err(HubError::Network("Connection closed".to_string()));
        }
        
        // Check message type (2 = API response)
        let message_type = buffer[0];
        if message_type != 2 {
            return Err(HubError::Network(format!("Unexpected message type: {}", message_type)));
        }
        
        // Deserialize response
        deserialize::<ApiResponse>(&buffer[1..size])
            .ok_or_else(|| HubError::Network("Failed to deserialize response".to_string()))
    }
    
    /// Publish a message to the peer
    pub fn publish_message<T: Send + Sync + 'static>(
        &self,
        message: Message<T>,
    ) -> Result<()> {
        // Serialize message
        let message_data = serialize(&message);
        
        // Lock the stream for the duration of this operation
        let mut stream = self.stream.lock().unwrap();
        
        // Send message type (3 = Published message) and data
        stream.write(&[3])?;
        stream.write(&message_data)?;
        
        Ok(())
    }
    
    /// Send a heartbeat to check if the peer is alive
    pub fn send_heartbeat(&self) -> Result<bool> {
        let mut stream = self.stream.lock().unwrap();
        
        // Send heartbeat message type (10)
        stream.write(&[10])?;
        
        // Read response
        let mut buffer = [0u8; 1];
        let size = stream.read(&mut buffer)?;
        
        if size == 0 {
            return Err(HubError::Network("Connection closed".to_string()));
        }
        
        // Check message type (11 = Heartbeat response)
        Ok(buffer[0] == 11)
    }
}