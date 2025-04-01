use std::collections::HashMap;
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::sync::{Arc, RwLock};
use std::thread;
use std::io::{Read, Write};

use crate::error::{HubError, Result};
use crate::hub::{Hub, ApiRequest, ApiResponse, ResponseStatus};
use crate::transport::{TlsConfig, create_server_tls_stream};

/// HTTP reverse proxy using the hub
pub struct HttpReverseProxy {
    /// The hub this proxy is connected to
    hub: Arc<Hub>,
    /// TLS configuration
    tls_config: TlsConfig,
    /// Address to bind to
    bind_address: SocketAddr,
    /// Map of path patterns to target URLs
    route_map: Arc<RwLock<HashMap<String, String>>>,
}

impl HttpReverseProxy {
    /// Create a new HTTP reverse proxy
    pub fn new(hub: Arc<Hub>, bind_address: SocketAddr, tls_config: TlsConfig) -> Self {
        HttpReverseProxy {
            hub,
            tls_config,
            bind_address,
            route_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Start the HTTP reverse proxy
    pub fn start(&self) -> Result<()> {
        // Start the HTTP server
        let listener = TcpListener::bind(self.bind_address)
            .map_err(|e| HubError::Io(e))?;
            
        println!("HTTP reverse proxy listening on {}", self.bind_address);
        
        // Register proxy API with the hub
        self.register_proxy_apis();
        
        // Handle incoming connections
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let hub = Arc::clone(&self.hub);
                    let tls_config = self.tls_config.clone();
                    let route_map = Arc::clone(&self.route_map);
                    
                    thread::spawn(move || {
                        if let Err(e) = Self::handle_http_connection(hub, stream, &tls_config, route_map) {
                            eprintln!("Error handling HTTP connection: {}", e);
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
    
    /// Register proxy APIs with the hub
    fn register_proxy_apis(&self) {
        // Register a handler for configuring proxy routes
        let route_map = Arc::clone(&self.route_map);
        
        let register_handler = move |request: &ApiRequest| {
            // Extract path and target from request
            if let Some(path) = request.data.downcast_ref::<String>() {
                if let Some(target) = request.metadata.get("target") {
                    let mut map = route_map.write().unwrap();
                    map.insert(path.clone(), target.clone());
                    
                    println!("Registered proxy route: {} -> {}", path, target);
                    
                    return ApiResponse {
                        data: Box::new(true),
                        metadata: HashMap::new(),
                        status: ResponseStatus::Success,
                    };
                }
            }
            
            ApiResponse {
                data: Box::new(false),
                metadata: HashMap::new(),
                status: ResponseStatus::Error,
            }
        };
        
        self.hub.register_api("/proxy/register", register_handler, HashMap::new());
        
        // Register a wildcard API for handling all HTTP requests
        let route_map = Arc::clone(&self.route_map);
        
        let http_handler = move |request: &ApiRequest| {
            // Extract the path from the request
            let path = &request.path[6..]; // Remove "/http/" prefix
            
            // Look up the target
            let map = route_map.read().unwrap();
            let mut target = None;
            
            // Check for exact match
            if let Some(t) = map.get(path) {
                target = Some(t.clone());
            } else {
                // Check for wildcard patterns
                for (pattern, t) in map.iter() {
                    if pattern.ends_with('*') && path.starts_with(&pattern[0..pattern.len()-1]) {
                        target = Some(t.clone());
                        break;
                    }
                }
                
                // Default handler
                if target.is_none() {
                    target = map.get("*").cloned();
                }
            }
            
            if let Some(target) = target {
                // In a real implementation, would forward the request to the target
                return ApiResponse {
                    data: Box::new(format!("Proxied to {}", target)),
                    metadata: HashMap::from([
                        ("content-type".to_string(), "text/plain".to_string()),
                    ]),
                    status: ResponseStatus::Success,
                };
            }
            
            ApiResponse {
                data: Box::new("No proxy target found"),
                metadata: HashMap::new(),
                status: ResponseStatus::NotFound,
            }
        };
        
        self.hub.register_api("/http/*", http_handler, HashMap::new());
    }
    
    /// Handle an HTTP connection
    fn handle_http_connection(
        hub: Arc<Hub>,
        stream: TcpStream,
        tls_config: &TlsConfig,
        _route_map: Arc<RwLock<HashMap<String, String>>>,
    ) -> Result<()> {
        // Set up TLS
        let mut tls_stream = create_server_tls_stream(stream, tls_config)?;
        
        // Read HTTP request
        let mut buffer = [0u8; 8192];
        let size = tls_stream.read(&mut buffer)?;
        
        if size == 0 {
            return Ok(());
        }
        
        // Parse HTTP request
        let http_request = String::from_utf8_lossy(&buffer[..size]);
        let first_line = http_request.lines().next().unwrap_or("");
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        
        if parts.len() >= 2 {
            let method = parts[0];
            let path = parts[1];
            
            // Create API request
            let request = ApiRequest {
                path: format!("/http{}", path),
                data: Box::new(http_request.to_string()),
                metadata: HashMap::from([
                    ("method".to_string(), method.to_string()),
                    ("path".to_string(), path.to_string()),
                ]),
                sender_id: "http-client".to_string(),
            };
            
            // Handle request using the hub
            let response = hub.handle_request(request);
            
            // Convert API response to HTTP response
            let http_response = match response.status {
                ResponseStatus::Success => {
                    if let Some(body) = response.data.downcast_ref::<String>() {
                        format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}", 
                            body.len(), body)
                    } else {
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\nOK".to_string()
                    }
                },
                ResponseStatus::NotFound => {
                    "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 9\r\n\r\nNot Found".to_string()
                },
                _ => {
                    "HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/plain\r\nContent-Length: 21\r\n\r\nInternal Server Error".to_string()
                }
            };
            
            // Send HTTP response
            tls_stream.write(http_response.as_bytes())?;
        }
        
        Ok(())
    }
    
    /// Add a proxy route
    pub fn add_route(&self, path: &str, target: &str) {
        let mut map = self.route_map.write().unwrap();
        map.insert(path.to_string(), target.to_string());
        println!("Added proxy route: {} -> {}", path, target);
    }
}