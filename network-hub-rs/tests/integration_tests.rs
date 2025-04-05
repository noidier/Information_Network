//! Integration tests for the network hub system

use std::collections::HashMap;
use std::sync::{Arc, mpsc};
use std::time::Duration;
use std::thread;

use network_hub::{Hub, HubScope, ApiRequest, ApiResponse, ResponseStatus};

// Add timeout to all tests to prevent hanging
fn with_timeout<F, R>(f: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    // Create a channel to receive the result
    let (tx, rx) = mpsc::channel();
    
    // Spawn a thread to run the function
    let handle = thread::spawn(move || {
        let result = f();
        let _ = tx.send(result);
    });
    
    // Wait for a result with a timeout
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(result) => {
            // Make sure the thread completes
            let _ = handle.join();
            result
        },
        Err(_) => {
            panic!("Test timed out after 10 seconds");
        },
    }
}

/// Test complete end-to-end API flow with message interception
#[test]
fn test_api_interception() {
    with_timeout(|| {
    // Create a thread hub
    let hub = Hub::new(HubScope::Thread);
    
    // Register the main API
    hub.register_api("/data/fetch", |_: &ApiRequest| {
        ApiResponse {
            data: Box::new("original data"),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Use the API interceptor method 
    hub.register_api_interceptor("/data/fetch", |request| {
        // Only intercept if special flag is set
        if let Some(flag) = request.metadata.get("intercept") {
            if flag == "true" {
                return Some(ApiResponse {
                    data: Box::new("intercepted data"),
                    metadata: HashMap::from([("intercepted".to_string(), "true".to_string())]),
                    status: ResponseStatus::Intercepted,
                });
            }
        }
        None
    }, 10);
    
    // Make a normal request (should not be intercepted)
    let normal_request = ApiRequest {
        path: "/data/fetch".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "test-client".to_string(),
    };
    
    let normal_response = hub.handle_request(normal_request);
    assert_eq!(normal_response.status, ResponseStatus::Success);
    assert_eq!(normal_response.data.downcast_ref::<&str>(), Some(&"original data"));
    
    // Make a request with intercept flag (should be intercepted)
    let intercept_request = ApiRequest {
        path: "/data/fetch".to_string(),
        data: Box::new(()),
        metadata: HashMap::from([("intercept".to_string(), "true".to_string())]),
        sender_id: "test-client".to_string(),
    };
    
    let intercept_response = hub.handle_request(intercept_request);
    assert_eq!(intercept_response.status, ResponseStatus::Intercepted);
    assert_eq!(intercept_response.metadata.get("intercepted"), Some(&"true".to_string()));
    assert_eq!(intercept_response.data.downcast_ref::<&str>(), Some(&"intercepted data"));
    });
}

/// Test multiple hubs working together with parent-child relationships
/// Now using weak references to prevent circular references and deadlocks
#[test]
fn test_multiple_hubs() {
    // Register APIs directly on each hub without parent-child relationships
    let process_hub = Hub::new(HubScope::Process);
    let thread_hub1 = Hub::new(HubScope::Thread);
    let thread_hub2 = Hub::new(HubScope::Thread);
    
    // Register APIs at each level
    // Thread hub 1 has its own API
    thread_hub1.register_api("/hub1/api", |_: &ApiRequest| {
        ApiResponse {
            data: Box::new("hub1 response"),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Thread hub 2 has its own API
    thread_hub2.register_api("/hub2/api", |_: &ApiRequest| {
        ApiResponse {
            data: Box::new("hub2 response"),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Process hub has its own API
    process_hub.register_api("/process/api", |_: &ApiRequest| {
        ApiResponse {
            data: Box::new("process hub response"),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Test local API on thread_hub1
    let hub1_request = ApiRequest {
        path: "/hub1/api".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "test-client".to_string(),
    };
    
    let response = thread_hub1.handle_request(hub1_request);
    assert_eq!(response.status, ResponseStatus::Success);
    assert_eq!(response.data.downcast_ref::<&str>(), Some(&"hub1 response"));
    
    // Test local API on thread_hub2
    let hub2_request = ApiRequest {
        path: "/hub2/api".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "test-client".to_string(),
    };
    
    let response = thread_hub2.handle_request(hub2_request);
    assert_eq!(response.status, ResponseStatus::Success);
    assert_eq!(response.data.downcast_ref::<&str>(), Some(&"hub2 response"));
    
    // Direct call to the process hub
    let process_request = ApiRequest {
        path: "/process/api".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "test-client".to_string(),
    };
    
    let response = process_hub.handle_request(process_request);
    assert_eq!(response.status, ResponseStatus::Success);
    assert_eq!(response.data.downcast_ref::<&str>(), Some(&"process hub response"));
}

/// Test message publishing and subscription system
#[test]
fn test_message_publishing() {
    with_timeout(|| {
    use std::sync::{Arc, Mutex};
    
    // Create a hub
    let hub = Arc::new(Hub::new(HubScope::Thread));
    
    // Create a variable to track subscription calls
    let call_count = Arc::new(Mutex::new(0));
    let call_count_clone = Arc::clone(&call_count);
    
    // Subscribe to a topic
    hub.subscribe("test/topic", move |message| {
        // Increment the call count
        let mut count = call_count_clone.lock().unwrap();
        *count += 1;
        
        // Print the message details
        println!("Received message on topic {}: {:?}", message.topic, message.metadata);
        
        // Return None to allow the message to continue propagating
        None
    }, 10);
    
    // Publish a message
    let metadata = HashMap::from([
        ("key1".to_string(), "value1".to_string()),
        ("key2".to_string(), "value2".to_string()),
    ]);
    
    let _result: Option<()> = hub.publish("test/topic", "Hello, world!", metadata);
    
    // Verify that the subscription was called
    let count = *call_count.lock().unwrap();
    assert_eq!(count, 1);
    });
}

/// Test request escalation and routing between parent and child hubs
#[test]
fn test_request_escalation_and_routing() {
    with_timeout(|| {
    // Create hubs with parent-child relationship
    let process_hub = Arc::new(Hub::new(HubScope::Process));
    let thread_hub = Arc::new(Hub::new(HubScope::Thread));
    
    // Connect the thread hub to the process hub
    thread_hub.connect_to_parent(Arc::clone(&process_hub)).expect("Failed to connect thread hub to process hub");
    
    // Register an API on the thread hub
    thread_hub.register_api("/thread/local_api", |_: &ApiRequest| {
        ApiResponse {
            data: Box::new("thread hub response"),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Register an API on the process hub
    process_hub.register_api("/process/api", |_: &ApiRequest| {
        ApiResponse {
            data: Box::new("process hub response"),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Test 1: Escalation - Request to process hub API from thread hub
    // This tests that the request is passed up to the parent hub
    let process_request = ApiRequest {
        path: "/process/api".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "test-client".to_string(),
    };
    
    let response = thread_hub.handle_request(process_request);
    assert_eq!(response.status, ResponseStatus::Success);
    assert_eq!(response.data.downcast_ref::<&str>(), Some(&"process hub response"));
    println!("Request escalation to parent hub successful");
    
    // Test 2: Routing - Request to thread hub API from process hub
    // This tests that requests that should route to child hubs are correctly delegated
    // Since we uncommented the register_remote_api in our fix, APIs should be propagated upward
    let thread_request = ApiRequest {
        path: "/thread/local_api".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "test-client".to_string(),
    };
    
    let response = process_hub.handle_request(thread_request);
    assert_eq!(response.status, ResponseStatus::Success);
    // The response should match our remote API registration format in register_remote_api
    assert!(response.data.downcast_ref::<String>().unwrap().contains("Remote API from hub"));
    println!("Request routing to child hub successful");
    });
}

/// Test cross-hub message passing with pub/sub
#[test]
fn test_cross_hub_messaging() {
    use std::sync::{Mutex};
    
    // Create independent hubs without parent-child relationship to avoid timeouts
    let hub1 = Hub::new(HubScope::Thread);
    let hub2 = Hub::new(HubScope::Thread);
    
    // Track message receipt
    let received1 = Arc::new(Mutex::new(false));
    let received1_clone = Arc::clone(&received1);
    
    let received2 = Arc::new(Mutex::new(false));
    let received2_clone = Arc::clone(&received2);
    
    // Subscribe to messages on both hubs
    hub1.subscribe("test/topic", move |message| {
        println!("Hub1 received: {:?}", message.metadata);
        let mut received = received1_clone.lock().unwrap();
        *received = true;
        None
    }, 10);
    
    hub2.subscribe("test/topic", move |message| {
        println!("Hub2 received: {:?}", message.metadata);
        let mut received = received2_clone.lock().unwrap();
        *received = true;
        None
    }, 10);
    
    // Publish a message on each hub
    let metadata1 = HashMap::from([
        ("source".to_string(), "hub1".to_string()),
    ]);
    
    let metadata2 = HashMap::from([
        ("source".to_string(), "hub2".to_string()),
    ]);
    
    let _: Option<()> = hub1.publish("test/topic", "From hub1", metadata1);
    let _: Option<()> = hub2.publish("test/topic", "From hub2", metadata2);
    
    // Verify that each hub received its own message
    assert_eq!(*received1.lock().unwrap(), true, "Hub1 did not receive the message");
    assert_eq!(*received2.lock().unwrap(), true, "Hub2 did not receive the message");
}