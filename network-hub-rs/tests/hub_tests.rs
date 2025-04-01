//! Tests for the hub core functionality

use std::collections::HashMap;
use std::sync::Arc;

use network_hub::{Hub, HubScope, ApiRequest, ApiResponse, ResponseStatus};

/// Test basic hub creation and API registration
#[test]
fn test_hub_creation_and_api_registration() {
    // Create a hub
    let hub = Hub::new(HubScope::Thread);
    
    // Assert that it has the correct scope
    assert_eq!(hub.scope, HubScope::Thread);
    
    // Verify that the hub ID is non-empty
    assert!(!hub.id.is_empty());
}

/// Test API registration and calling
#[test]
fn test_api_registration_and_calling() {
    // Create a hub
    let hub = Hub::new(HubScope::Thread);
    
    // Register an API that doesn't clone the request data
    hub.register_api("/test/echo", |request: &ApiRequest| {
        // Create new data with the same value by downcasting
        let response_data: Box<dyn std::any::Any + Send + Sync> = if let Some(s) = request.data.downcast_ref::<&str>() {
            Box::new(*s)
        } else {
            Box::new("unknown data")
        };
        
        ApiResponse {
            data: response_data,
            metadata: request.metadata.clone(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Create a request
    let request = ApiRequest {
        path: "/test/echo".to_string(),
        data: Box::new("test data"),
        metadata: HashMap::from([("test".to_string(), "metadata".to_string())]),
        sender_id: "test-client".to_string(),
    };
    
    // Call the API
    let response = hub.handle_request(request);
    
    // Verify response
    assert_eq!(response.status, ResponseStatus::Success);
    assert_eq!(response.metadata.get("test"), Some(&"metadata".to_string()));
    
    // Downcast the response data
    let data = response.data.downcast_ref::<&str>();
    assert!(data.is_some());
    assert_eq!(data, Some(&"test data"));
}

/// Test fallback paths
#[test]
fn test_api_fallback() {
    // Create a hub
    let hub = Hub::new(HubScope::Thread);
    
    // Register an API with fallback
    hub.register_api("/api/v2/resource", |_: &ApiRequest| {
        ApiResponse {
            data: Box::new("v2"),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::from([("fallback".to_string(), "/api/v1/resource".to_string())]));
    
    hub.register_api("/api/v1/resource", |_: &ApiRequest| {
        ApiResponse {
            data: Box::new("v1"),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Test direct hit on v2
    let request_v2 = ApiRequest {
        path: "/api/v2/resource".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "test-client".to_string(),
    };
    
    let response_v2 = hub.handle_request(request_v2);
    assert_eq!(response_v2.status, ResponseStatus::Success);
    assert_eq!(response_v2.data.downcast_ref::<&str>(), Some(&"v2"));
    
    // Test fallback to v1
    let request_v1 = ApiRequest {
        path: "/api/v1/resource".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "test-client".to_string(),
    };
    
    let response_v1 = hub.handle_request(request_v1);
    assert_eq!(response_v1.status, ResponseStatus::Success);
    assert_eq!(response_v1.data.downcast_ref::<&str>(), Some(&"v1"));
}

/// Test parent-child hub relationships
#[test]
fn test_parent_child_relationship() {
    // Create parent and child hubs
    let parent_hub = Arc::new(Hub::new(HubScope::Process));
    let child_hub = Hub::new(HubScope::Thread);
    
    // Connect child to parent
    let _ = child_hub.connect_to_parent(Arc::clone(&parent_hub));
    
    // Register API only on parent
    parent_hub.register_api("/parent/api", |_: &ApiRequest| {
        ApiResponse {
            data: Box::new("parent response"),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Call API from child - should cascade to parent
    let request = ApiRequest {
        path: "/parent/api".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "test-client".to_string(),
    };
    
    let response = child_hub.handle_request(request);
    
    // Verify response came from parent
    assert_eq!(response.status, ResponseStatus::Success);
    assert_eq!(response.data.downcast_ref::<&str>(), Some(&"parent response"));
}

/// Test not found response
#[test]
fn test_api_not_found() {
    // Create a hub
    let hub = Hub::new(HubScope::Thread);
    
    // Create a request for non-existent API
    let request = ApiRequest {
        path: "/non/existent/api".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "test-client".to_string(),
    };
    
    // Call the API
    let response = hub.handle_request(request);
    
    // Verify response is not found
    assert_eq!(response.status, ResponseStatus::NotFound);
}