mod types;
mod registry;
mod interceptor;

pub use types::{
    HubScope, 
    Message, 
    ApiRequest, 
    ApiResponse, 
    ResponseStatus,
    Subscription,
    Interceptor,
};
pub use interceptor::InterceptorManager;
pub use registry::ApiRegistry;

use crate::error::{HubError, Result};
use crate::utils::{generate_uuid, current_time_millis};

use std::sync::{Arc, RwLock, Mutex};
use std::collections::HashMap;
use std::any::Any;
use dashmap::DashMap;

/// The central hub that manages routing and discovery
pub struct Hub {
    /// Unique identifier for this hub
    pub id: String,
    /// Scope level of this hub
    pub scope: HubScope,
    /// API registry for service lookup
    registry: Arc<ApiRegistry>,
    /// Parent hub connection
    parent_hub: RwLock<Option<Arc<Hub>>>,
    /// Child hub connections
    child_hubs: RwLock<Vec<Arc<Hub>>>,
    /// Message interceptors
    interceptors: Arc<InterceptorManager>,
    /// Active subscriptions
    subscriptions: Arc<DashMap<String, Vec<Subscription>>>,
}

impl Hub {
    /// Create a new hub with the specified scope
    pub fn new(scope: HubScope) -> Self {
        Hub {
            id: generate_uuid(),
            scope,
            registry: Arc::new(ApiRegistry::new()),
            parent_hub: RwLock::new(None),
            child_hubs: RwLock::new(Vec::new()),
            interceptors: Arc::new(InterceptorManager::new()),
            subscriptions: Arc::new(DashMap::new()),
        }
    }
    
    /// Initialize a hub at the appropriate scope and connect to parent hubs
    pub fn initialize(scope: HubScope) -> Arc<Self> {
        let hub = Arc::new(Hub::new(scope));
        
        // In a real implementation, would discover and connect to parent hubs
        // based on the scope level
        
        hub
    }
    
    /// Connect to a parent hub
    pub fn connect_to_parent(&self, parent: Arc<Hub>) -> Result<()> {
        if parent.scope <= self.scope {
            return Err(HubError::InvalidState(
                format!("Parent hub scope ({:?}) must be greater than child hub scope ({:?})",
                        parent.scope, self.scope)
            ));
        }
        
        // Set parent reference
        let mut parent_lock = self.parent_hub.write().unwrap();
        *parent_lock = Some(Arc::clone(&parent));
        
        // Add this hub as a child of the parent
        let mut parent_children = parent.child_hubs.write().unwrap();
        parent_children.push(Arc::new(self.clone()));
        
        Ok(())
    }
    
    /// Register an API endpoint with the hub
    pub fn register_api<F>(&self, path: &str, handler: F, metadata: HashMap<String, String>) 
    where
        F: Fn(&ApiRequest) -> ApiResponse + Send + Sync + 'static,
    {
        self.registry.register(path, handler, metadata.clone());
        
        // Propagate to parent if exists
        if let Some(_parent) = self.parent_hub.read().unwrap().as_ref() {
            let _parent_metadata = metadata.clone();
            // In a real implementation, would propagate registration info to parent
            // parent.register_remote_api(path, self.id.clone(), parent_metadata);
        }
    }
    
    /// Handle an API request with cascading search and interception
    pub fn handle_request(&self, request: ApiRequest) -> ApiResponse {
        // 1. Check for interception
        if let Some(intercepted) = self.interceptors.try_intercept_api_request(&request) {
            let mut response = intercepted;
            response.metadata.insert("intercepted".to_string(), "true".to_string());
            response.status = ResponseStatus::Intercepted;
            return response;
        }
        
        // 2. Check local registry
        if let Some(api) = self.registry.lookup(&request.path) {
            return (api.handler)(&request);
        }
        
        // 3. Escalate to parent hub
        if let Some(parent) = self.parent_hub.read().unwrap().as_ref() {
            return parent.handle_request(request);
        }
        
        // 4. Try fallback
        if let Some((fallback_path, _)) = self.registry.lookup_fallback(&request.path) {
            let mut fallback_request = ApiRequest {
                path: fallback_path.clone(),
                data: request.data,
                metadata: request.metadata.clone(),
                sender_id: request.sender_id.clone(),
            };
            fallback_request.metadata.insert("original_path".to_string(), request.path.clone());
            return self.handle_request(fallback_request);
        }
        
        // 5. Try approximation
        if let Some((similar_path, _)) = self.registry.lookup_similar(&request.path, 0.8) {
            let mut approx_request = ApiRequest {
                path: similar_path.clone(),
                data: request.data,
                metadata: request.metadata.clone(),
                sender_id: request.sender_id.clone(),
            };
            approx_request.metadata.insert("original_path".to_string(), request.path.clone());
            let mut response = self.handle_request(approx_request);
            response.metadata.insert("approximated".to_string(), "true".to_string());
            response.status = ResponseStatus::Approximated;
            return response;
        }
        
        // 6. Not found
        ApiResponse {
            data: Box::new(()),
            metadata: HashMap::new(),
            status: ResponseStatus::NotFound,
        }
    }
    
    /// Register a message interceptor for a specific topic
    pub fn register_interceptor<T, R, F>(&self, topic: &str, handler: F, priority: i32) -> String
    where
        T: 'static + Send + Sync,
        R: 'static + Send + Sync,
        F: Fn(&Message<T>) -> Option<R> + Send + Sync + 'static,
    {
        self.interceptors.register(topic, handler, priority)
    }
    
    /// Subscribe to messages matching a pattern
    pub fn subscribe<F>(&self, pattern: &str, callback: F, priority: i32) -> String
    where
        F: Fn(&Message<Box<dyn Any + Send + Sync>>) -> Option<Box<dyn Any + Send + Sync>> + Send + Sync + 'static,
    {
        let id = generate_uuid();
        let subscription = Subscription {
            id: id.clone(),
            priority,
            handler: Arc::new(Mutex::new(Box::new(callback))),
        };
        
        self.subscriptions
            .entry(pattern.to_string())
            .or_default()
            .push(subscription);
        
        // Sort subscriptions by priority (highest first)
        if let Some(mut subs) = self.subscriptions.get_mut(pattern) {
            subs.sort_by(|a, b| b.priority.cmp(&a.priority));
        }
        
        id
    }
    
    /// Publish a message with interception capability
    pub fn publish<T, R>(&self, topic: &str, data: T, metadata: HashMap<String, String>) -> Option<R>
    where
        T: 'static + Send + Sync,
        R: 'static + Send + Sync,
    {
        let message = Message {
            topic: topic.to_string(),
            data,
            metadata,
            sender_id: "current-sender".to_string(), // Would get from context
            timestamp: current_time_millis(),
        };
        
        // Try to intercept the message
        if let Some(result) = self.interceptors.try_intercept_message::<T, R>(&message) {
            return Some(result);
        }
        
        // If not intercepted and we have a parent, try there
        if let Some(_parent) = self.parent_hub.read().unwrap().as_ref() {
            // In a real implementation, would need to serialize data for transport
            // return parent.publish(topic, message.data, message.metadata);
        }
        
        None
    }
}

impl Clone for Hub {
    fn clone(&self) -> Self {
        Hub {
            id: self.id.clone(),
            scope: self.scope,
            registry: Arc::clone(&self.registry),
            parent_hub: RwLock::new(self.parent_hub.read().unwrap().clone()),
            child_hubs: RwLock::new(self.child_hubs.read().unwrap().clone()),
            interceptors: Arc::clone(&self.interceptors),
            subscriptions: Arc::clone(&self.subscriptions),
        }
    }
}