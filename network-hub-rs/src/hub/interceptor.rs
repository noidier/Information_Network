use std::any::{Any, TypeId};
use std::collections::{HashMap, BTreeMap};
use std::sync::{Arc, RwLock};

use crate::utils::generate_uuid;
use crate::hub::types::{Message, ApiRequest, ApiResponse, Interceptor};

/// Manager for message and API interceptors
pub struct InterceptorManager {
    /// Message interceptors by topic
    message_interceptors: RwLock<HashMap<String, BTreeMap<i32, Box<dyn Any + Send + Sync>>>>,
    /// Method interceptors by type ID and method name
    method_interceptors: RwLock<HashMap<TypeId, HashMap<String, BTreeMap<i32, Box<dyn Any + Send + Sync>>>>>,
    /// API interceptors by path
    api_interceptors: RwLock<HashMap<String, BTreeMap<i32, Box<dyn Fn(&ApiRequest) -> Option<ApiResponse> + Send + Sync>>>>,
}

impl InterceptorManager {
    /// Create a new interceptor manager
    pub fn new() -> Self {
        InterceptorManager {
            message_interceptors: RwLock::new(HashMap::new()),
            method_interceptors: RwLock::new(HashMap::new()),
            api_interceptors: RwLock::new(HashMap::new()),
        }
    }
    
    /// Register a message interceptor
    pub fn register<T, R, F>(&self, topic: &str, handler: F, priority: i32) -> String
    where
        T: 'static + Send + Sync,
        R: 'static + Send + Sync,
        F: Fn(&Message<T>) -> Option<R> + Send + Sync + 'static,
    {
        let id = generate_uuid();
        
        let interceptor = Interceptor {
            id: id.clone(),
            priority,
            handler: Arc::new(handler),
        };
        
        let mut interceptors = self.message_interceptors.write().unwrap();
        let topic_interceptors = interceptors
            .entry(topic.to_string())
            .or_insert_with(BTreeMap::new);
        
        // Use negative priority for reverse ordering (highest first)
        topic_interceptors.insert(-priority, Box::new(interceptor));
        
        id
    }
    
    /// Register an API interceptor
    pub fn register_api_interceptor<F>(&self, path: &str, handler: F, priority: i32) -> String
    where
        F: Fn(&ApiRequest) -> Option<ApiResponse> + Send + Sync + 'static,
    {
        let id = generate_uuid();
        
        let mut interceptors = self.api_interceptors.write().unwrap();
        let path_interceptors = interceptors
            .entry(path.to_string())
            .or_insert_with(BTreeMap::new);
        
        // Use negative priority for reverse ordering (highest first)
        path_interceptors.insert(-priority, Box::new(handler));
        
        id
    }
    
    /// Try to intercept an API request
    pub fn try_intercept_api_request(&self, request: &ApiRequest) -> Option<ApiResponse> {
        let interceptors = self.api_interceptors.read().unwrap();
        
        // Check for exact path match
        if let Some(path_interceptors) = interceptors.get(&request.path) {
            for (_neg_priority, handler) in path_interceptors.iter() {
                if let Some(response) = handler(request) {
                    return Some(response);
                }
            }
        }
        
        // Check for wildcard patterns
        for (pattern, path_interceptors) in interceptors.iter() {
            if pattern.ends_with('*') && request.path.starts_with(&pattern[0..pattern.len()-1]) {
                for (_neg_priority, handler) in path_interceptors.iter() {
                    if let Some(response) = handler(request) {
                        return Some(response);
                    }
                }
            }
        }
        
        None
    }
    
    /// Try to intercept a message
    pub fn try_intercept_message<T, R>(&self, message: &Message<T>) -> Option<R>
    where
        T: 'static + Send + Sync,
        R: 'static + Send + Sync,
    {
        let interceptors = self.message_interceptors.read().unwrap();
        
        // Check for exact topic match
        if let Some(topic_interceptors) = interceptors.get(&message.topic) {
            for (_neg_priority, interceptor_box) in topic_interceptors.iter() {
                // We need to cast based on our message wrapper and expected response type
                let interceptor_ref = interceptor_box.downcast_ref::<Interceptor<Message<T>, R>>();
                if let Some(interceptor) = interceptor_ref {
                    if let Some(result) = (interceptor.handler)(message) {
                        return Some(result);
                    }
                }
            }
        }
        
        // Check for wildcard patterns
        for (pattern, topic_interceptors) in interceptors.iter() {
            if pattern.ends_with('*') && message.topic.starts_with(&pattern[0..pattern.len()-1]) {
                for (_neg_priority, interceptor_box) in topic_interceptors.iter() {
                    let interceptor_ref = interceptor_box.downcast_ref::<Interceptor<Message<T>, R>>();
                    if let Some(interceptor) = interceptor_ref {
                        if let Some(result) = (interceptor.handler)(message) {
                            return Some(result);
                        }
                    }
                }
            }
        }
        
        None
    }
    
    /// Register a method interceptor
    pub fn register_method_interceptor<T, A, R, F>(
        &self,
        class_type: TypeId,
        method_name: &str,
        handler: F,
        priority: i32,
    ) -> String
    where
        T: 'static + Send + Sync,
        A: 'static + Send + Sync,
        R: 'static + Send + Sync,
        F: Fn(&T, &A) -> Option<R> + Send + Sync + 'static,
    {
        let id = generate_uuid();
        
        let mut interceptors = self.method_interceptors.write().unwrap();
        let type_interceptors = interceptors
            .entry(class_type)
            .or_insert_with(HashMap::new);
        
        let method_interceptors = type_interceptors
            .entry(method_name.to_string())
            .or_insert_with(BTreeMap::new);
        
        // Use negative priority for reverse ordering (highest first)
        method_interceptors.insert(-priority, Box::new(handler));
        
        id
    }
    
    /// Try to intercept a method call
    pub fn try_intercept_method<T, A, R>(
        &self,
        target: &T,
        method_name: &str,
        args: &A,
    ) -> Option<R>
    where
        T: 'static + Send + Sync,
        A: 'static + Send + Sync,
        R: 'static + Send + Sync,
    {
        let interceptors = self.method_interceptors.read().unwrap();
        let type_id = TypeId::of::<T>();
        
        if let Some(type_interceptors) = interceptors.get(&type_id) {
            if let Some(method_interceptors) = type_interceptors.get(method_name) {
                for (_neg_priority, handler_box) in method_interceptors.iter() {
                    // In real code, we'd need a better way to handle this casting
                    // This is a placeholder - would need proper trait objects and dynamic dispatch
                    if let Some(handler) = handler_box.downcast_ref::<Box<dyn Fn(&T, &A) -> Option<R> + Send + Sync>>() {
                        if let Some(result) = handler(target, args) {
                            return Some(result);
                        }
                    }
                }
            }
        }
        
        None
    }
}