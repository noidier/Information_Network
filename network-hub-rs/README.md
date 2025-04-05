# Network Hub for Information Network

A secure, distributed communication system with TLS encryption that can span from thread to network levels. This crate provides a hierarchical hub system that allows communication across scope levels from individual threads up to networked machines.

## Features

- **Hierarchical Routing**: Thread → Process → Machine → Network
- **Message Interception**: Priority-based handling and modification of messages 
- **API Registry**: Register and discover APIs across scope boundaries
- **Request Timeouts**: Built-in timeout handling for network communication
- **Encrypted Communication**: TLS for all network traffic
- **Reverse Proxy**: Secure API gateway with route configuration

## Project Structure

```
src/
  ├── bin/                      - Binary executables
  │   ├── network_hub.rs        - Network hub binary
  │   └── reverse_proxy.rs      - Reverse proxy binary
  ├── hub/                      - Core hub implementation
  │   ├── mod.rs                - Hub module
  │   ├── types.rs              - Core data types
  │   ├── registry.rs           - API registry
  │   └── interceptor.rs        - Interceptor management
  ├── transport/                - Network transport layer
  │   ├── mod.rs                - Transport module
  │   ├── tls.rs                - TLS implementation
  │   ├── network_peer.rs       - Peer management
  │   └── message_codec.rs      - Message serialization
  ├── proxy/                    - Reverse proxy implementation
  │   └── mod.rs                - HTTP reverse proxy
  ├── error.rs                  - Error types
  ├── utils.rs                  - Utility functions
  └── lib.rs                    - Library exports
examples/
  ├── distributed_hubs.rs       - Example of multi-level hub hierarchy
  └── timeout_handling.rs       - Example of timeout handling
tests/
  ├── hub_communication_tests.rs - Tests for hub-to-hub communication
  ├── network_hub_tests.rs       - Tests for network communication
  ├── hub_tests.rs               - Tests for hub core functionality
  └── integration_tests.rs       - End-to-end integration tests
```

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
network-hub-rs = "0.1.0"
```

## Building the Project

```bash
# Build the library and binaries
cargo build

# Run the examples
cargo run --example distributed_hubs
cargo run --example timeout_handling

# Run tests
cargo test
```

## Getting Started

### Create a Basic Hub

```rust
use std::collections::HashMap;
use std::sync::Arc;
use network_hub::{Hub, HubScope, ApiRequest, ApiResponse, ResponseStatus};

// Create a hub at the Thread scope
let hub = Arc::new(Hub::new(HubScope::Thread));

// Register an API endpoint
hub.register_api("/example/api", |request: &ApiRequest| {
    // Handle the request
    println!("Received request at /example/api");
    
    ApiResponse {
        data: Box::new("Hello from the hub!"),
        metadata: HashMap::new(),
        status: ResponseStatus::Success,
    }
}, HashMap::new());

// Make a request to the API
let request = ApiRequest {
    path: "/example/api".to_string(),
    data: Box::new(()),
    metadata: HashMap::new(),
    sender_id: "client".to_string(),
};

let response = hub.handle_request(request);
assert_eq!(response.status, ResponseStatus::Success);
```

### Using Multiple Hub Levels

```rust
// Create hubs at different scope levels
let thread_hub = Arc::new(Hub::new(HubScope::Thread));
let process_hub = Arc::new(Hub::new(HubScope::Process));

// Connect thread hub to process hub
thread_hub.connect_to_parent(Arc::clone(&process_hub)).unwrap();

// Register an API at the process level
process_hub.register_api("/process/api", |_| {
    ApiResponse {
        data: Box::new("Response from process hub"),
        metadata: HashMap::new(),
        status: ResponseStatus::Success,
    }
}, HashMap::new());

// Call the process API from the thread hub (request will be elevated)
let request = ApiRequest {
    path: "/process/api".to_string(),
    data: Box::new(()),
    metadata: HashMap::new(),
    sender_id: "client".to_string(),
};

let response = thread_hub.handle_request(request);
// Response comes from process hub through the hierarchy
```

### Network Transport with Timeouts

```rust
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use network_hub::transport::{NetworkTransport, TlsConfig};

// Create hubs
let hub1 = Arc::new(Hub::new(HubScope::Network));
let hub2 = Arc::new(Hub::new(HubScope::Network));

// Create TLS configuration
let tls_config = TlsConfig {
    cert_path: "certs/cert.pem".to_string(),
    key_path: "certs/key.pem".to_string(),
    ca_path: None,
};

// Create network transports
let addr1 = SocketAddr::from_str("127.0.0.1:9001").unwrap();
let addr2 = SocketAddr::from_str("127.0.0.1:9002").unwrap();

let transport1 = NetworkTransport::new(Arc::clone(&hub1), addr1, tls_config.clone());
let transport2 = NetworkTransport::new(Arc::clone(&hub2), addr2, tls_config.clone());

// Start transports in separate threads
std::thread::spawn(move || transport1.start().unwrap());
std::thread::spawn(move || transport2.start().unwrap());

// Connect transport1 to transport2
let peer_id = transport1.connect_to_peer(addr2).unwrap();

// Send a request with a timeout
let request = ApiRequest {
    path: "/some/api".to_string(),
    data: Box::new(()),
    metadata: HashMap::new(),
    sender_id: hub1.id.clone(),
};

// 500ms timeout
match transport1.send_request_to_peer_with_timeout(&peer_id, request, Duration::from_millis(500)) {
    Ok(response) => println!("Got response: {:?}", response.status),
    Err(e) => println!("Request timed out or failed: {}", e),
}
```

### API Interception

```rust
// Register regular API
hub.register_api("/data/fetch", |_| {
    ApiResponse {
        data: Box::new("original data"),
        metadata: HashMap::new(),
        status: ResponseStatus::Success,
    }
}, HashMap::new());

// Register interceptor for the API
hub.register_api_interceptor("/data/fetch", |request| {
    // Intercept only when specific flag is set
    if let Some(flag) = request.metadata.get("intercept") {
        if flag == "true" {
            return Some(ApiResponse {
                data: Box::new("intercepted data"),
                metadata: HashMap::from([("intercepted".to_string(), "true".to_string())]),
                status: ResponseStatus::Intercepted,
            });
        }
    }
    None
}, 10); // Priority 10

// Make a request with interception
let request = ApiRequest {
    path: "/data/fetch".to_string(),
    data: Box::new(()),
    metadata: HashMap::from([("intercept".to_string(), "true".to_string())]),
    sender_id: "client".to_string(),
};

let response = hub.handle_request(request);
// Will return the intercepted response
```

## TLS Certificates

For development and testing, you can generate self-signed certificates:

```bash
# Generate a private key
openssl genrsa -out certs/key.pem 2048

# Generate a self-signed certificate
openssl req -new -x509 -key certs/key.pem -out certs/cert.pem -days 365
```

## Running the Examples

```bash
# Run the distributed hubs example (demonstrates multi-level hub hierarchy)
cargo run --example distributed_hubs

# Run the timeout handling example (demonstrates timeout mechanisms)
cargo run --example timeout_handling

# Run the cross-network communication example (demonstrates network communication and discovery)
cargo run --example cross_network_communication
```

## Running the Tests

```bash
# Run all tests
cargo test

# Run just the communication tests
cargo test --test hub_communication_tests

# Run tests with output
cargo test -- --nocapture
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.