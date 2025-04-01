# Network Hub for Information Network

A secure, distributed communication system with TLS encryption that can span from thread to network levels.

## Features

- **Hierarchical Routing**: Thread → Process → Machine → Network
- **Message Interception**: Priority-based handling of messages
- **Function Replacement**: Dynamic method interception and proxying
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
```

## Building the Project

```bash
# Build the library and binaries
cargo build

# Run the network hub
cargo run --bin network-hub -- --cert /path/to/cert.pem --key /path/to/key.pem

# Run the reverse proxy
cargo run --bin reverse-proxy -- --cert /path/to/cert.pem --key /path/to/key.pem
```

## TLS Certificates

For development and testing, you can generate self-signed certificates:

```bash
# Generate a private key
openssl genrsa -out key.pem 2048

# Generate a self-signed certificate
openssl req -new -x509 -key key.pem -out cert.pem -days 365
```

## Using the Reverse Proxy

The reverse proxy can route HTTP requests to different backend services:

```bash
# Start the proxy with a custom route
cargo run --bin reverse-proxy -- \
  --cert /path/to/cert.pem \
  --key /path/to/key.pem \
  --add-route "/api=https://api.example.com" \
  --add-route "/app=https://app.example.com"
```

## API Documentation

### Hub API

```rust
// Create a hub
let hub = Hub::initialize(HubScope::Network);

// Register an API
hub.register_api("/example", |request| {
    // Handle the request
    ApiResponse {
        data: Box::new("Response data"),
        metadata: HashMap::new(),
        status: ResponseStatus::Success,
    }
}, HashMap::new());

// Register an interceptor
hub.register_interceptor("/example", |message| {
    // Intercept messages to "/example"
    Some("Intercepted response")
}, 10);
```

### Transport API

```rust
// Create and start a network transport
let transport = NetworkTransport::new(hub, bind_addr, tls_config);
transport.start()?;

// Connect to a peer
let peer_id = transport.connect_to_peer("192.168.1.100:8443")?;

// Send a request to a peer
let response = transport.send_request_to_peer(&peer_id, request)?;
```

### Reverse Proxy API

```rust
// Create a reverse proxy
let proxy = HttpReverseProxy::new(hub, bind_addr, tls_config);

// Add routes
proxy.add_route("/api", "https://api.example.com");
proxy.add_route("/app", "https://app.example.com");

// Start the proxy
proxy.start()?;
```

## License

See the LICENSE file in the repository.