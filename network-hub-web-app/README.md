# Network Hub Web Application

A web-based interface for the network-hub-rs library that provides a clean UI for managing the hub, registering APIs, and configuring proxy routes.

## Features

- Web-based interface for managing Network Hub operations
- API registration and testing
- Hub statistics and monitoring
- Static file serving with Rust Embed

For a detailed breakdown of implemented features, see [FEATURES.md](./FEATURES.md).

## Architecture

The application consists of:

1. **Axum Web Server** - Provides HTTP endpoints and static file serving
2. **Network Hub Integration** - Connects to the network-hub-rs library
3. **Web UI** - HTML/CSS/JS interface for managing hub operations

## Running the Application

### Prerequisites

- Rust toolchain (latest stable recommended)
- network-hub-rs library (referenced locally)

### Build and Run

```bash
cargo build
cargo run
```

The server will start on http://localhost:3000

## API Endpoints

The web interface provides the following API endpoints:

- `GET /api/routes` - List all configured proxy routes
- `POST /api/routes` - Add a new proxy route
- `GET /api/routes/:path` - Get details of a specific route
- `DELETE /api/routes/:path` - Remove a proxy route
- `GET /api/apis` - List all registered API endpoints
- `POST /api/apis` - Register a new API endpoint
- `GET /api/apis/:path` - Get details of a specific API
- `DELETE /api/apis/:path` - Remove an API endpoint
- `POST /api/request` - Send a request to a registered API
- `GET /api/hub/stats` - Get hub statistics

## Limitations

- This application currently implements a subset of the full network-hub-rs functionality
- Not all API endpoints are fully implemented
- Proxy configuration is available in the UI but not connected to actual proxy functionality

## Future Improvements

- Implement full proxy functionality
- Add message publishing and subscription management
- Add real-time hub statistics and monitoring
- Add visualizations for hub hierarchy
- Add security features including authentication