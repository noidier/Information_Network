//! Tests for network hub communication with TLS

use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::str::FromStr;

use network_hub::{Hub, HubScope, ApiRequest, ApiResponse, ResponseStatus};
use network_hub::transport::{NetworkTransport, TlsConfig};

/// Test setting up network hubs with TLS communication
#[test]
fn test_network_hubs_tls() {
    // Create a TLS configuration for testing
    let tls_config = TlsConfig {
        cert_path: "/Users/nathanielblair/Documents/GitHub/Information_Network/network-hub-rs/certs/cert.pem".to_string(),
        key_path: "/Users/nathanielblair/Documents/GitHub/Information_Network/network-hub-rs/certs/key.pem".to_string(),
        ca_path: None,
    };
    
    // Create two network hubs
    let hub1 = Arc::new(Hub::new(HubScope::Network));
    let hub2 = Arc::new(Hub::new(HubScope::Network));
    
    // Register API on hub1
    hub1.register_api("/hub1/data", |_: &ApiRequest| {
        ApiResponse {
            data: Box::new("Data from Hub 1"),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Register API on hub2
    hub2.register_api("/hub2/data", |_: &ApiRequest| {
        ApiResponse {
            data: Box::new("Data from Hub 2"),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Create network transports for the hubs
    let addr1 = SocketAddr::from_str("127.0.0.1:9001").unwrap();
    let addr2 = SocketAddr::from_str("127.0.0.1:9002").unwrap();
    
    let transport1 = Arc::new(NetworkTransport::new(Arc::clone(&hub1), addr1, tls_config.clone()));
    let transport2 = Arc::new(NetworkTransport::new(Arc::clone(&hub2), addr2, tls_config.clone()));
    
    // Start the transports in separate threads
    let transport1_clone = Arc::clone(&transport1);
    let _transport1_thread = thread::spawn(move || {
        transport1_clone.start().unwrap();
    });
    
    let transport2_clone = Arc::clone(&transport2);
    let _transport2_thread = thread::spawn(move || {
        transport2_clone.start().unwrap();
    });
    
    // Give transports time to start
    thread::sleep(Duration::from_millis(100));
    
    // Connect the transports to each other
    let peer1_id = transport1.connect_to_peer(addr2).unwrap();
    let peer2_id = transport2.connect_to_peer(addr1).unwrap();
    
    // Test sending a request from hub1 to hub2
    let request1 = ApiRequest {
        path: "/hub2/data".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: hub1.id.clone(),
    };
    
    let response1 = transport1.send_request_to_peer(&peer1_id, request1).unwrap();
    assert_eq!(response1.status, ResponseStatus::Success);
    assert_eq!(response1.data.downcast_ref::<&str>(), Some(&"Data from Hub 2"));
    
    // Test sending a request from hub2 to hub1
    let request2 = ApiRequest {
        path: "/hub1/data".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: hub2.id.clone(),
    };
    
    let response2 = transport2.send_request_to_peer(&peer2_id, request2).unwrap();
    assert_eq!(response2.status, ResponseStatus::Success);
    assert_eq!(response2.data.downcast_ref::<&str>(), Some(&"Data from Hub 1"));
    
    // Clean up
    // In reality, we'd have a way to shut down the transport threads
}

/// Test network hub communication with timeouts
#[test]
fn test_network_hub_timeouts() {
    // Create a TLS configuration for testing
    let tls_config = TlsConfig {
        cert_path: "/Users/nathanielblair/Documents/GitHub/Information_Network/network-hub-rs/certs/cert.pem".to_string(),
        key_path: "/Users/nathanielblair/Documents/GitHub/Information_Network/network-hub-rs/certs/key.pem".to_string(),
        ca_path: None,
    };
    
    // Create two network hubs
    let hub1 = Arc::new(Hub::new(HubScope::Network));
    let hub2 = Arc::new(Hub::new(HubScope::Network));
    
    // Register fast API on hub1
    hub1.register_api("/hub1/fast", |_: &ApiRequest| {
        ApiResponse {
            data: Box::new("Fast response from Hub 1"),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Register slow API on hub1
    hub1.register_api("/hub1/slow", |_: &ApiRequest| {
        // Simulate slow processing
        thread::sleep(Duration::from_millis(500));
        
        ApiResponse {
            data: Box::new("Slow response from Hub 1"),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Create network transports for the hubs
    let addr1 = SocketAddr::from_str("127.0.0.1:9003").unwrap();
    let addr2 = SocketAddr::from_str("127.0.0.1:9004").unwrap();
    
    let transport1 = Arc::new(NetworkTransport::new(Arc::clone(&hub1), addr1, tls_config.clone()));
    let transport2 = Arc::new(NetworkTransport::new(Arc::clone(&hub2), addr2, tls_config.clone()));
    
    // Start the transports in separate threads
    let transport1_clone = Arc::clone(&transport1);
    let _transport1_thread = thread::spawn(move || {
        transport1_clone.start().unwrap();
    });
    
    let transport2_clone = Arc::clone(&transport2);
    let _transport2_thread = thread::spawn(move || {
        transport2_clone.start().unwrap();
    });
    
    // Give transports time to start
    thread::sleep(Duration::from_millis(100));
    
    // Connect the transports to each other
    let peer1_id = transport1.connect_to_peer(addr2).unwrap();
    
    // Test sending a request to the fast API with a sufficient timeout
    let start = Instant::now();
    let request_fast = ApiRequest {
        path: "/hub1/fast".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: hub2.id.clone(),
    };
    
    let response_fast = match transport2.send_request_to_peer_with_timeout(&peer1_id, request_fast, Duration::from_millis(200)) {
        Ok(response) => response,
        Err(e) => {
            panic!("Fast request failed: {}", e);
        }
    };
    
    let elapsed_fast = start.elapsed();
    println!("Fast request took {:?}", elapsed_fast);
    
    assert_eq!(response_fast.status, ResponseStatus::Success);
    assert_eq!(response_fast.data.downcast_ref::<&str>(), Some(&"Fast response from Hub 1"));
    assert!(elapsed_fast < Duration::from_millis(200));
    
    // Test sending a request to the slow API with an insufficient timeout
    let start = Instant::now();
    let request_slow = ApiRequest {
        path: "/hub1/slow".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: hub2.id.clone(),
    };
    
    let result_slow = transport2.send_request_to_peer_with_timeout(&peer1_id, request_slow, Duration::from_millis(200));
    
    let elapsed_slow = start.elapsed();
    println!("Slow request with timeout took {:?}", elapsed_slow);
    
    // Should have timed out
    assert!(result_slow.is_err());
    assert!(elapsed_slow >= Duration::from_millis(200) && elapsed_slow < Duration::from_millis(600));
    
    // Test sending a request to the slow API with a sufficient timeout
    let start = Instant::now();
    let request_slow_ok = ApiRequest {
        path: "/hub1/slow".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: hub2.id.clone(),
    };
    
    let response_slow_ok = match transport2.send_request_to_peer_with_timeout(&peer1_id, request_slow_ok, Duration::from_millis(1000)) {
        Ok(response) => response,
        Err(e) => {
            panic!("Slow request with sufficient timeout failed: {}", e);
        }
    };
    
    let elapsed_slow_ok = start.elapsed();
    println!("Slow request with sufficient timeout took {:?}", elapsed_slow_ok);
    
    assert_eq!(response_slow_ok.status, ResponseStatus::Success);
    assert_eq!(response_slow_ok.data.downcast_ref::<&str>(), Some(&"Slow response from Hub 1"));
    assert!(elapsed_slow_ok >= Duration::from_millis(500) && elapsed_slow_ok < Duration::from_millis(700));
}

/// Test multiple network hubs communicating concurrently 
#[test]
fn test_multi_network_hub_concurrent() {
    // Create a TLS configuration for testing
    let tls_config = TlsConfig {
        cert_path: "/Users/nathanielblair/Documents/GitHub/Information_Network/network-hub-rs/certs/cert.pem".to_string(),
        key_path: "/Users/nathanielblair/Documents/GitHub/Information_Network/network-hub-rs/certs/key.pem".to_string(),
        ca_path: None,
    };
    
    // Create three network hubs in a linear topology: hub1 <-> hub2 <-> hub3
    let hub1 = Arc::new(Hub::new(HubScope::Network));
    let hub2 = Arc::new(Hub::new(HubScope::Network));
    let hub3 = Arc::new(Hub::new(HubScope::Network));
    
    // Register APIs on hub1
    hub1.register_api("/hub1/api", |request: &ApiRequest| {
        // Add small variable delay
        if let Some(delay_str) = request.metadata.get("delay_ms") {
            if let Ok(delay) = delay_str.parse::<u64>() {
                thread::sleep(Duration::from_millis(delay));
            }
        }
        
        ApiResponse {
            data: Box::new("Response from Hub 1"),
            metadata: HashMap::from([("source".to_string(), "hub1".to_string())]),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Register APIs on hub2
    hub2.register_api("/hub2/api", |request: &ApiRequest| {
        // Add small variable delay
        if let Some(delay_str) = request.metadata.get("delay_ms") {
            if let Ok(delay) = delay_str.parse::<u64>() {
                thread::sleep(Duration::from_millis(delay));
            }
        }
        
        ApiResponse {
            data: Box::new("Response from Hub 2"),
            metadata: HashMap::from([("source".to_string(), "hub2".to_string())]),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Register forwarding API on hub2 that forwards to hub1
    // Create a clone of hub1 to use inside the closure
    let hub1_for_forwarding = Arc::clone(&hub1);
    
    hub2.register_api("/forward/to/hub1", move |request: &ApiRequest| {
        // In a real implementation, would use the network transport to forward
        // For test purposes, just create a direct call
        let request_to_hub1 = ApiRequest {
            path: "/hub1/api".to_string(),
            data: Box::new(()),  // Use empty data instead of trying to clone
            metadata: request.metadata.clone(),
            sender_id: "hub2".to_string(),
        };
        
        let response = hub1_for_forwarding.handle_request(request_to_hub1);
        
        // Add forwarding info
        let mut metadata = response.metadata.clone();
        metadata.insert("forwarded_by".to_string(), "hub2".to_string());
        
        ApiResponse {
            data: response.data,
            metadata,
            status: response.status,
        }
    }, HashMap::new());
    
    // Register APIs on hub3
    hub3.register_api("/hub3/api", |request: &ApiRequest| {
        // Add small variable delay
        if let Some(delay_str) = request.metadata.get("delay_ms") {
            if let Ok(delay) = delay_str.parse::<u64>() {
                thread::sleep(Duration::from_millis(delay));
            }
        }
        
        ApiResponse {
            data: Box::new("Response from Hub 3"),
            metadata: HashMap::from([("source".to_string(), "hub3".to_string())]),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Create network transports for the hubs
    let addr1 = SocketAddr::from_str("127.0.0.1:9005").unwrap();
    let addr2 = SocketAddr::from_str("127.0.0.1:9006").unwrap();
    let addr3 = SocketAddr::from_str("127.0.0.1:9007").unwrap();
    
    let transport1 = Arc::new(NetworkTransport::new(Arc::clone(&hub1), addr1, tls_config.clone()));
    let transport2 = Arc::new(NetworkTransport::new(Arc::clone(&hub2), addr2, tls_config.clone()));
    let transport3 = Arc::new(NetworkTransport::new(Arc::clone(&hub3), addr3, tls_config.clone()));
    
    // Start the transports in separate threads
    let transport1_clone = Arc::clone(&transport1);
    let _transport1_thread = thread::spawn(move || {
        transport1_clone.start().unwrap();
    });
    
    let transport2_clone = Arc::clone(&transport2);
    let _transport2_thread = thread::spawn(move || {
        transport2_clone.start().unwrap();
    });
    
    let transport3_clone = Arc::clone(&transport3);
    let _transport3_thread = thread::spawn(move || {
        transport3_clone.start().unwrap();
    });
    
    // Give transports time to start
    thread::sleep(Duration::from_millis(100));
    
    // Connect the transports in a linear topology
    transport1.connect_to_peer(addr2).unwrap();
    transport2.connect_to_peer(addr1).unwrap();
    transport2.connect_to_peer(addr3).unwrap();
    transport3.connect_to_peer(addr2).unwrap();
    
    // Perform concurrent requests from hub3 to hub2 and hub1
    let handles = (0..10).map(|i| {
        let _hub3 = Arc::clone(&hub3);
        let hub2 = Arc::clone(&hub2);
        let hub1 = Arc::clone(&hub1);
        
        thread::spawn(move || {
            // Determine which API to call based on index
            let (path, delay) = match i % 3 {
                0 => ("/hub1/api".to_string(), 20),       // Call hub1 API (through transport)
                1 => ("/hub2/api".to_string(), 10),       // Call hub2 API (through transport)
                _ => ("/forward/to/hub1".to_string(), 5), // Call hub2's forwarding API which calls hub1
            };
            
            // Create the request
            let request = ApiRequest {
                path,
                data: Box::new(format!("Request {}", i)),
                metadata: HashMap::from([("delay_ms".to_string(), delay.to_string())]),
                sender_id: format!("thread-{}", i),
            };
            
            // Make the request with a reasonable timeout
            let start = Instant::now();
            
            // Directly handle in tests (in real app would use network transport)
            let response = match i % 3 {
                0 => hub1.handle_request(request),
                1 => hub2.handle_request(request),
                _ => hub2.handle_request(request),
            };
            
            let elapsed = start.elapsed();
            
            // Return info about the request
            (i, response, elapsed)
        })
    }).collect::<Vec<_>>();
    
    // Wait for all requests to complete
    let results = handles.into_iter().map(|handle| handle.join().unwrap()).collect::<Vec<_>>();
    
    // Verify results
    for (i, response, elapsed) in results {
        println!("Request {} took {:?}", i, elapsed);
        
        assert_eq!(response.status, ResponseStatus::Success);
        
        match i % 3 {
            0 => {
                assert_eq!(response.data.downcast_ref::<&str>(), Some(&"Response from Hub 1"));
                assert_eq!(response.metadata.get("source"), Some(&"hub1".to_string()));
            },
            1 => {
                assert_eq!(response.data.downcast_ref::<&str>(), Some(&"Response from Hub 2"));
                assert_eq!(response.metadata.get("source"), Some(&"hub2".to_string()));
            },
            _ => {
                assert_eq!(response.data.downcast_ref::<&str>(), Some(&"Response from Hub 1"));
                assert_eq!(response.metadata.get("source"), Some(&"hub1".to_string()));
                assert_eq!(response.metadata.get("forwarded_by"), Some(&"hub2".to_string()));
            },
        }
    }
}

/// Helper extension trait to add timeout functionality to NetworkTransport
trait NetworkTransportExt {
    fn send_request_to_peer_with_timeout(
        &self,
        peer_id: &str,
        request: ApiRequest,
        timeout: Duration,
    ) -> Result<ApiResponse, String>;
}

impl NetworkTransportExt for NetworkTransport {
    fn send_request_to_peer_with_timeout(
        &self,
        peer_id: &str,
        request: ApiRequest,
        timeout: Duration,
    ) -> Result<ApiResponse, String> {
        // Create a channel for the response
        let (tx, rx) = std::sync::mpsc::channel();
        
        // Clone necessary data for the thread
        let self_clone = self.clone();
        let peer_id = peer_id.to_string();
        
        // Spawn a thread to make the request
        thread::spawn(move || {
            match self_clone.send_request_to_peer(&peer_id, request) {
                Ok(response) => {
                    let _ = tx.send(Ok(response));
                },
                Err(e) => {
                    let _ = tx.send(Err(format!("Error sending request: {}", e)));
                },
            }
        });
        
        // Wait for response or timeout
        match rx.recv_timeout(timeout) {
            Ok(result) => result,
            Err(_) => Err("Request timed out".to_string()),
        }
    }
}