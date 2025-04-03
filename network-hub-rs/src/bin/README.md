# Thread Hub Demo

This directory contains several binary examples for demonstrating the Information Network Hub architecture.

## Hub Demo (`hub_demo.rs`)

The Hub Demo showcases how to connect two thread hubs together over a network connection. This demo:

1. Creates two separate network-level hubs
2. Initializes network transports for both hubs on different ports
3. Establishes bidirectional connections between the hubs
4. Registers API endpoints on each hub
5. Demonstrates API requests between hubs
6. Shows message publishing between hubs

### Running the Demo

```bash
cargo run --bin hub-demo
```

### Key Components

- **Hub**: The central component that manages routing and discovery
- **NetworkTransport**: Handles network communication for hubs
- **TLS Configuration**: Used for secure communication

### Demo Flow

1. **Setup**: Creates two hubs with network scope and configures TLS
2. **Connection**: Establishes connections between hubs using TCP/TLS
3. **API Registration**: Each hub registers an API endpoint
4. **Request Exchange**: Hubs send API requests to each other
5. **Message Publishing**: Hubs publish messages to each other

### Implementation Notes

The demo uses pseudocode implementations for some components:
- Serialization/deserialization placeholders in `message_codec.rs`
- Simplified TLS implementation in `tls.rs`

In a production implementation, these would be replaced with proper implementations using libraries like serde for serialization and rustls for TLS.

## Architecture

The Information Network Hub architecture allows for seamless communication across various scope levels:

- **Thread**: Within a single thread
- **Process**: Across threads in a process
- **Machine**: Across processes on a machine
- **Network**: Across machines on a network

Each hub can connect to parent hubs with larger scope, creating a hierarchical routing network.