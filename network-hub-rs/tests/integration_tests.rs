//! Integration tests for the network hub system

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use std::thread;

use network_hub::{Hub, HubScope, ApiRequest, ApiResponse, ResponseStatus};

/// Test complete end-to-end API flow with message interception
#[test]
fn test_api_interception() {
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
    
    // Register an API interceptor
    hub.register_api_interceptor("/data/fetch", |request: &ApiRequest| {
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
}

/// Test multiple hubs working together
#[test]
fn test_multiple_hubs() {
    // Create multiple hubs at different scopes
    let process_hub = Arc::new(Hub::new(HubScope::Process));
    let mut thread_hub1 = Hub::new(HubScope::Thread);
    let mut thread_hub2 = Hub::new(HubScope::Thread);
    
    // Connect thread hubs to process hub
    let _ = thread_hub1.connect_to_parent(Arc::clone(&process_hub));
    let _ = thread_hub2.connect_to_parent(Arc::clone(&process_hub));
    
    // Register API on hub1
    thread_hub1.register_api("/hub1/api", |_: &ApiRequest| {
        ApiResponse {
            data: Box::new("hub1 response"),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Register API on hub2
    thread_hub2.register_api("/hub2/api", |_: &ApiRequest| {
        ApiResponse {
            data: Box::new("hub2 response"),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Register API on process hub
    process_hub.register_api("/process/api", |_: &ApiRequest| {
        ApiResponse {
            data: Box::new("process hub response"),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Call API on hub1 that's registered on hub2 (should cascade through process hub)
    let hub2_request = ApiRequest {
        path: "/hub2/api".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "test-client".to_string(),
    };
    
    let response = thread_hub1.handle_request(hub2_request);
    assert_eq!(response.status, ResponseStatus::Success);
    assert_eq!(response.data.downcast_ref::<&str>(), Some(&"hub2 response"));
    
    // Call API on hub2 that's registered on process hub
    let process_request = ApiRequest {
        path: "/process/api".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "test-client".to_string(),
    };
    
    let response = thread_hub2.handle_request(process_request);
    assert_eq!(response.status, ResponseStatus::Success);
    assert_eq!(response.data.downcast_ref::<&str>(), Some(&"process hub response"));
}

/// Test for message publishing and subscription
#[test]
fn test_message_publishing() {
    // Create a hub
    let hub = Hub::new(HubScope::Thread);
    
    // Set up a flag to verify callback was called
    let mut callback_called = false;
    
    // Subscribe to messages
    hub.subscribe("/notifications/*", |message| {
        // Verify message content
        if let Some(topic) = message.metadata.get("topic") {
            if topic == "test-topic" {
                callback_called = true;
            }
        }
        None
    }, 0);
    
    // Publish a message
    hub.publish("/notifications/test", "test data", HashMap::from([
        ("topic".to_string(), "test-topic".to_string()),
    ]));
    
    // Give the callback time to execute
    thread::sleep(Duration::from_millis(50));
    
    // Verify callback was called
    assert!(callback_called);
}