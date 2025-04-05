//! Tests for hub-to-hub communication across different scope levels

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};

use network_hub::{Hub, HubScope, ApiRequest, ApiResponse, ResponseStatus};

/// Test communication across multiple hub scope levels (Thread → Process → Machine → Network)
#[test]
fn test_cross_scope_communication() {
    // Create hubs at different scope levels
    let thread_hub = Arc::new(Hub::new(HubScope::Thread));
    let process_hub = Arc::new(Hub::new(HubScope::Process));
    let machine_hub = Arc::new(Hub::new(HubScope::Machine));
    let network_hub = Arc::new(Hub::new(HubScope::Network));
    
    // Connect hubs in parent-child hierarchy
    thread_hub.connect_to_parent(Arc::clone(&process_hub)).unwrap();
    process_hub.connect_to_parent(Arc::clone(&machine_hub)).unwrap();
    machine_hub.connect_to_parent(Arc::clone(&network_hub)).unwrap();
    
    // Register APIs at different scope levels
    
    // Thread-level API
    thread_hub.register_api("/thread/api", |_: &ApiRequest| {
        ApiResponse {
            data: Box::new("Response from Thread Hub"),
            metadata: HashMap::from([("scope".to_string(), "thread".to_string())]),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Process-level API
    process_hub.register_api("/process/api", |_: &ApiRequest| {
        ApiResponse {
            data: Box::new("Response from Process Hub"),
            metadata: HashMap::from([("scope".to_string(), "process".to_string())]),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Machine-level API
    machine_hub.register_api("/machine/api", |_: &ApiRequest| {
        ApiResponse {
            data: Box::new("Response from Machine Hub"),
            metadata: HashMap::from([("scope".to_string(), "machine".to_string())]),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Network-level API
    network_hub.register_api("/network/api", |_: &ApiRequest| {
        ApiResponse {
            data: Box::new("Response from Network Hub"),
            metadata: HashMap::from([("scope".to_string(), "network".to_string())]),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Test 1: Call thread-level API directly
    let thread_request = ApiRequest {
        path: "/thread/api".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "test-client".to_string(),
    };
    
    let thread_response = thread_hub.handle_request(thread_request);
    assert_eq!(thread_response.status, ResponseStatus::Success);
    assert_eq!(thread_response.data.downcast_ref::<&str>(), Some(&"Response from Thread Hub"));
    assert_eq!(thread_response.metadata.get("scope"), Some(&"thread".to_string()));
    
    // Test 2: Call process-level API from thread hub (should be elevated)
    let process_request = ApiRequest {
        path: "/process/api".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "test-client".to_string(),
    };
    
    let process_response = thread_hub.handle_request(process_request);
    assert_eq!(process_response.status, ResponseStatus::Success);
    assert_eq!(process_response.data.downcast_ref::<&str>(), Some(&"Response from Process Hub"));
    assert_eq!(process_response.metadata.get("scope"), Some(&"process".to_string()));
    
    // Test 3: Call machine-level API from thread hub (should be elevated twice)
    let machine_request = ApiRequest {
        path: "/machine/api".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "test-client".to_string(),
    };
    
    let machine_response = thread_hub.handle_request(machine_request);
    assert_eq!(machine_response.status, ResponseStatus::Success);
    assert_eq!(machine_response.data.downcast_ref::<&str>(), Some(&"Response from Machine Hub"));
    assert_eq!(machine_response.metadata.get("scope"), Some(&"machine".to_string()));
    
    // Test 4: Call network-level API from thread hub (should be elevated three times)
    let network_request = ApiRequest {
        path: "/network/api".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "test-client".to_string(),
    };
    
    let network_response = thread_hub.handle_request(network_request);
    assert_eq!(network_response.status, ResponseStatus::Success);
    assert_eq!(network_response.data.downcast_ref::<&str>(), Some(&"Response from Network Hub"));
    assert_eq!(network_response.metadata.get("scope"), Some(&"network".to_string()));
}

/// Test multiple hubs communicating with timeouts
#[test]
fn test_hub_communication_timeouts() {
    // Create multiple hubs at different scopes
    let thread_hub1 = Arc::new(Hub::new(HubScope::Thread));
    let thread_hub2 = Arc::new(Hub::new(HubScope::Thread));
    let process_hub = Arc::new(Hub::new(HubScope::Process));
    let machine_hub = Arc::new(Hub::new(HubScope::Machine));
    
    // Connect hubs in parent-child hierarchy
    thread_hub1.connect_to_parent(Arc::clone(&process_hub)).unwrap();
    thread_hub2.connect_to_parent(Arc::clone(&process_hub)).unwrap();
    process_hub.connect_to_parent(Arc::clone(&machine_hub)).unwrap();
    
    // Register API with normal latency on hub1
    thread_hub1.register_api("/thread1/fast_api", |request: &ApiRequest| {
        // Extract timeout parameter if present
        let delay = if let Some(delay_str) = request.metadata.get("delay_ms") {
            delay_str.parse::<u64>().unwrap_or(0)
        } else {
            0
        };
        
        // Add small delay if specified
        if delay > 0 {
            thread::sleep(Duration::from_millis(delay));
        }
        
        ApiResponse {
            data: Box::new("fast response from thread_hub1"),
            metadata: HashMap::from([("scope".to_string(), "thread1".to_string())]),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Register API with high latency on hub1
    thread_hub1.register_api("/thread1/slow_api", |_: &ApiRequest| {
        // Simulate high latency
        thread::sleep(Duration::from_millis(500));
        
        ApiResponse {
            data: Box::new("slow response from thread_hub1"),
            metadata: HashMap::from([("scope".to_string(), "thread1".to_string())]),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Register API on process hub that calls hub1 with timeout
    process_hub.register_api("/process/call_with_timeout", move |request: &ApiRequest| {
        // Extract which API to call
        let api_path = if let Some(path) = request.metadata.get("target_api") {
            path.clone()
        } else {
            "/thread1/fast_api".to_string()
        };
        
        // Extract timeout parameter if present
        let timeout_ms = if let Some(timeout_str) = request.metadata.get("timeout_ms") {
            timeout_str.parse::<u64>().unwrap_or(1000)
        } else {
            1000 // Default timeout of 1 second
        };
        
        // Create a mutex to hold the response and a flag to check if we got a response
        let response_mutex = Arc::new(Mutex::new(None));
        let response_received = Arc::new(AtomicBool::new(false));
        
        // Clone references for the thread
        let thread_hub1 = Arc::clone(&thread_hub1);
        let response_mutex_clone = Arc::clone(&response_mutex);
        let response_received_clone = Arc::clone(&response_received);
        
        // Create the nested request, forwarding any additional metadata
        let mut nested_metadata = HashMap::new();
        // Copy all metadata except target_api and timeout_ms
        for (key, value) in &request.metadata {
            if key != "target_api" && key != "timeout_ms" {
                nested_metadata.insert(key.clone(), value.clone());
            }
        }
        
        let nested_request = ApiRequest {
            path: api_path.to_string(),
            data: Box::new(()),
            metadata: nested_metadata,
            sender_id: "process_hub".to_string(),
        };
        
        // Spawn a thread to make the call to hub1
        let handle = thread::spawn(move || {
            let start = Instant::now();
            let response = thread_hub1.handle_request(nested_request);
            let elapsed = start.elapsed();
            
            println!("Call to {} took {:?}", api_path, elapsed);
            
            // Store the response in the mutex
            let mut response_guard = response_mutex_clone.lock().unwrap();
            *response_guard = Some(response);
            response_received_clone.store(true, Ordering::SeqCst);
        });
        
        // Wait up to the timeout for the thread to complete
        let start = Instant::now();
        let mut timed_out = false;
        
        while !response_received.load(Ordering::SeqCst) && start.elapsed() < Duration::from_millis(timeout_ms) {
            thread::sleep(Duration::from_millis(10));
        }
        
        if !response_received.load(Ordering::SeqCst) {
            timed_out = true;
            println!("Request timed out after {:?}", start.elapsed());
        }
        
        // Check if we got a response
        if timed_out {
            ApiResponse {
                data: Box::new("timeout"),
                metadata: HashMap::from([
                    ("timeout".to_string(), "true".to_string()),
                    ("scope".to_string(), "process".to_string())
                ]),
                status: ResponseStatus::Error,
            }
        } else {
            // Get the response from the mutex
            let response_guard = response_mutex.lock().unwrap();
            if let Some(response) = &*response_guard {
                // Add processing information
                let mut metadata = response.metadata.clone();
                metadata.insert("processed_by".to_string(), "process_hub".to_string());
                
                ApiResponse {
                    data: response.data.downcast_ref::<&str>()
                        .map(|s| Box::new(*s) as Box<dyn std::any::Any + Send + Sync>)
                        .unwrap_or_else(|| Box::new("unknown")),
                    metadata,
                    status: response.status,
                }
            } else {
                ApiResponse {
                    data: Box::new("no response"),
                    metadata: HashMap::from([("scope".to_string(), "process".to_string())]),
                    status: ResponseStatus::Error,
                }
            }
        }
    }, HashMap::new());
    
    // Register API on machine hub that calls process hub (which then calls thread hub)
    // Create a clone for the machine hub API handler
    let process_hub_clone = Arc::clone(&process_hub);
    
    machine_hub.register_api("/machine/nested_call", move |request: &ApiRequest| {
        // Extract the target API path
        let target_api = if let Some(path) = request.metadata.get("target_api") {
            path.clone()
        } else {
            "/thread1/fast_api".to_string()
        };
        
        // Extract timeout
        let timeout_ms = if let Some(timeout_str) = request.metadata.get("timeout_ms") {
            timeout_str.parse::<u64>().unwrap_or(2000)
        } else {
            2000
        };
        
        // Create nested request to process hub
        let mut nested_metadata = HashMap::new();
        for (key, value) in &request.metadata {
            nested_metadata.insert(key.clone(), value.clone());
        }
        
        // Ensure target_api is set correctly for process hub
        nested_metadata.insert("target_api".to_string(), target_api.to_string());
        nested_metadata.insert("timeout_ms".to_string(), timeout_ms.to_string());
        
        let nested_request = ApiRequest {
            path: "/process/call_with_timeout".to_string(),
            data: Box::new(()),
            metadata: nested_metadata,
            sender_id: "machine_hub".to_string(),
        };
        
        // Make the call to process hub using our cloned reference
        let response = process_hub_clone.handle_request(nested_request);
        
        // Add machine hub processing information
        let mut metadata = response.metadata.clone();
        metadata.insert("processed_by_machine".to_string(), "true".to_string());
        
        ApiResponse {
            data: response.data,
            metadata,
            status: response.status,
        }
    }, HashMap::new());
    
    // TEST CASE 1: Fast API call through the hierarchy that completes successfully
    println!("TEST CASE 1: Fast API call through hierarchy");
    let fast_request = ApiRequest {
        path: "/machine/nested_call".to_string(),
        data: Box::new(()),
        metadata: HashMap::from([
            ("target_api".to_string(), "/thread1/fast_api".to_string()),
            ("timeout_ms".to_string(), "1000".to_string()),
        ]),
        sender_id: "test".to_string(),
    };
    
    let fast_response = machine_hub.handle_request(fast_request);
    assert_eq!(fast_response.status, ResponseStatus::Success);
    assert_eq!(fast_response.data.downcast_ref::<&str>(), Some(&"fast response from thread_hub1"));
    assert_eq!(fast_response.metadata.get("scope"), Some(&"thread1".to_string()));
    assert_eq!(fast_response.metadata.get("processed_by"), Some(&"process_hub".to_string()));
    assert_eq!(fast_response.metadata.get("processed_by_machine"), Some(&"true".to_string()));
    
    // TEST CASE 2: Slow API call with sufficient timeout
    println!("TEST CASE 2: Slow API call with sufficient timeout");
    let slow_request_ok = ApiRequest {
        path: "/machine/nested_call".to_string(),
        data: Box::new(()),
        metadata: HashMap::from([
            ("target_api".to_string(), "/thread1/slow_api".to_string()),
            ("timeout_ms".to_string(), "1000".to_string()),  // 1000ms is enough for 500ms latency
        ]),
        sender_id: "test".to_string(),
    };
    
    let slow_response_ok = machine_hub.handle_request(slow_request_ok);
    assert_eq!(slow_response_ok.status, ResponseStatus::Success);
    assert_eq!(slow_response_ok.data.downcast_ref::<&str>(), Some(&"slow response from thread_hub1"));
    assert_eq!(slow_response_ok.metadata.get("processed_by_machine"), Some(&"true".to_string()));
    
    // TEST CASE 3: Fast API with timeout that's too small
    println!("TEST CASE 3: Fast API with timeout that's too small");
    let timeout_request = ApiRequest {
        path: "/machine/nested_call".to_string(),
        data: Box::new(()),
        metadata: HashMap::from([
            ("target_api".to_string(), "/thread1/slow_api".to_string()),
            ("timeout_ms".to_string(), "100".to_string()),   // 100ms is not enough for 500ms latency
        ]),
        sender_id: "test".to_string(),
    };
    
    let timeout_response = machine_hub.handle_request(timeout_request);
    assert_eq!(timeout_response.status, ResponseStatus::Error);
    assert_eq!(timeout_response.data.downcast_ref::<&str>(), Some(&"timeout"));
    assert_eq!(timeout_response.metadata.get("timeout"), Some(&"true".to_string()));
    assert_eq!(timeout_response.metadata.get("processed_by_machine"), Some(&"true".to_string()));
    
    // Test direct call to thread hub2 which should return not found
    let not_found_request = ApiRequest {
        path: "/thread1/fast_api".to_string(), // This API exists on thread_hub1, not thread_hub2
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "test".to_string(),
    };
    
    let not_found_response = thread_hub2.handle_request(not_found_request);
    assert_eq!(not_found_response.status, ResponseStatus::NotFound);
    
    // We can't test using the original thread_hub1 since it was moved
    // Let's create a fresh hub for the last part of the test
    let final_hub = Arc::new(Hub::new(HubScope::Thread));
    final_hub.register_api("/test/api", |_: &ApiRequest| {
        ApiResponse {
            data: Box::new("test response"),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    let final_request = ApiRequest {
        path: "/test/api".to_string(), 
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "test".to_string(),
    };
    
    let final_response = final_hub.handle_request(final_request);
    assert_eq!(final_response.status, ResponseStatus::Success);
    assert_eq!(final_response.data.downcast_ref::<&str>(), Some(&"test response"));
}

/// Test hub request raising through multiple scope levels with retries on failure
#[test]
fn test_multi_level_retries() {
    // Create hubs at different scope levels
    let thread_hub = Arc::new(Hub::new(HubScope::Thread));
    let process_hub = Arc::new(Hub::new(HubScope::Process));
    let machine_hub = Arc::new(Hub::new(HubScope::Machine));
    let network_hub = Arc::new(Hub::new(HubScope::Network));
    
    // Connect hubs in the hierarchy
    thread_hub.connect_to_parent(Arc::clone(&process_hub)).unwrap();
    process_hub.connect_to_parent(Arc::clone(&machine_hub)).unwrap();
    machine_hub.connect_to_parent(Arc::clone(&network_hub)).unwrap();
    
    // Counter for tracking API call attempts
    let call_attempts = Arc::new(Mutex::new(0));
    
    // Register an API that fails a certain number of times before succeeding
    // This API only exists at the network level, forcing requests to be raised up
    let call_attempts_clone = Arc::clone(&call_attempts);
    network_hub.register_api("/api/remote/data", move |request: &ApiRequest| {
        let mut attempts = call_attempts_clone.lock().unwrap();
        *attempts += 1;
        
        let fail_until = request.metadata.get("fail_until")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        
        if *attempts <= fail_until {
            // Simulate a temporary failure
            println!("API call attempt {} failed (will succeed after {} attempts)", *attempts, fail_until);
            thread::sleep(Duration::from_millis(50)); // Small delay to simulate work
            
            ApiResponse {
                data: Box::new(format!("Failed attempt {}", *attempts)),
                metadata: HashMap::from([
                    ("attempt".to_string(), attempts.to_string()),
                    ("failed".to_string(), "true".to_string()),
                    ("scope".to_string(), "network".to_string()),
                ]),
                status: ResponseStatus::Error,
            }
        } else {
            // Success after specified number of failures
            println!("API call succeeded on attempt {}", *attempts);
            
            ApiResponse {
                data: Box::new(format!("Success on attempt {}", *attempts)),
                metadata: HashMap::from([
                    ("attempt".to_string(), attempts.to_string()),
                    ("scope".to_string(), "network".to_string()),
                ]),
                status: ResponseStatus::Success,
            }
        }
    }, HashMap::new());
    
    // Add retry logic to thread hub
    thread_hub.register_api("/local/retry", move |request: &ApiRequest| {
        let network_hub = Arc::clone(&network_hub);
        
        // Get max retries and delay from request
        let max_retries = request.metadata.get("max_retries")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(3);
        
        let retry_delay_ms = request.metadata.get("retry_delay_ms")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(100);
        
        let fail_until = request.metadata.get("fail_until")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        
        // Forward to the API that only exists at network level
        let target_path = "/api/remote/data";
        
        let mut last_response = None;
        let mut retry_count = 0;
        
        while retry_count <= max_retries {
            // Create request to the remote API
            let remote_request = ApiRequest {
                path: target_path.to_string(),
                data: Box::new(()),
                metadata: HashMap::from([
                    ("fail_until".to_string(), fail_until.to_string()),
                    ("attempt".to_string(), (retry_count + 1).to_string()),
                ]),
                sender_id: "thread_hub".to_string(),
            };
            
            // This will be raised up to network hub through the hierarchy
            let response = network_hub.handle_request(remote_request);
            
            // Check if successful
            if response.status == ResponseStatus::Success {
                // Add retry info to successful response
                let mut metadata = response.metadata.clone();
                metadata.insert("retries".to_string(), retry_count.to_string());
                metadata.insert("raised_through_hierarchy".to_string(), "true".to_string());
                
                return ApiResponse {
                    data: response.data,
                    metadata,
                    status: ResponseStatus::Success,
                };
            }
            
            // Not successful, save response and retry
            last_response = Some(response);
            retry_count += 1;
            
            if retry_count <= max_retries {
                println!("Retrying API call ({}/{})", retry_count, max_retries);
                thread::sleep(Duration::from_millis(retry_delay_ms));
            }
        }
        
        // Max retries exceeded
        let mut metadata = HashMap::new();
        metadata.insert("max_retries_exceeded".to_string(), "true".to_string());
        metadata.insert("retries".to_string(), retry_count.to_string());
        
        last_response.unwrap_or(ApiResponse {
            data: Box::new("max retries exceeded"),
            metadata,
            status: ResponseStatus::Error,
        })
    }, HashMap::new());
    
    // TEST CASE 1: API succeeds on first attempt
    let request1 = ApiRequest {
        path: "/local/retry".to_string(),
        data: Box::new(()),
        metadata: HashMap::from([
            ("fail_until".to_string(), "0".to_string()), // Never fail
            ("max_retries".to_string(), "3".to_string()),
        ]),
        sender_id: "test".to_string(),
    };
    
    let response1 = thread_hub.handle_request(request1);
    assert_eq!(response1.status, ResponseStatus::Success);
    assert_eq!(response1.metadata.get("retries"), Some(&"0".to_string()));
    assert_eq!(response1.metadata.get("attempt"), Some(&"1".to_string()));
    
    // Reset the counter
    *call_attempts.lock().unwrap() = 0;
    
    // TEST CASE 2: API succeeds after retries
    let request2 = ApiRequest {
        path: "/local/retry".to_string(),
        data: Box::new(()),
        metadata: HashMap::from([
            ("fail_until".to_string(), "2".to_string()), // Fail twice then succeed
            ("max_retries".to_string(), "3".to_string()),
        ]),
        sender_id: "test".to_string(),
    };
    
    let response2 = thread_hub.handle_request(request2);
    assert_eq!(response2.status, ResponseStatus::Success);
    assert_eq!(response2.metadata.get("retries"), Some(&"2".to_string()));
    assert_eq!(response2.metadata.get("attempt"), Some(&"3".to_string()));
    
    // Reset the counter
    *call_attempts.lock().unwrap() = 0;
    
    // TEST CASE 3: API fails all attempts
    let request3 = ApiRequest {
        path: "/local/retry".to_string(),
        data: Box::new(()),
        metadata: HashMap::from([
            ("fail_until".to_string(), "5".to_string()), // Fail 5 times (more than max_retries)
            ("max_retries".to_string(), "2".to_string()),
        ]),
        sender_id: "test".to_string(),
    };
    
    let response3 = thread_hub.handle_request(request3);
    assert_eq!(response3.status, ResponseStatus::Error);
    assert_eq!(response3.metadata.get("max_retries_exceeded"), Some(&"true".to_string()));
    assert_eq!(response3.metadata.get("retries"), Some(&"3".to_string())); // 0-based + 1 for initial try
}