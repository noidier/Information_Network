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
#[derive(Clone)]
pub struct NetworkTransport {
    /// The hub this transport is connected to
    hub: Arc<Hub>,
    /// Connected peers
    peers: Arc<RwLock<HashMap<String, NetworkPeer>>>,
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
            peers: Arc::new(RwLock::new(HashMap::new())),
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
        println!("Starting network discovery service");
        
        let hub_id = self.hub.id.clone();
        let hub_scope = self.hub.scope;
        let bind_address = self.bind_address;
        let peers = Arc::clone(&self.peers);
        let tls_config = self.tls_config.clone();
        let self_transport = self.clone();
        
        // Broadcast discovery message to allow other hubs to find this one
        thread::spawn(move || {
            let discovery_port = 8765; // Dedicated discovery port
            let broadcast_addr = SocketAddr::new(
                bind_address.ip().is_ipv4().then(|| "255.255.255.255".parse().unwrap())
                    .unwrap_or_else(|| "ff02::1".parse().unwrap()),
                discovery_port
            );
            
            // Create a broadcast UDP socket
            let socket = match std::net::UdpSocket::bind("0.0.0.0:0") {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to create discovery broadcast socket: {}", e);
                    return;
                }
            };
            
            // Set socket to broadcast mode
            if let Err(e) = socket.set_broadcast(true) {
                eprintln!("Failed to set broadcast mode: {}", e);
                return;
            }
            
            // Listen for discovery responses on a separate socket
            let listen_socket = match std::net::UdpSocket::bind(format!("0.0.0.0:{}", discovery_port)) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to create discovery listen socket: {}", e);
                    return;
                }
            };
            
            // Set listen socket to non-blocking mode
            if let Err(e) = listen_socket.set_nonblocking(true) {
                eprintln!("Failed to set non-blocking mode: {}", e);
            }
            
            // Start listener thread
            let listen_peers = Arc::clone(&peers);
            let listen_tls_config = tls_config.clone();
            let listen_self_transport = self_transport.clone();
            
            thread::spawn(move || {
                let mut buf = [0u8; 1024];
                
                loop {
                    match listen_socket.recv_from(&mut buf) {
                        Ok((size, sender)) => {
                            // Process discovery message
                            if size >= 3 && buf[0] == b'H' && buf[1] == b'U' && buf[2] == b'B' {
                                // Valid discovery message, extract info
                                if let Ok(msg) = std::str::from_utf8(&buf[3..size]) {
                                    if let Some((peer_id, peer_addr_str, peer_scope_str)) = msg.split_once(',')
                                        .and_then(|(id, rest)| rest.split_once(',')
                                        .map(|(addr, scope)| (id, addr, scope))) {
                                        
                                        if let (Ok(peer_addr), Ok(peer_scope)) = (
                                            peer_addr_str.parse::<SocketAddr>(),
                                            match peer_scope_str {
                                                "Thread" => Ok(HubScope::Thread),
                                                "Process" => Ok(HubScope::Process),
                                                "Machine" => Ok(HubScope::Machine),
                                                "Network" => Ok(HubScope::Network),
                                                _ => Err(())
                                            }
                                        ) {
                                            println!("Discovered hub: {} at {} with scope {:?}", 
                                                    peer_id, peer_addr, peer_scope);
                                            
                                            // Don't connect to hubs with lower scope
                                            if peer_scope >= hub_scope {
                                                // Check if we're already connected
                                                let already_connected = {
                                                    let peer_map = listen_peers.read().unwrap();
                                                    peer_map.values().any(|p| p.id == peer_id)
                                                };
                                                
                                                if !already_connected {
                                                    // Connect to the discovered peer
                                                    println!("Connecting to discovered hub: {}", peer_id);
                                                    
                                                    if let Err(e) = listen_self_transport.connect_to_peer(peer_addr) {
                                                        eprintln!("Failed to connect to discovered hub: {}", e);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            // No data available yet, just continue
                            thread::sleep(Duration::from_millis(100));
                        },
                        Err(e) => {
                            eprintln!("Error receiving discovery message: {}", e);
                            thread::sleep(Duration::from_millis(100));
                        }
                    }
                }
            });
            
            // Broadcast loop
            loop {
                // Create discovery message with our hub ID, address and scope
                let message = format!("HUB{},{},{:?}", hub_id, bind_address, hub_scope);
                
                // Broadcast presence
                println!("Broadcasting hub presence: {}", hub_id);
                
                if let Err(e) = socket.send_to(message.as_bytes(), broadcast_addr) {
                    eprintln!("Failed to broadcast discovery message: {}", e);
                }
                
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
    
    /// Send a request to a peer with a timeout
    pub fn send_request_to_peer_with_timeout(
        &self,
        peer_id: &str,
        request: ApiRequest,
        timeout: Duration,
    ) -> Result<ApiResponse> {
        // Create communication channels
        let (tx, rx) = std::sync::mpsc::channel();
        
        // Clone necessary data for the thread
        let self_clone = self.clone();
        let peer_id = peer_id.to_string();
        
        // Spawn a thread to make the request
        thread::spawn(move || {
            match self_clone.send_request_to_peer(&peer_id, request) {
                Ok(response) => {
                    let _ = tx.send(Ok(response));
                },
                Err(e) => {
                    let _ = tx.send(Err(e));
                },
            }
        });
        
        // Wait for response or timeout
        match rx.recv_timeout(timeout) {
            Ok(result) => result,
            Err(_) => Err(HubError::Network(format!("Request to peer {} timed out after {:?}", peer_id, timeout))),
        }
    }
}