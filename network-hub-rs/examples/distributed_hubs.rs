use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::net::SocketAddr;
use std::str::FromStr;

use network_hub::{Hub, HubScope, ApiRequest, ApiResponse, ResponseStatus};
use network_hub::transport::{NetworkTransport, TlsConfig};

/// Example of distributed hubs communicating across a network
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Distributed Network Hubs Example ===");
    
    // Create a simple TLS configuration
    let tls_config = TlsConfig {
        cert_path: "certs/cert.pem".to_string(),
        key_path: "certs/key.pem".to_string(),
        ca_path: None,
    };
    
    // Create a multi-level hub structure
    println!("Creating hubs at different scope levels...");
    
    // Create hubs with different scopes
    let thread_hub1 = Arc::new(Hub::new(HubScope::Thread));
    let thread_hub2 = Arc::new(Hub::new(HubScope::Thread));
    let process_hub = Arc::new(Hub::new(HubScope::Process));
    let machine_hub = Arc::new(Hub::new(HubScope::Machine));
    let network_hub = Arc::new(Hub::new(HubScope::Network));
    
    // Connect the hubs in a parent-child hierarchy
    thread_hub1.connect_to_parent(Arc::clone(&process_hub))?;
    thread_hub2.connect_to_parent(Arc::clone(&process_hub))?;
    process_hub.connect_to_parent(Arc::clone(&machine_hub))?;
    machine_hub.connect_to_parent(Arc::clone(&network_hub))?;
    
    println!("Hub hierarchy created:");
    println!("- Network Hub: {}", network_hub.id);
    println!("  - Machine Hub: {}", machine_hub.id);
    println!("    - Process Hub: {}", process_hub.id);
    println!("      - Thread Hub 1: {}", thread_hub1.id);
    println!("      - Thread Hub 2: {}", thread_hub2.id);
    
    // Register APIs at different scope levels
    println!("\nRegistering APIs at different scope levels...");
    
    // Thread-level API (local to thread)
    thread_hub1.register_api("/local/data", |request: &ApiRequest| {
        println!("Thread Hub 1 handling local data request");
        
        ApiResponse {
            data: Box::new("Thread-local data from Thread Hub 1"),
            metadata: HashMap::from([
                ("scope".to_string(), "thread".to_string()),
                ("hub_id".to_string(), "thread_hub1".to_string()),
            ]),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Process-level API (shared between threads in same process)
    process_hub.register_api("/process/data", |request: &ApiRequest| {
        println!("Process Hub handling process data request");
        
        ApiResponse {
            data: Box::new("Process-level data from Process Hub"),
            metadata: HashMap::from([
                ("scope".to_string(), "process".to_string()),
                ("hub_id".to_string(), "process_hub".to_string()),
            ]),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Machine-level API (shared between processes on same machine)
    machine_hub.register_api("/machine/data", |request: &ApiRequest| {
        println!("Machine Hub handling machine data request");
        
        ApiResponse {
            data: Box::new("Machine-level data from Machine Hub"),
            metadata: HashMap::from([
                ("scope".to_string(), "machine".to_string()),
                ("hub_id".to_string(), "machine_hub".to_string()),
            ]),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Network-level API (shared across the network)
    network_hub.register_api("/network/data", |request: &ApiRequest| {
        println!("Network Hub handling network data request");
        
        ApiResponse {
            data: Box::new("Network-level data from Network Hub"),
            metadata: HashMap::from([
                ("scope".to_string(), "network".to_string()),
                ("hub_id".to_string(), "network_hub".to_string()),
            ]),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Create a network transport for the network hub
    println!("\nSetting up network transport...");
    let addr = SocketAddr::from_str("127.0.0.1:9000")?;
    let transport = NetworkTransport::new(Arc::clone(&network_hub), addr, tls_config);
    
    // Start the transport in a separate thread
    let transport_thread = thread::spawn(move || {
        println!("Starting network transport on {}", addr);
        transport.start().unwrap();
    });
    
    // Give the transport time to start
    thread::sleep(Duration::from_millis(100));
    
    println!("\n=== Testing API Calls Across Hub Hierarchy ===");
    
    // Test 1: Call a thread-local API directly
    println!("\nTest 1: Call thread-local API directly on Thread Hub 1");
    let thread_request = ApiRequest {
        path: "/local/data".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "example-client".to_string(),
    };
    
    let thread_response = thread_hub1.handle_request(thread_request);
    
    println!("Response status: {:?}", thread_response.status);
    if let Some(data) = thread_response.data.downcast_ref::<&str>() {
        println!("Response data: {}", data);
    }
    
    // Test 2: Call a process-level API from Thread Hub 2 (should be elevated)
    println!("\nTest 2: Call process-level API from Thread Hub 2");
    let process_request = ApiRequest {
        path: "/process/data".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "example-client".to_string(),
    };
    
    let process_response = thread_hub2.handle_request(process_request);
    
    println!("Response status: {:?}", process_response.status);
    if let Some(data) = process_response.data.downcast_ref::<&str>() {
        println!("Response data: {}", data);
    }
    
    // Test 3: Call a machine-level API from Thread Hub 1 (should be elevated twice)
    println!("\nTest 3: Call machine-level API from Thread Hub 1");
    let machine_request = ApiRequest {
        path: "/machine/data".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "example-client".to_string(),
    };
    
    let machine_response = thread_hub1.handle_request(machine_request);
    
    println!("Response status: {:?}", machine_response.status);
    if let Some(data) = machine_response.data.downcast_ref::<&str>() {
        println!("Response data: {}", data);
    }
    
    // Test 4: Call a network-level API from Thread Hub 2 (should be elevated three times)
    println!("\nTest 4: Call network-level API from Thread Hub 2");
    let network_request = ApiRequest {
        path: "/network/data".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "example-client".to_string(),
    };
    
    let network_response = thread_hub2.handle_request(network_request);
    
    println!("Response status: {:?}", network_response.status);
    if let Some(data) = network_response.data.downcast_ref::<&str>() {
        println!("Response data: {}", data);
    }
    
    // Test 5: Call an API with intercept
    println!("\nTest 5: Call API with intercept");
    
    // Register the main API
    thread_hub1.register_api("/data/fetch", |_: &ApiRequest| {
        println!("Original API handler called");
        ApiResponse {
            data: Box::new("original data"),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Register interceptor
    thread_hub1.register_api_interceptor("/data/fetch", |request| {
        // Only intercept if special flag is set
        if let Some(flag) = request.metadata.get("intercept") {
            if flag == "true" {
                println!("Intercepting API call");
                return Some(ApiResponse {
                    data: Box::new("intercepted data"),
                    metadata: HashMap::from([("intercepted".to_string(), "true".to_string())]),
                    status: ResponseStatus::Intercepted,
                });
            }
        }
        println!("Not intercepting API call");
        None
    }, 10);
    
    // Make a request with intercept flag
    let intercept_request = ApiRequest {
        path: "/data/fetch".to_string(),
        data: Box::new(()),
        metadata: HashMap::from([("intercept".to_string(), "true".to_string())]),
        sender_id: "example-client".to_string(),
    };
    
    let intercept_response = thread_hub1.handle_request(intercept_request);
    
    println!("Response status: {:?}", intercept_response.status);
    if let Some(data) = intercept_response.data.downcast_ref::<&str>() {
        println!("Response data: {}", data);
    }
    println!("Intercepted: {}", intercept_response.metadata.get("intercepted").unwrap_or(&"false".to_string()));
    
    println!("\n=== Example completed successfully ===");
    
    Ok(())
}