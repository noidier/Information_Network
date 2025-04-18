use std::collections::HashMap;
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::sync::{Arc, RwLock};
use std::thread;
use std::io::{Read, Write};

use crate::error::{HubError, Result};
use crate::hub::{Hub, ApiRequest, ApiResponse, ResponseStatus};
use crate::transport::{TlsConfig, create_server_tls_stream};

/// HTTP reverse proxy using the hub
#[derive(Clone)]
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
        let proxy = HttpReverseProxy {
            hub,
            tls_config,
            bind_address,
            route_map: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Register APIs
        proxy.register_proxy_apis();
        
        proxy
    }
    
    /// Start the HTTP reverse proxy
    pub fn start(&self) -> Result<()> {
        // Start the HTTP server
        let listener = TcpListener::bind(self.bind_address)
            .map_err(|e| HubError::Io(e))?;
            
        println!("HTTP reverse proxy listening on {}", self.bind_address);
        
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
        let _hub = Arc::clone(&self.hub);
        
        // Move a clone of self into the closure to avoid the lifetime issue
        let this = self.clone();
        
        let http_handler = move |request: &ApiRequest| {
            // Extract the path from the request
            let path = &request.path[6..]; // Remove "/http/" prefix
            
            println!("HTTP Handler called with path: {}", request.path);
            println!("After prefix removal: {}", path);
            
            // Get additional metadata
            if let Some(method) = request.metadata.get("method") {
                println!("Request method: {}", method);
            }
            
            if let Some(meta_path) = request.metadata.get("path") {
                println!("Path from metadata: {}", meta_path);
            }
            
            // Look up the target
            let map = route_map.read().unwrap();
            let mut target = None;
            
            // Get the actual path from metadata - this is what the test is sending
            // The test includes metadata with the actual path after /http/
            let actual_path = if let Some(metadata_path) = request.metadata.get("path") {
                metadata_path.clone()
            } else {
                path.to_string()
            };
            
            println!("Routes available:");
            for (k, v) in map.iter() {
                println!("  {} -> {}", k, v);
            }
            
            println!("Looking for route matching: {}", actual_path);
            
            // First try root path for the empty or "/" paths
            if actual_path == "/" || actual_path.is_empty() {
                if let Some(t) = map.get("/") {
                    println!("Found root match: / -> {}", t);
                    target = Some(t.clone());
                }
            } 
            
            // Try exact match if we haven't found a target yet
            if target.is_none() {
                if let Some(t) = map.get(&actual_path) {
                    println!("Found exact match: {} -> {}", actual_path, t);
                    target = Some(t.clone());
                } else {
                    // Check for wildcard patterns
                    for (pattern, t) in map.iter() {
                        if pattern.ends_with('*') && actual_path.starts_with(&pattern[0..pattern.len()-1]) {
                            println!("Found wildcard match: {} matches pattern {}", actual_path, pattern);
                            target = Some(t.clone());
                            break;
                        }
                    }
                }
            }
            
            // Use default fallbacks if needed
            if target.is_none() {
                // Try root as fallback
                if let Some(t) = map.get("/") {
                    println!("Using root as fallback for {}", actual_path);
                    target = Some(t.clone());
                } else if let Some(t) = map.get("*") {
                    // Try wildcard as fallback
                    println!("Using '*' as fallback for {}", actual_path);
                    target = Some(t.clone());
                }
            }
            
            if let Some(target) = target {
                println!("Found target: {}", target);
                
                // Forward the request to the target
                return this.forward_request(target, &actual_path, request);
            }
            
            println!("No proxy target found for {}", actual_path);
            ApiResponse {
                data: Box::new(format!("No proxy target found for path: {}", actual_path)),
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
        route_map: Arc<RwLock<HashMap<String, String>>>,
    ) -> Result<()> {
        // Set the stream to non-blocking to prevent indefinite hanging
        stream.set_nonblocking(false).map_err(|e| {
            eprintln!("Error setting stream to blocking mode: {}", e);
            HubError::Io(e)
        })?;
        
        // Log client connection
        let client_addr = stream.peer_addr().map_err(|e| {
            eprintln!("Error getting peer address: {}", e);
            HubError::Io(e)
        })?;
        println!("Client connected from: {}", client_addr);
        
        // Set up TLS
        println!("Setting up TLS for client: {}", client_addr);
        let mut tls_stream = match create_server_tls_stream(stream, tls_config) {
            Ok(stream) => stream,
            Err(e) => {
                eprintln!("TLS setup error for client {}: {}", client_addr, e);
                return Err(e);
            }
        };
        
        // Read HTTP request
        println!("Reading request from client: {}", client_addr);
        let mut buffer = [0u8; 8192];
        let size = match tls_stream.read(&mut buffer) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error reading from stream (client {}): {}", client_addr, e);
                return Err(HubError::Io(e));
            }
        };
        
        if size == 0 {
            println!("Empty request from client: {}", client_addr);
            return Ok(());
        }
        
        // Parse HTTP request
        let http_request = String::from_utf8_lossy(&buffer[..size]);
        let first_line = http_request.lines().next().unwrap_or("");
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        
        if parts.len() >= 2 {
            let method = parts[0];
            let path = parts[1];
            
            println!("Received {} request for {} from {}", method, path, client_addr);
            
            // Print available routes for debugging
            println!("Available routes:");
            {
                let routes = route_map.read().unwrap();
                for (route_path, target) in routes.iter() {
                    println!("  {} -> {}", route_path, target);
                }
            }
            
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
            println!("Forwarding request to hub for path: {}", request.path);
            let response = hub.handle_request(request);
            println!("Got response from hub with status: {:?}", response.status);
            
            // Convert API response to HTTP response
            let http_response = match response.status {
                ResponseStatus::Success | ResponseStatus::Approximated | ResponseStatus::Intercepted => {
                    // Consider approximated and intercepted as successful responses for HTTP clients
                    if let Some(body) = response.data.downcast_ref::<String>() {
                        println!("Sending 200 OK response to client {} (status: {:?})", client_addr, response.status);
                        format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}", 
                            body.len(), body)
                    } else {
                        println!("Sending 200 OK response to client {} (default body, status: {:?})", client_addr, response.status);
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\nOK".to_string()
                    }
                },
                ResponseStatus::NotFound => {
                    println!("Sending 404 Not Found response to client {}", client_addr);
                    "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 9\r\n\r\nNot Found".to_string()
                },
                ResponseStatus::Error => {
                    println!("Sending 500 Internal Server Error response to client {}", client_addr);
                    "HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/plain\r\nContent-Length: 21\r\n\r\nInternal Server Error".to_string()
                }
            };
            
            // Send HTTP response
            println!("Writing response to client: {}", client_addr);
            match tls_stream.write(http_response.as_bytes()) {
                Ok(bytes_written) => println!("Wrote {} bytes to client {}", bytes_written, client_addr),
                Err(e) => {
                    eprintln!("Error writing to client {}: {}", client_addr, e);
                    return Err(HubError::Io(e));
                }
            }
        } else {
            eprintln!("Invalid HTTP request from client {}: '{}'", client_addr, first_line);
            // Send 400 Bad Request
            let bad_request = "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nContent-Length: 11\r\n\r\nBad Request";
            match tls_stream.write(bad_request.as_bytes()) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Error writing 400 response to client {}: {}", client_addr, e);
                    return Err(HubError::Io(e));
                }
            }
        }
        
        println!("Finished handling request from client: {}", client_addr);
        Ok(())
    }
    
    /// Add a proxy route
    pub fn add_route(&self, path: &str, target: &str) {
        let mut map = self.route_map.write().unwrap();
        map.insert(path.to_string(), target.to_string());
        println!("Added proxy route: {} -> {}", path, target);
    }
    
    /// Forward a request to a target URL
    fn forward_request(&self, target: String, path: &str, request: &ApiRequest) -> ApiResponse {
        use std::io::{BufReader, BufRead};
        
        println!("Forwarding request to target: {}{}", target, path);
        
        // Extract method from metadata or default to GET
        let method = request.metadata.get("method").cloned().unwrap_or_else(|| "GET".to_string());
        
        // Parse target URL
        let target_url = if target.ends_with('/') {
            format!("{}{}", target, path.trim_start_matches('/'))
        } else {
            format!("{}{}", target, path)
        };
        
        println!("Target URL: {}", target_url);
        
        // Parse the URL to get host, port, and path
        let url_parts = match url::Url::parse(&target_url) {
            Ok(url) => url,
            Err(e) => {
                eprintln!("Error parsing target URL '{}': {}", target_url, e);
                return ApiResponse {
                    data: Box::new(format!("Error parsing target URL: {}", e)),
                    metadata: HashMap::new(),
                    status: ResponseStatus::Error,
                };
            }
        };
        
        let host = match url_parts.host_str() {
            Some(h) => h.to_string(),
            None => {
                eprintln!("No host in target URL: {}", target_url);
                return ApiResponse {
                    data: Box::new("No host in target URL".to_string()),
                    metadata: HashMap::new(),
                    status: ResponseStatus::Error,
                };
            }
        };
        
        let port = url_parts.port().unwrap_or_else(|| {
            if url_parts.scheme() == "https" { 443 } else { 80 }
        });
        
        let path_with_query = if let Some(query) = url_parts.query() {
            format!("{}?{}", url_parts.path(), query)
        } else {
            url_parts.path().to_string()
        };
        
        println!("Connecting to {}:{} with path {}", host, port, path_with_query);
        
        // Extract request body if present
        let body = if let Some(body_str) = request.data.downcast_ref::<String>() {
            // Real implementation would parse the body from the HTTP request
            // Just using the raw request string for this example
            body_str.clone()
        } else {
            String::new()
        };
        
        // Connect to the target server
        let target_addr = format!("{}:{}", host, port);
        let mut stream = match TcpStream::connect(&target_addr) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error connecting to target server {}: {}", target_addr, e);
                return ApiResponse {
                    data: Box::new(format!("Error connecting to target server: {}", e)),
                    metadata: HashMap::new(),
                    status: ResponseStatus::Error,
                };
            }
        };
        
        // Set stream to blocking mode for simplicity
        if let Err(e) = stream.set_nonblocking(false) {
            eprintln!("Error setting stream to blocking mode: {}", e);
            return ApiResponse {
                data: Box::new(format!("Error setting stream to blocking mode: {}", e)),
                metadata: HashMap::new(),
                status: ResponseStatus::Error,
            };
        }
        
        // Create HTTP request
        let http_request = format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
            method,
            path_with_query,
            host,
            body.len(),
            body
        );
        
        println!("Sending request to target server:");
        println!("{}", http_request);
        
        // Send the request
        if let Err(e) = stream.write_all(http_request.as_bytes()) {
            eprintln!("Error writing to target server: {}", e);
            return ApiResponse {
                data: Box::new(format!("Error writing to target server: {}", e)),
                metadata: HashMap::new(),
                status: ResponseStatus::Error,
            };
        }
        
        // Read the response
        let mut reader = BufReader::new(&stream);
        
        // Read status line
        let mut status_line = String::new();
        if let Err(e) = reader.read_line(&mut status_line) {
            eprintln!("Error reading status line from target server: {}", e);
            return ApiResponse {
                data: Box::new(format!("Error reading status line from target server: {}", e)),
                metadata: HashMap::new(),
                status: ResponseStatus::Error,
            };
        }
        
        println!("Received status line: {}", status_line.trim());
        
        // Parse status code
        let status_parts: Vec<&str> = status_line.split_whitespace().collect();
        let status_code = if status_parts.len() >= 2 {
            match status_parts[1].parse::<u16>() {
                Ok(code) => code,
                Err(_) => {
                    eprintln!("Invalid status code in response: {}", status_line);
                    return ApiResponse {
                        data: Box::new(format!("Invalid status code in response: {}", status_line)),
                        metadata: HashMap::new(),
                        status: ResponseStatus::Error,
                    };
                }
            }
        } else {
            eprintln!("Invalid status line: {}", status_line);
            return ApiResponse {
                data: Box::new(format!("Invalid status line: {}", status_line)),
                metadata: HashMap::new(),
                status: ResponseStatus::Error,
            };
        };
        
        // Read headers
        let mut headers = HashMap::new();
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let line = line.trim();
                    if line.is_empty() {
                        break; // End of headers
                    }
                    
                    if let Some(idx) = line.find(':') {
                        let key = line[..idx].trim().to_lowercase();
                        let value = line[idx+1..].trim().to_string();
                        headers.insert(key, value);
                    }
                },
                Err(e) => {
                    eprintln!("Error reading headers from target server: {}", e);
                    return ApiResponse {
                        data: Box::new(format!("Error reading headers from target server: {}", e)),
                        metadata: HashMap::new(),
                        status: ResponseStatus::Error,
                    };
                }
            }
        }
        
        println!("Received headers:");
        for (key, value) in &headers {
            println!("  {}: {}", key, value);
        }
        
        // Read body
        let content_length = headers.get("content-length")
            .and_then(|s| s.parse::<usize>().ok());
        
        let mut body = Vec::new();
        if let Some(length) = content_length {
            // Read exactly content-length bytes
            let mut buffer = vec![0; length];
            match reader.read_exact(&mut buffer) {
                Ok(_) => body = buffer,
                Err(e) => {
                    eprintln!("Error reading body from target server: {}", e);
                    return ApiResponse {
                        data: Box::new(format!("Error reading body from target server: {}", e)),
                        metadata: HashMap::new(),
                        status: ResponseStatus::Error,
                    };
                }
            }
        } else {
            // Read until EOF
            match reader.read_until(0, &mut body) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Error reading body from target server: {}", e);
                    return ApiResponse {
                        data: Box::new(format!("Error reading body from target server: {}", e)),
                        metadata: HashMap::new(),
                        status: ResponseStatus::Error,
                    };
                }
            }
        }
        
        // Convert the body to a string if possible
        let body_str = match String::from_utf8(body) {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Body is not valid UTF-8");
                return ApiResponse {
                    data: Box::new("Body is not valid UTF-8".to_string()),
                    metadata: HashMap::new(),
                    status: ResponseStatus::Error,
                };
            }
        };
        
        println!("Received body (first 100 chars): {}", 
                 if body_str.len() > 100 { &body_str[..100] } else { &body_str });
        
        // Determine response status based on HTTP status code
        let response_status = match status_code {
            200..=299 => ResponseStatus::Success,
            404 => ResponseStatus::NotFound,
            _ => ResponseStatus::Error,
        };
        
        // Convert headers to metadata
        let metadata: HashMap<String, String> = headers.into_iter().collect();
        
        // Create and return API response
        ApiResponse {
            data: Box::new(body_str),
            metadata,
            status: response_status,
        }
    }
}