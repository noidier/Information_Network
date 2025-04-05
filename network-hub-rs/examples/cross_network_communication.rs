/// Example demonstrating cross-network communication between multiple hubs
/// This example shows how hubs at different scope levels discover each other
/// and communicate across network boundaries

use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::net::SocketAddr;
use std::str::FromStr;

use network_hub::{Hub, HubScope, ApiRequest, ApiResponse, ResponseStatus};
use network_hub::transport::{NetworkTransport, TlsConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Cross-Network Communication Example ===");

    // The TLS configuration (paths to cert/key files)
    let tls_config = TlsConfig {
        cert_path: "certs/cert.pem".to_string(),
        key_path: "certs/key.pem".to_string(),
        ca_path: None,
    };

    // Create a hub hierarchy
    println!("\nCreating hub hierarchy...");
    
    // Create hubs with different scope levels
    let thread_hub = Arc::new(Hub::new(HubScope::Thread));
    let process_hub = Arc::new(Hub::new(HubScope::Process));
    let machine_hub = Arc::new(Hub::new(HubScope::Machine));
    let network_hub1 = Arc::new(Hub::new(HubScope::Network));
    let network_hub2 = Arc::new(Hub::new(HubScope::Network));
    
    // Connect them in hierarchy
    thread_hub.connect_to_parent(Arc::clone(&process_hub))?;
    process_hub.connect_to_parent(Arc::clone(&machine_hub))?;
    machine_hub.connect_to_parent(Arc::clone(&network_hub1))?;
    
    println!("Hub hierarchy created:");
    println!("- Network Hub 1: {}", network_hub1.id);
    println!("  - Machine Hub: {}", machine_hub.id);
    println!("    - Process Hub: {}", process_hub.id);
    println!("      - Thread Hub: {}", thread_hub.id);
    println!("- Network Hub 2: {}", network_hub2.id);
    
    // Register APIs at different levels
    
    // Thread-level API
    thread_hub.register_api("/thread/api", |request: &ApiRequest| {
        println!("Thread API called");
        ApiResponse {
            data: Box::new(format!("Response from Thread API (hub {})", 
                                  "thread_hub")),
            metadata: HashMap::from([
                ("scope".to_string(), "thread".to_string()),
                ("timestamp".to_string(), chrono::Local::now().to_rfc3339()),
            ]),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Network-level API with timeout simulation
    network_hub1.register_api("/network/delay_api", |request: &ApiRequest| {
        println!("Network Delay API called");
        
        // Get delay parameter or default to 0ms
        let delay_ms = if let Some(delay_str) = request.metadata.get("delay_ms") {
            delay_str.parse::<u64>().unwrap_or(0)
        } else {
            0
        };
        
        // Simulate processing delay
        if delay_ms > 0 {
            println!("Simulating delay of {}ms", delay_ms);
            thread::sleep(Duration::from_millis(delay_ms));
        }
        
        ApiResponse {
            data: Box::new(format!("Response from Network Delay API after {}ms", delay_ms)),
            metadata: HashMap::from([
                ("scope".to_string(), "network".to_string()),
                ("delay_ms".to_string(), delay_ms.to_string()),
                ("timestamp".to_string(), chrono::Local::now().to_rfc3339()),
            ]),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Set up network transports
    println!("\nSetting up network transports...");
    
    // Create socket addresses for the network hubs
    let addr1 = SocketAddr::from_str("127.0.0.1:9001")?;
    let addr2 = SocketAddr::from_str("127.0.0.1:9002")?;
    
    println!("Network Hub 1 address: {}", addr1);
    println!("Network Hub 2 address: {}", addr2);
    
    // Create network transports
    let transport1 = Arc::new(NetworkTransport::new(
        Arc::clone(&network_hub1), 
        addr1, 
        tls_config.clone()
    ));
    
    let transport2 = Arc::new(NetworkTransport::new(
        Arc::clone(&network_hub2), 
        addr2, 
        tls_config.clone()
    ));
    
    // Start network transports in separate threads
    println!("\nStarting network transports...");
    
    let transport1_clone = Arc::clone(&transport1);
    let transport1_thread = thread::spawn(move || {
        println!("Network Hub 1 transport running on {}", addr1);
        transport1_clone.start().unwrap();
    });
    
    let transport2_clone = Arc::clone(&transport2);
    let transport2_thread = thread::spawn(move || {
        println!("Network Hub 2 transport running on {}", addr2);
        transport2_clone.start().unwrap();
    });
    
    // Give transports time to start
    thread::sleep(Duration::from_millis(500));
    
    // Connect the transports
    println!("\nConnecting network hubs...");
    let peer_id = transport1.connect_to_peer(addr2)?;
    println!("Connected Network Hub 1 to Network Hub 2: {}", peer_id);
    
    // Give time for connection to establish
    thread::sleep(Duration::from_millis(500));
    
    // Test 1: Call thread-level API from thread hub directly
    println!("\nTest 1: Call thread-level API from thread hub directly");
    let thread_request = ApiRequest {
        path: "/thread/api".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "example-client".to_string(),
    };
    
    let thread_response = thread_hub.handle_request(thread_request);
    
    println!("Response status: {:?}", thread_response.status);
    if let Some(data) = thread_response.data.downcast_ref::<String>() {
        println!("Response data: {}", data);
    }
    if let Some(timestamp) = thread_response.metadata.get("timestamp") {
        println!("Timestamp: {}", timestamp);
    }
    
    // Test 2: Call network-level API from thread hub (should be elevated)
    println!("\nTest 2: Call network-level API from thread hub");
    let network_request = ApiRequest {
        path: "/network/delay_api".to_string(),
        data: Box::new(()),
        metadata: HashMap::from([
            ("delay_ms".to_string(), "100".to_string()),
        ]),
        sender_id: "example-client".to_string(),
    };
    
    let network_response = thread_hub.handle_request(network_request);
    
    println!("Response status: {:?}", network_response.status);
    if let Some(data) = network_response.data.downcast_ref::<String>() {
        println!("Response data: {}", data);
    }
    if let Some(timestamp) = network_response.metadata.get("timestamp") {
        println!("Timestamp: {}", timestamp);
    }
    
    // Test 3: Call network-level API from network hub 2 (across network)
    println!("\nTest 3: Call network-level API from network hub 2 (across network)");
    
    let cross_network_request = ApiRequest {
        path: "/network/delay_api".to_string(),
        data: Box::new(()),
        metadata: HashMap::from([
            ("delay_ms".to_string(), "250".to_string()),
        ]),
        sender_id: network_hub2.id.clone(),
    };
    
    // Send the request from network hub 2 to network hub 1
    println!("Sending request from Network Hub 2 to Network Hub 1...");
    let cross_network_response = transport2.send_request_to_peer_with_timeout(
        &peer_id, 
        cross_network_request,
        Duration::from_millis(1000)
    )?;
    
    println!("Response status: {:?}", cross_network_response.status);
    if let Some(data) = cross_network_response.data.downcast_ref::<String>() {
        println!("Response data: {}", data);
    }
    if let Some(timestamp) = cross_network_response.metadata.get("timestamp") {
        println!("Timestamp: {}", timestamp);
    }
    
    // Test 4: Test timeout handling
    println!("\nTest 4: Call network-level API with timeout");
    
    let timeout_request = ApiRequest {
        path: "/network/delay_api".to_string(),
        data: Box::new(()),
        metadata: HashMap::from([
            ("delay_ms".to_string(), "500".to_string()), // 500ms delay
        ]),
        sender_id: network_hub2.id.clone(),
    };
    
    // Send the request with a timeout that's too short
    println!("Sending request with 100ms timeout for a 500ms operation...");
    match transport2.send_request_to_peer_with_timeout(
        &peer_id, 
        timeout_request,
        Duration::from_millis(100) // Only 100ms timeout
    ) {
        Ok(response) => {
            println!("Unexpected success: {:?}", response.status);
        },
        Err(e) => {
            println!("Expected timeout error: {}", e);
        }
    }
    
    // Show successful request with sufficient timeout
    println!("\nNow trying with sufficient timeout (1000ms)...");
    let sufficient_timeout_request = ApiRequest {
        path: "/network/delay_api".to_string(),
        data: Box::new(()),
        metadata: HashMap::from([
            ("delay_ms".to_string(), "500".to_string()), // 500ms delay
        ]),
        sender_id: network_hub2.id.clone(),
    };
    
    match transport2.send_request_to_peer_with_timeout(
        &peer_id, 
        sufficient_timeout_request,
        Duration::from_millis(1000) // 1000ms timeout is enough
    ) {
        Ok(response) => {
            println!("Success: {:?}", response.status);
            if let Some(data) = response.data.downcast_ref::<String>() {
                println!("Response data: {}", data);
            }
        },
        Err(e) => {
            println!("Unexpected error: {}", e);
        }
    }
    
    println!("\n=== Cross-Network Communication Example completed ===");
    
    // In a real application, would join transport threads here
    
    Ok(())
}