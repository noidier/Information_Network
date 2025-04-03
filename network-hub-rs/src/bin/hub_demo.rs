use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use network_hub::hub::{Hub, HubScope, ApiRequest, ApiResponse, ResponseStatus};
use network_hub::transport::{NetworkTransport, TlsConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up certificates
    let certs_dir = std::env::current_dir()?.join("certs");
    let cert_path = certs_dir.join("cert.pem").to_string_lossy().to_string();
    let key_path = certs_dir.join("key.pem").to_string_lossy().to_string();
    
    let tls_config = TlsConfig {
        cert_path,
        key_path,
        ca_path: None,
    };
    
    println!("Creating two hub instances...");
    
    // Create two hubs at network scope
    let hub1 = Arc::new(Hub::new(HubScope::Network));
    let hub2 = Arc::new(Hub::new(HubScope::Network));
    
    println!("Hub 1 ID: {}", hub1.id);
    println!("Hub 2 ID: {}", hub2.id);
    
    // Set up network transports
    let bind_addr1 = "127.0.0.1:8443".parse()?;
    let bind_addr2 = "127.0.0.1:8444".parse()?;
    
    let transport1 = Arc::new(NetworkTransport::new(Arc::clone(&hub1), bind_addr1, tls_config.clone()));
    let transport2 = Arc::new(NetworkTransport::new(Arc::clone(&hub2), bind_addr2, tls_config.clone()));
    
    // Start transports in separate threads
    let t1_transport = Arc::clone(&transport1);
    let _t1 = thread::spawn(move || {
        println!("Starting Hub 1 network transport...");
        if let Err(e) = t1_transport.start() {
            eprintln!("Hub 1 transport error: {}", e);
        }
    });
    
    let t2_transport = Arc::clone(&transport2);
    let _t2 = thread::spawn(move || {
        println!("Starting Hub 2 network transport...");
        if let Err(e) = t2_transport.start() {
            eprintln!("Hub 2 transport error: {}", e);
        }
    });
    
    // Give time for the transports to start
    println!("Waiting for transports to initialize...");
    thread::sleep(Duration::from_secs(2));
    
    // Connect the hubs to each other
    println!("\nConnecting Hub 1 to Hub 2...");
    let peer2_id = match transport1.connect_to_peer(bind_addr2) {
        Ok(id) => {
            println!("Hub 1 connected to Hub 2 with peer ID: {}", id);
            id
        },
        Err(e) => {
            eprintln!("Failed to connect Hub 1 to Hub 2: {}", e);
            return Ok(());
        }
    };
    
    // Short delay to ensure connection stability
    thread::sleep(Duration::from_millis(500));
    
    println!("Connecting Hub 2 to Hub 1...");
    let peer1_id = match transport2.connect_to_peer(bind_addr1) {
        Ok(id) => {
            println!("Hub 2 connected to Hub 1 with peer ID: {}", id);
            id
        },
        Err(e) => {
            eprintln!("Failed to connect Hub 2 to Hub 1: {}", e);
            return Ok(());
        }
    };
    
    // Register APIs on each hub
    println!("\nRegistering API on Hub 1...");
    hub1.register_api("/hub1/greeting", |_| {
        ApiResponse {
            data: Box::new("Hello from Hub 1!"),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    println!("Registering API on Hub 2...");
    hub2.register_api("/hub2/greeting", |_| {
        ApiResponse {
            data: Box::new("Hello from Hub 2!"),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Give a short delay before exchanging messages
    thread::sleep(Duration::from_millis(500));
    
    // Exchange messages between hubs
    println!("\n--- Testing API Requests Between Hubs ---");
    
    println!("\nSending request from Hub 1 to Hub 2...");
    let request_to_hub2 = ApiRequest {
        path: "/hub2/greeting".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: hub1.id.clone(),
    };
    
    match transport1.send_request_to_peer(&peer2_id, request_to_hub2) {
        Ok(response) => {
            // Note: In a real implementation with proper serialization, this downcast would work
            // For this pseudocode implementation, it may not actually return data
            let response_str = response.data.downcast_ref::<&str>()
                .map(|s| *s)
                .unwrap_or("Unable to read response data");
            
            println!("Response received by Hub 1 from Hub 2: {}", response_str);
            println!("Response status: {:?}", response.status);
        },
        Err(e) => {
            eprintln!("Error sending request from Hub 1 to Hub 2: {}", e);
        }
    }
    
    thread::sleep(Duration::from_millis(500));
    
    println!("\nSending request from Hub 2 to Hub 1...");
    let request_to_hub1 = ApiRequest {
        path: "/hub1/greeting".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: hub2.id.clone(),
    };
    
    match transport2.send_request_to_peer(&peer1_id, request_to_hub1) {
        Ok(response) => {
            // Note: In a real implementation with proper serialization, this downcast would work
            // For this pseudocode implementation, it may not actually return data
            let response_str = response.data.downcast_ref::<&str>()
                .map(|s| *s)
                .unwrap_or("Unable to read response data");
            
            println!("Response received by Hub 2 from Hub 1: {}", response_str);
            println!("Response status: {:?}", response.status);
        },
        Err(e) => {
            eprintln!("Error sending request from Hub 2 to Hub 1: {}", e);
        }
    }
    
    // Test message publishing
    println!("\n--- Testing Message Publishing Between Hubs ---");
    
    println!("\nPublishing message from Hub 1 to Hub 2...");
    let metadata = HashMap::new();
    match transport1.publish_to_peer(&peer2_id, "hub1_updates", "Update from Hub 1", metadata) {
        Ok(_) => println!("Message published from Hub 1 to Hub 2"),
        Err(e) => eprintln!("Error publishing message from Hub 1 to Hub 2: {}", e),
    }
    
    thread::sleep(Duration::from_millis(500));
    
    println!("\nPublishing message from Hub 2 to Hub 1...");
    let metadata = HashMap::new();
    match transport2.publish_to_peer(&peer1_id, "hub2_updates", "Update from Hub 2", metadata) {
        Ok(_) => println!("Message published from Hub 2 to Hub 1"),
        Err(e) => eprintln!("Error publishing message from Hub 2 to Hub 1: {}", e),
    }
    
    // Summary
    println!("\n--- Hub Connection Demo Summary ---");
    println!("Hub 1 (ID: {}) and Hub 2 (ID: {}) are connected", hub1.id, hub2.id);
    println!("Each hub has registered an API endpoint and can send/receive messages");
    println!("\nNote: This demo uses pseudocode implementations for serialization and TLS.");
    println!("In a real implementation, proper serialization and secure TLS would be implemented.");
    
    // Keep the program running for a short time to observe output
    println!("\nDemo completed. Exiting in 5 seconds...");
    thread::sleep(Duration::from_secs(5));
    
    Ok(())
}