//! Tests for the HTTP reverse proxy functionality

use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use network_hub::{Hub, HubScope, HttpReverseProxy, TlsConfig, ApiRequest, ApiResponse, ResponseStatus};

/// Test proxy route configuration
#[test]
fn test_proxy_route_configuration() {
    // Create a hub
    let hub = Arc::new(Hub::new(HubScope::Network));
    
    // Create TLS config
    let tls_config = TlsConfig {
        cert_path: "certs/cert.pem".to_string(),
        key_path: "certs/key.pem".to_string(),
        ca_path: None,
    };
    
    // Create proxy on an unused port (won't actually start)
    let bind_addr = SocketAddr::from_str("127.0.0.1:0").unwrap();
    let proxy = HttpReverseProxy::new(hub, bind_addr, tls_config);
    
    // Add routes
    proxy.add_route("/api", "https://api.example.com");
    proxy.add_route("/app", "https://app.example.com");
    proxy.add_route("/static/*", "https://static.example.com");
    
    // Test if routes are registered correctly
    // This is an indirect test via the hub's API registration
    
    // Register handler for proxy register API
    let mut registered_routes: HashMap<String, String> = HashMap::new();
    
    // Simulate a request to register a route
    let request = ApiRequest {
        path: "/proxy/register".to_string(),
        data: Box::new("/test".to_string()),
        metadata: HashMap::from([("target".to_string(), "https://test.example.com".to_string())]),
        sender_id: "test-client".to_string(),
    };
    
    // Send the request through the hub
    let _response = hub.handle_request(request);
    
    // Test HTTP handler with a request
    let http_request = ApiRequest {
        path: "/http/api".to_string(),
        data: Box::new("GET /api HTTP/1.1\r\nHost: localhost\r\n\r\n"),
        metadata: HashMap::from([
            ("method".to_string(), "GET".to_string()),
            ("path".to_string(), "/api".to_string()),
        ]),
        sender_id: "test-client".to_string(),
    };
    
    // Send the request through the hub
    let response = hub.handle_request(http_request);
    
    // Verify the response contains the proxy target
    assert_eq!(response.status, ResponseStatus::Success);
    
    // Extract response data
    if let Some(body) = response.data.downcast_ref::<String>() {
        assert!(body.contains("Proxied to https://api.example.com"));
    } else {
        panic!("Response data is not a String");
    }
}