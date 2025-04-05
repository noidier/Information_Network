use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};

use network_hub::{Hub, HubScope, ApiRequest, ApiResponse, ResponseStatus};

/// Example demonstrating communication between multiple hubs with timeouts
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Hub Communication with Timeouts Example ===");
    
    // Create hubs
    let service_hub = Arc::new(Hub::new(HubScope::Thread));
    let client_hub = Arc::new(Hub::new(HubScope::Thread));
    
    println!("Created service hub with ID: {}", service_hub.id);
    println!("Created client hub with ID: {}", client_hub.id);
    
    // Register API with variable response times on service hub
    service_hub.register_api("/service/variable_latency", |request: &ApiRequest| {
        // Extract latency parameter
        let latency_ms = if let Some(latency_str) = request.metadata.get("latency_ms") {
            latency_str.parse::<u64>().unwrap_or(0)
        } else {
            0
        };
        
        println!("Service handling request with latency: {}ms", latency_ms);
        
        // Simulate processing time
        if latency_ms > 0 {
            thread::sleep(Duration::from_millis(latency_ms));
        }
        
        // Return response with latency info
        ApiResponse {
            data: Box::new(format!("Service response after {}ms", latency_ms)),
            metadata: HashMap::from([
                ("latency_ms".to_string(), latency_ms.to_string()),
                ("processed_at".to_string(), chrono::Local::now().to_rfc3339()),
            ]),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Register API on client hub that calls service with timeout
    client_hub.register_api("/client/call_with_timeout", {
        let service_hub = Arc::clone(&service_hub);
        
        move |request: &ApiRequest| {
            // Get timeout from request metadata
            let timeout_ms = if let Some(timeout_str) = request.metadata.get("timeout_ms") {
                timeout_str.parse::<u64>().unwrap_or(1000)
            } else {
                1000 // Default 1s timeout
            };
            
            // Get latency parameter to pass to service
            let latency_ms = if let Some(latency_str) = request.metadata.get("latency_ms") {
                latency_str.parse::<u64>().unwrap_or(0)
            } else {
                0
            };
            
            println!("Client making request to service with timeout: {}ms and latency: {}ms", 
                     timeout_ms, latency_ms);
            
            // Prepare to track response
            let response_mutex = Arc::new(Mutex::new(None));
            let response_received = Arc::new(AtomicBool::new(false));
            
            // Clone for thread
            let response_mutex_clone = Arc::clone(&response_mutex);
            let response_received_clone = Arc::clone(&response_received);
            let service_hub_clone = Arc::clone(&service_hub);
            
            // Create request to service
            let service_request = ApiRequest {
                path: "/service/variable_latency".to_string(),
                data: Box::new(()),
                metadata: HashMap::from([
                    ("latency_ms".to_string(), latency_ms.to_string()),
                    ("request_id".to_string(), uuid::Uuid::new_v4().to_string()),
                ]),
                sender_id: "client_hub".to_string(),
            };
            
            // Spawn thread to make the call with timeout
            let start = Instant::now();
            thread::spawn(move || {
                let response = service_hub_clone.handle_request(service_request);
                
                // Store the response
                let mut response_guard = response_mutex_clone.lock().unwrap();
                *response_guard = Some(response);
                response_received_clone.store(true, Ordering::SeqCst);
            });
            
            // Wait for response or timeout
            while !response_received.load(Ordering::SeqCst) && start.elapsed() < Duration::from_millis(timeout_ms) {
                thread::sleep(Duration::from_millis(10));
            }
            
            // Check if we got a response or timed out
            if !response_received.load(Ordering::SeqCst) {
                println!("Request timed out after {}ms", start.elapsed().as_millis());
                
                // Timeout response
                ApiResponse {
                    data: Box::new("Timeout"),
                    metadata: HashMap::from([
                        ("timeout".to_string(), "true".to_string()),
                        ("elapsed_ms".to_string(), start.elapsed().as_millis().to_string()),
                        ("timeout_ms".to_string(), timeout_ms.to_string()),
                    ]),
                    status: ResponseStatus::Error,
                }
            } else {
                // Got a response
                let elapsed = start.elapsed();
                println!("Request completed in {}ms", elapsed.as_millis());
                
                let response_guard = response_mutex.lock().unwrap();
                let service_response = response_guard.as_ref().unwrap();
                
                // Add client metadata
                let mut metadata = service_response.metadata.clone();
                metadata.insert("client_elapsed_ms".to_string(), elapsed.as_millis().to_string());
                
                // Return the response
                ApiResponse {
                    data: service_response.data.downcast_ref::<String>()
                        .map(|s| Box::new(s.clone()) as Box<dyn std::any::Any + Send + Sync>)
                        .unwrap_or_else(|| Box::new("Unknown response type")),
                    metadata,
                    status: service_response.status,
                }
            }
        }
    }, HashMap::new());
    
    println!("\n=== Testing Timeout Scenarios ===");
    
    // Test 1: Fast response (should complete successfully)
    println!("\nTest 1: Fast response (should succeed)");
    let fast_request = ApiRequest {
        path: "/client/call_with_timeout".to_string(),
        data: Box::new(()),
        metadata: HashMap::from([
            ("timeout_ms".to_string(), "1000".to_string()),  // 1s timeout
            ("latency_ms".to_string(), "100".to_string()),   // 100ms latency (should complete in time)
        ]),
        sender_id: "main".to_string(),
    };
    
    let fast_response = client_hub.handle_request(fast_request);
    println!("Response status: {:?}", fast_response.status);
    if let Some(data) = fast_response.data.downcast_ref::<String>() {
        println!("Response data: {}", data);
    }
    println!("Client elapsed: {}ms", fast_response.metadata.get("client_elapsed_ms").unwrap_or(&"unknown".to_string()));
    
    // Test 2: Slow response (should timeout)
    println!("\nTest 2: Slow response (should timeout)");
    let slow_request = ApiRequest {
        path: "/client/call_with_timeout".to_string(),
        data: Box::new(()),
        metadata: HashMap::from([
            ("timeout_ms".to_string(), "200".to_string()),   // 200ms timeout
            ("latency_ms".to_string(), "500".to_string()),   // 500ms latency (should timeout)
        ]),
        sender_id: "main".to_string(),
    };
    
    let slow_response = client_hub.handle_request(slow_request);
    println!("Response status: {:?}", slow_response.status);
    if let Some(data) = slow_response.data.downcast_ref::<&str>() {
        println!("Response data: {}", data);
    }
    if let Some(timeout) = slow_response.metadata.get("timeout") {
        println!("Timeout: {}", timeout);
        println!("Elapsed before timeout: {}ms", slow_response.metadata.get("elapsed_ms").unwrap_or(&"unknown".to_string()));
    }
    
    // Test 3: Edge case (timeout == latency)
    println!("\nTest 3: Edge case (timeout == latency)");
    let edge_request = ApiRequest {
        path: "/client/call_with_timeout".to_string(),
        data: Box::new(()),
        metadata: HashMap::from([
            ("timeout_ms".to_string(), "300".to_string()),   // 300ms timeout
            ("latency_ms".to_string(), "300".to_string()),   // 300ms latency (race condition)
        ]),
        sender_id: "main".to_string(),
    };
    
    let edge_response = client_hub.handle_request(edge_request);
    println!("Response status: {:?}", edge_response.status);
    
    if edge_response.status == ResponseStatus::Error {
        if let Some(timeout) = edge_response.metadata.get("timeout") {
            println!("Timeout: {}", timeout);
            println!("Elapsed before timeout: {}ms", edge_response.metadata.get("elapsed_ms").unwrap_or(&"unknown".to_string()));
        }
    } else {
        if let Some(data) = edge_response.data.downcast_ref::<String>() {
            println!("Response data: {}", data);
        }
        println!("Client elapsed: {}ms", edge_response.metadata.get("client_elapsed_ms").unwrap_or(&"unknown".to_string()));
    }
    
    println!("\n=== Timeout Handling Example Completed ===");
    
    Ok(())
}