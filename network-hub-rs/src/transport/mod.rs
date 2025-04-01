mod tls;
mod network_peer;
mod message_codec;

pub use tls::TlsConfig;
pub use tls::TlsStream;
pub use tls::create_server_tls_stream;
pub use tls::create_client_tls_stream;
pub use network_peer::NetworkPeer;

use crate::error::{HubError, Result};
use crate::hub::{Hub, ApiRequest, ApiResponse, Message};
use crate::utils::current_time_millis;

use std::collections::HashMap;
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use std::io::{Read, Write};

use message_codec::{serialize, deserialize};

/// Network transport layer for hub communication
pub struct NetworkTransport {
    /// The hub this transport is connected to
    hub: Arc<Hub>,
    /// Connected peers
    peers: RwLock<HashMap<String, NetworkPeer>>,
    /// TLS configuration
    tls_config: TlsConfig,
    /// Address to bind to
    bind_address: SocketAddr,
}

impl NetworkTransport {
    /// Create a new network transport
    pub fn new(hub: Arc<Hub>, bind_address: SocketAddr, tls_config: TlsConfig) -> Self {
        NetworkTransport {
            hub,
            peers: RwLock::new(HashMap::new()),
            tls_config,
            bind_address,
        }
    }
    
    /// Start the network transport
    pub fn start(&self) -> Result<()> {
        // Start the network hub server
        let listener = TcpListener::bind(self.bind_address)
            .map_err(|e| HubError::Io(e))?;
            
        println!("Network hub listening on {}", self.bind_address);
        
        // Start discovery service
        self.start_discovery();
        
        // Handle incoming connections
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let hub = Arc::clone(&self.hub);
                    let tls_config = self.tls_config.clone();
                    
                    thread::spawn(move || {
                        if let Err(e) = Self::handle_connection(hub, stream, &tls_config) {
                            eprintln!("Error handling connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Connection error: {}", e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Start discovery service
    fn start_discovery(&self) {
        // In a real implementation, would start a service discovery protocol
        println!("Starting network discovery service");
        
        let hub_id = self.hub.id.clone();
        
        thread::spawn(move || {
            loop {
                // Broadcast presence
                println!("Broadcasting hub presence: {}", hub_id);
                
                // In a real implementation, would broadcast to network
                
                // Sleep for discovery interval
                thread::sleep(Duration::from_secs(30));
            }
        });
    }
    
    /// Handle an incoming connection
    fn handle_connection(hub: Arc<Hub>, stream: TcpStream, tls_config: &TlsConfig) -> Result<()> {
        // Set up TLS
        let mut tls_stream = create_server_tls_stream(stream, tls_config)
            .map_err(|e| HubError::Tls(e.to_string()))?;
            
        // Read message type and content
        let mut buffer = [0u8; 8192];
        loop {
            match tls_stream.read(&mut buffer) {
                Ok(0) => {
                    // Connection closed
                    break;
                }
                Ok(size) => {
                    // Process message
                    let message_data = &buffer[..size];
                    let message_type = message_data.get(0).copied().unwrap_or(0);
                    
                    match message_type {
                        // API request
                        1 => {
                            if let Some(request) = deserialize::<ApiRequest>(&message_data[1..]) {
                                let response = hub.handle_request(request);
                                let response_data = serialize(&response);
                                tls_stream.write(&[2])?; // Response message type
                                tls_stream.write(&response_data)?;
                            }
                        }
                        // Published message
                        3 => {
                            // In a real implementation, would handle published messages
                        }
                        // Heartbeat
                        10 => {
                            tls_stream.write(&[11])?; // Heartbeat response
                        }
                        _ => {
                            eprintln!("Unknown message type: {}", message_type);
                        }
                    }
                }
                Err(e) => {
                    return Err(HubError::Io(e));
                }
            }
        }
        
        Ok(())
    }
    
    /// Connect to a peer
    pub fn connect_to_peer(&self, address: SocketAddr) -> Result<String> {
        // Connect to remote hub
        println!("Connecting to peer at {}", address);
        
        // Establish TCP connection
        let stream = TcpStream::connect(address)
            .map_err(|e| HubError::Io(e))?;
            
        // Set up TLS
        let tls_stream = create_client_tls_stream(stream, &self.tls_config)
            .map_err(|e| HubError::Tls(e.to_string()))?;
            
        // Create peer ID
        let peer_id = format!("peer-{}", address);
        
        // Create network peer
        let peer = NetworkPeer::new(peer_id.clone(), address, tls_stream);
        
        // Store peer connection
        self.peers.write().unwrap().insert(peer_id.clone(), peer);
        
        // In a real implementation, would exchange hub information
        
        Ok(peer_id)
    }
    
    /// Send a request to a peer
    pub fn send_request_to_peer(&self, peer_id: &str, request: ApiRequest) -> Result<ApiResponse> {
        let peers = self.peers.read().unwrap();
        
        if let Some(peer) = peers.get(peer_id) {
            let response = peer.send_request(request)?;
            Ok(response)
        } else {
            Err(HubError::Network(format!("Peer not found: {}", peer_id)))
        }
    }
    
    /// Publish a message to a peer
    pub fn publish_to_peer<T: Send + Sync + 'static>(
        &self,
        peer_id: &str,
        topic: &str,
        data: T,
        metadata: HashMap<String, String>,
    ) -> Result<()> {
        let peers = self.peers.read().unwrap();
        
        if let Some(peer) = peers.get(peer_id) {
            let message = Message {
                topic: topic.to_string(),
                data,
                metadata,
                sender_id: self.hub.id.clone(),
                timestamp: current_time_millis(),
            };
            
            peer.publish_message(message)?;
            Ok(())
        } else {
            Err(HubError::Network(format!("Peer not found: {}", peer_id)))
        }
    }
}