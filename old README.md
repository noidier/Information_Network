# Information Network System

A secure, distributed communication system for building virtual networks that can span from thread to network levels with encrypted communication.

## Core Components

This repository contains the following components:

1. **Hub Core (Rust)** - Central routing and discovery service
   - File: `hub-core.rs`
   - Supports thread, process, machine, and network scopes
   - Manages API registry, message routing, and interception

2. **Node Libraries** - Language-specific client libraries
   - Python: `node_library.py`
   - JavaScript: `node_library.js`
   - API registration and calling
   - Message publishing and subscription
   - Method interception and proxying

3. **Network Transport Layer (Rust)** - TLS encrypted communication
   - File: `network_transport.rs`
   - Secure communication between network hubs
   - Parent-child hub relationships for routing

4. **Secure Reverse Proxy** - Encrypted API gateway
   - Rust example: `reverse_proxy_example.rs`
   - Python implementation: `secure_api_gateway.py`
   - SSL/TLS encrypted connections
   - API routing and interception

## Key Features

- **Hierarchical Routing**: Thread → Process → Machine → Network
- **Message Interception**: Priority-based message handling
- **Function Replacement**: Method proxying and interception
- **Encrypted Communication**: TLS for all network communications
- **Reverse Proxy Capability**: Route APIs across networks securely

## Getting Started

### Prerequisites

- Rust (for core hub and network transport)
- Python 3.6+ (for Python node library)
- Node.js (for JavaScript node library)
- OpenSSL (for TLS certificates)

### Generate TLS Certificates

For testing purposes, you can generate self-signed certificates:

```bash
# Generate a private key
openssl genrsa -out key.pem 2048

# Generate a self-signed certificate
openssl req -new -x509 -key key.pem -out cert.pem -days 365
```

### Running the Rust Reverse Proxy

```bash
cargo run --bin reverse_proxy_example -- --cert /path/to/cert.pem --key /path/to/key.pem
```

Add a route:

```bash
cargo run --bin reverse_proxy_example -- --cert /path/to/cert.pem --key /path/to/key.pem route /api https://api.example.com
```

### Running the Python API Gateway

```bash
python secure_api_gateway.py --cert /path/to/cert.pem --key /path/to/key.pem
```

## Architecture

The system consists of a hierarchical network of hubs:

1. **Thread Hub**: Manages communication within a thread
2. **Process Hub**: Connects thread hubs within a process
3. **Machine Hub**: Connects process hubs on a machine
4. **Network Hub**: Connects machine hubs across the network

Each hub can intercept messages based on patterns and priorities, allowing for dynamic behavior modification.

## Example Usage

### Python Example

```python
from node_library import create_node

# Create a node
node = create_node()

# Register an API
node.register_api('/echo', lambda request: {
    'data': request.data,
    'status': 'success'
})

# Call the API
response = node.call_api('/echo', {'message': 'Hello, World!'})
print(f"Response: {response.data}")

# Set up interception
node.register_interceptor('/echo', lambda message: {
    'intercepted': True,
    'original': message.data
}, priority=10)
```

### JavaScript Example

```javascript
const { createNode } = require('./node_library');

// Create a node
const node = createNode();

// Register an API
node.registerApi('/echo', (request) => {
    return {
        data: request.data,
        status: 'success'
    };
});

// Call the API
const response = node.callApi('/echo', { message: 'Hello, World!' });
console.log(`Response: ${JSON.stringify(response.data)}`);

// Set up interception
node.registerInterceptor('/echo', (message) => {
    return {
        intercepted: true,
        original: message.data
    };
}, 10);
```

## Security Features

- **TLS Encryption**: All network communication is encrypted using TLS
- **API Key Authentication**: Support for API key validation
- **Rate Limiting**: Configurable rate limits for API calls
- **Priority-based Interception**: Higher priority handlers can override security checks

## Use Cases

- **Reverse Proxy**: Route traffic across networks securely
- **Service Mesh**: Connect microservices with encrypted communication
- **API Gateway**: Secure and manage API access
- **Load Balancer**: Distribute traffic across multiple backends
- **Mock Server**: Intercept APIs for testing purposes

## License

See the LICENSE file in the repository.