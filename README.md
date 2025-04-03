# Information Network Hub System

This document provides comprehensive instructions for using the Information Network Hub system, including the Rust core hub, Python and JavaScript clients, and the shallow hub implementations.

## Table of Contents

1. [System Architecture](#system-architecture)
2. [Components](#components)
3. [Installation](#installation)
4. [Running the Rust Core Hub](#running-the-rust-core-hub)
5. [Python Client API](#python-client-api)
6. [JavaScript Client API](#javascript-client-api)
7. [Shallow Hub for In-Process Communication](#shallow-hub-for-in-process-communication)
8. [Common Usage Patterns](#common-usage-patterns)
9. [API Examples](#api-examples)
10. [Advanced Usage](#advanced-usage)

## System Architecture

The Information Network Hub system is a distributed communication platform designed to work across different scope levels:

- **Thread**: Communication between threads in the same process
- **Process**: Communication between processes on the same machine
- **Machine**: Communication between applications on the same machine
- **Network**: Communication between machines across a network

The system follows a hub-and-spoke model where nodes connect to hubs, and hubs can connect to parent hubs in a hierarchical structure. This allows messages to flow up and down the hierarchy, providing a unified messaging system across different scope levels.

### Key Features

- **API Registration and Calling**: Nodes can register APIs and call APIs on other nodes
- **Message Publishing and Subscription**: Nodes can publish messages and subscribe to message patterns
- **Message Interception**: Intercept and potentially modify messages based on patterns
- **TLS Encryption**: Secure network communication
- **Hub Hierarchy**: Hubs can connect to parent hubs to form a hierarchy
- **Pattern-Based Routing**: Messages can be routed based on patterns
- **Local and Remote Operation**: Works both within a process and across a network

## Components

The system consists of several components:

1. **Network Hub Rust Core** (`network-hub-rs/`): The central hub implementation in Rust
2. **Python Client** (`network_hub_python_client.py`): Python client to connect to the hub
3. **JavaScript Client** (`network_hub_js_client.js`): JavaScript client to connect to the hub
4. **Python Shallow Hub** (`shallow_hub_python.py`): Lightweight hub for in-process communication in Python
5. **JavaScript Shallow Hub** (`shallow_hub_js.js`): Lightweight hub for in-process communication in JavaScript

## Installation

### Rust Core Hub

```bash
cd network-hub-rs
cargo build
```

### Python Dependencies

```bash
pip install uuid
```

### JavaScript Dependencies

None for the basic client. If using Node.js:

```bash
npm install ws  # If using WebSockets
```

## Running the Rust Core Hub

### Generate TLS Certificates

For development and testing, you can generate self-signed certificates:

```bash
# Generate a private key
openssl genrsa -out certs/key.pem 2048

# Generate a self-signed certificate
openssl req -new -x509 -key certs/key.pem -out certs/cert.pem -days 365
```

### Start the Network Hub

```bash
cd network-hub-rs
cargo run --bin network-hub -- --cert certs/cert.pem --key certs/key.pem --port 8443
```

### Run the Network Hub Demo

```bash
cd network-hub-rs
cargo run --bin hub-demo
```

### Run the Thread Hub Demo

```bash
cd network-hub-rs
cargo run --bin thread-hub-demo
```

## Python Client API

The Python client provides a simple API to connect to the network hub.

### Connecting to a Network Hub

```python
from network_hub_python_client import create_node

# Create a node and connect to a hub
node = create_node(
    host="localhost", 
    port=8443,
    cert_path="certs/client.pem",
    key_path="certs/client_key.pem",
    verify_ssl=False  # For testing only
)

# Connect to the hub
node.connect()

# When done
node.disconnect()
```

### Core API Methods

```python
# Register an API
node.register_api("/my/api", handler_function, metadata={})

# Call an API
response = node.call_api("/some/api", data={"key": "value"})

# Subscribe to messages
subscription_id = node.subscribe("topic/*", message_handler)

# Publish a message
result = node.publish("topic/name", data={"message": "Hello"})
```

## JavaScript Client API

The JavaScript client provides a Promise-based API to connect to the network hub.

### Connecting to a Network Hub

```javascript
const { createNode } = require('./network_hub_js_client');
// Or in browser: const { createNode } = window.NetworkHub;

// Create a node and connect to a hub
const node = createNode({
  host: 'localhost',
  port: 8443,
  useTls: true,
  rejectUnauthorized: false, // For testing only
  certificates: {
    cert: 'path/to/client.pem',
    key: 'path/to/client_key.pem'
  }
});

// Connect to the hub
await node.connect();

// When done
node.disconnect();
```

### Core API Methods

```javascript
// Register an API
await node.registerApi('/my/api', handlerFunction, {});

// Call an API
const response = await node.callApi('/some/api', { key: 'value' });

// Subscribe to messages
const subscriptionId = await node.subscribe('topic/*', messageHandler);

// Publish a message
const result = await node.publish('topic/name', { message: 'Hello' });
```

## Shallow Hub for In-Process Communication

Both Python and JavaScript implementations provide a shallow hub for in-process communication without networking.

### Python Shallow Hub

```python
from shallow_hub_python import create_shallow_hub, create_node

# Create a shallow hub
hub = create_shallow_hub()

# Create nodes connected to the hub
node1 = create_node(hub, "node1")
node2 = create_node(hub, "node2")

# Register APIs, publish messages, etc.
node1.register_api("/node1/api", handler_function)
response = node2.call_api("/node1/api", data={"key": "value"})

# When done
node1.disconnect()
node2.disconnect()
hub.shutdown()
```

### JavaScript Shallow Hub

```javascript
const { createShallowHub, createNode } = require('./shallow_hub_js');
// Or in browser: const { createShallowHub, createNode } = window.ShallowHub;

// Create a shallow hub
const hub = createShallowHub();

// Create nodes connected to the hub
const node1 = createNode(hub, 'node1');
const node2 = createNode(hub, 'node2');

// Register APIs, publish messages, etc.
await node1.registerApi('/node1/api', handlerFunction);
const response = await node2.callApi('/node1/api', { key: 'value' });

// When done
node1.disconnect();
node2.disconnect();
hub.shutdown();
```

## Common Usage Patterns

### Setting Up a Hub Hierarchy

1. Start with a network hub for cross-machine communication
2. Connect process hubs to the network hub
3. Connect thread hubs to process hubs
4. Connect nodes to appropriate hubs based on their scope

### Implementing a Service

1. Create a node
2. Register APIs for the service's capabilities
3. Subscribe to relevant message topics
4. Connect to an appropriate hub (thread, process, network)

### Message Interception

1. Register an interceptor for a specific pattern
2. Intercept and optionally modify messages matching the pattern
3. Return a value from the interceptor to prevent further processing

### Proxying Requests

1. Register an API that forwards requests to another API
2. Use the same node to call other APIs and return the results

## API Examples

### Python API Example

```python
from network_hub_python_client import create_node, ApiResponse, ResponseStatus

# Create and connect a node
node = create_node(host="localhost", port=8443)
node.connect()

# Register an API for calculator functionality
def add_handler(request):
    try:
        a = request.data["a"]
        b = request.data["b"]
        result = a + b
        return ApiResponse(
            data={"result": result},
            metadata={},
            status=ResponseStatus.SUCCESS
        )
    except Exception as e:
        return ApiResponse(
            data={"error": str(e)},
            metadata={},
            status=ResponseStatus.ERROR
        )

# Register the API
node.register_api("/calculator/add", add_handler)

# Call the API
response = node.call_api("/calculator/add", {"a": 5, "b": 3})
print(f"Result: {response.data['result']}")  # Output: Result: 8

# Subscribe to calculator events
def calc_event_handler(message):
    print(f"Calculator event: {message.topic} - {message.data}")

node.subscribe("calculator/events/*", calc_event_handler)

# Publish a calculation event
node.publish("calculator/events/addition", {
    "operation": "add",
    "operands": [5, 3],
    "result": 8
})
```

### JavaScript API Example

```javascript
const { createNode, ApiResponse, ResponseStatus } = require('./network_hub_js_client');

async function main() {
  // Create and connect a node
  const node = createNode({
    host: 'localhost',
    port: 8443
  });
  await node.connect();
  
  // Register an API for calculator functionality
  await node.registerApi('/calculator/multiply', (request) => {
    try {
      const a = request.data.a;
      const b = request.data.b;
      const result = a * b;
      return new ApiResponse(
        { result },
        {},
        ResponseStatus.SUCCESS
      );
    } catch (error) {
      return new ApiResponse(
        { error: error.message },
        {},
        ResponseStatus.ERROR
      );
    }
  });
  
  // Call the API
  const response = await node.callApi('/calculator/multiply', { a: 5, b: 3 });
  console.log(`Result: ${response.data.result}`);  // Output: Result: 15
  
  // Subscribe to calculator events
  await node.subscribe('calculator/events/*', (message) => {
    console.log(`Calculator event: ${message.topic} -`, message.data);
  });
  
  // Publish a calculation event
  await node.publish('calculator/events/multiplication', {
    operation: 'multiply',
    operands: [5, 3],
    result: 15
  });
  
  // Disconnect when done
  node.disconnect();
}

main().catch(console.error);
```

### Python Shallow Hub Example

```python
from shallow_hub_python import create_shallow_hub, create_node, ApiResponse, ResponseStatus

# Create a shallow hub and nodes
hub = create_shallow_hub()
node1 = create_node(hub, "service1")
node2 = create_node(hub, "client1")

# Register a data processing API on node1
def process_data_handler(request):
    data = request.data
    # Process the data
    result = data.upper() if isinstance(data, str) else str(data)
    return ApiResponse(
        data=result,
        metadata={"processed_by": "service1"},
        status=ResponseStatus.SUCCESS
    )

node1.register_api("/service1/process", process_data_handler)

# Call the API from node2
response = node2.call_api("/service1/process", "hello world")
print(f"Processed data: {response.data}")  # Output: Processed data: HELLO WORLD

# Set up a subscription for processed data
def data_processed_handler(message):
    print(f"Data processed notification: {message.data}")

node2.subscribe("service1/data/processed", data_processed_handler)

# Publish a notification
node1.publish("service1/data/processed", {
    "original": "hello world",
    "processed": "HELLO WORLD"
})

# Clean up
node1.disconnect()
node2.disconnect()
hub.shutdown()
```

### JavaScript Shallow Hub Example

```javascript
const { createShallowHub, createNode, ApiResponse, ResponseStatus } = require('./shallow_hub_js');

async function main() {
  // Create a shallow hub and nodes
  const hub = createShallowHub();
  const node1 = createNode(hub, 'service1');
  const node2 = createNode(hub, 'client1');
  
  // Register a data processing API on node1
  await node1.registerApi('/service1/process', (request) => {
    const data = request.data;
    // Process the data
    const result = typeof data === 'string' ? data.toUpperCase() : String(data);
    return new ApiResponse(
      result,
      { processed_by: 'service1' },
      ResponseStatus.SUCCESS
    );
  });
  
  // Call the API from node2
  const response = await node2.callApi('/service1/process', 'hello world');
  console.log(`Processed data: ${response.data}`);  // Output: Processed data: HELLO WORLD
  
  // Set up a subscription for processed data
  await node2.subscribe('service1/data/processed', (message) => {
    console.log('Data processed notification:', message.data);
  });
  
  // Publish a notification
  await node1.publish('service1/data/processed', {
    original: 'hello world',
    processed: 'HELLO WORLD'
  });
  
  // Clean up
  node1.disconnect();
  node2.disconnect();
  hub.shutdown();
}

main().catch(console.error);
```

## Advanced Usage

### Combining Network Hub and Shallow Hub

You can use both the network hub and shallow hub together to create a hybrid system:

```python
# Python example
from network_hub_python_client import create_node as create_network_node
from shallow_hub_python import create_shallow_hub, create_node as create_local_node

# Create a network node to connect to external hub
network_node = create_network_node(host="hub.example.com", port=8443)
network_node.connect()

# Create a local hub for in-process communication
local_hub = create_shallow_hub()
local_node1 = create_local_node(local_hub, "local1")
local_node2 = create_local_node(local_hub, "local2")

# Register a bridging API on network_node to forward to local_node1
def bridge_handler(request):
    # Forward the request to local_node1
    result = local_node1.call_api("/local/api", request.data)
    return result

network_node.register_api("/bridge/api", bridge_handler)
```

### Method Interception

The system supports method interception to dynamically change behavior:

```python
# Python example
from shallow_hub_python import create_shallow_hub, create_node

hub = create_shallow_hub()
node = create_node(hub)

# Original class
class Calculator:
    def add(self, a, b):
        return a + b

# Register an interceptor for the add method
def add_interceptor(message):
    if message.data["a"] < 0 or message.data["b"] < 0:
        # Handle negative numbers differently
        return abs(message.data["a"]) + abs(message.data["b"])
    return None  # Not intercepted, use original method

node.register_interceptor("calculator/add", add_interceptor)

# Use the calculator
calc = Calculator()
result = calc.add(5, -3)  # This would be intercepted
```

### Custom Serialization

For performance or special data types, you can implement custom serialization:

```python
# Python example - simplified
def custom_serialize(data):
    # Implementation depends on your data types
    return json.dumps(data, cls=CustomEncoder)

def custom_deserialize(data_str):
    # Implementation depends on your data types
    return json.loads(data_str, cls=CustomDecoder)

# Then use these functions with your API calls
```

### Hub Federation

You can federate multiple hubs for complex topologies:

```python
# Python example - simplified
# Hub 1
hub1 = create_shallow_hub()
node1 = create_node(hub1, "bridge1")

# Hub 2
hub2 = create_shallow_hub()
node2 = create_node(hub2, "bridge2")

# Set up bidirectional forwarding between hubs
def forward_to_hub2(message):
    node2.publish(message.topic, message.data, message.metadata)

node1.subscribe("hub2/*", forward_to_hub2)

def forward_to_hub1(message):
    node1.publish(message.topic, message.data, message.metadata)

node2.subscribe("hub1/*", forward_to_hub1)
```

This completes the instruction manual for the Information Network Hub system. For more detailed examples and documentation, refer to the source code and comments in each component.