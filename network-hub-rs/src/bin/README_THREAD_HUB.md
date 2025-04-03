# Thread Hub Demo

This demo illustrates how the Information Network Hub architecture can be used to create a thread-level communication system where multiple services (virtual nodes) can communicate within the same program without using any networking.

## Key Features Demonstrated

1. **Thread-Level Hub**: Uses `HubScope::Thread` to create a hub that works within a single process
2. **Multiple Virtual Nodes**: Creates multiple services that register APIs with the hub
3. **Inter-Service Communication**: Services use other services' APIs through the hub
4. **Request Routing**: The hub routes requests between services
5. **Concurrent Access**: Multiple clients access services simultaneously

## Architecture

The demo implements the following components:

### CalculatorService

A simple service that registers basic math operations:
- `/calculator/add` - Adds two numbers
- `/calculator/subtract` - Subtracts one number from another
- `/calculator/multiply` - Multiplies two numbers

### MathService

A higher-level service that uses the CalculatorService:
- `/math/square` - Calculates the square of a number by calling the multiply API
- `/math/evaluate` - Evaluates complex expressions by making multiple calls to the calculator APIs

### Client

Represents a user of the services. Multiple clients can:
- Make requests to either service
- Compose operations by calling multiple APIs
- Execute in parallel threads

## How It Works

1. **Hub Initialization**: A central thread-level hub is created
2. **Service Registration**: Services register their APIs with the hub
3. **Request Handling**: When a service needs functionality from another service, it creates an API request and sends it through the hub
4. **Request Routing**: The hub routes the request to the appropriate API handler
5. **Response Handling**: Responses are returned through the hub back to the requester

## Running the Demo

```bash
cargo run --bin thread-hub-demo
```

## Key Takeaways

This demo showcases how the Information Network Hub architecture can be used to:

1. **Decouple Components**: Services don't need direct knowledge of each other
2. **Simplify Communication**: Hub handles all routing and message passing
3. **Enable Modularity**: New services can be added without changing existing ones
4. **Support Hierarchical Design**: Higher-level services can build on lower-level ones
5. **Provide Thread-Safe Communication**: Multiple threads can access the hub concurrently

All of this is achieved without using any networking or inter-process communication, demonstrating the hub's usefulness even in single-process applications.