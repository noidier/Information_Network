// message_system.rs - Enhanced Hub with message interception

use std::collections::{HashMap, BTreeMap};
use std::sync::{Arc, Mutex, RwLock};
use std::any::{Any, TypeId};

// Message type for typed messages
struct Message<T> {
    topic: String,
    data: T,
    metadata: HashMap<String, String>,
    sender_id: String,
    timestamp: u64,
}

// Interceptor function type
type InterceptorFn<T, R> = Arc<dyn Fn(&Message<T>) -> Option<R> + Send + Sync>;

// Type-erased message for the broker
struct AnyMessage {
    topic: String,
    data: Box<dyn Any + Send>,
    metadata: HashMap<String, String>,
    sender_id: String,
    timestamp: u64,
    type_id: TypeId,
}

struct InterceptorEntry<T, R> {
    handler: InterceptorFn<T, R>,
    priority: i32,
    id: String,
}

// Enhanced Hub with message passing and interception
struct EnhancedHub {
    // Original hub components
    registry: Registry,
    parent_hub: Option<Arc<Mutex<EnhancedHub>>>,
    
    // New message system components
    message_interceptors: RwLock<HashMap<String, BTreeMap<i32, Box<dyn Any + Send + Sync>>>>,
    method_interceptors: RwLock<HashMap<TypeId, HashMap<String, BTreeMap<i32, Box<dyn Any + Send + Sync>>>>>,
}

impl EnhancedHub {
    // Create a new hub
    fn new() -> Self {
        EnhancedHub {
            registry: Registry::new(),
            parent_hub: None,
            message_interceptors: RwLock::new(HashMap::new()),
            method_interceptors: RwLock::new(HashMap::new()),
        }
    }
    
    // Register a message interceptor for a specific topic
    fn register_interceptor<T: 'static + Send, R: 'static + Send>(
        &self,
        topic: &str,
        handler: impl Fn(&Message<T>) -> Option<R> + Send + Sync + 'static,
        priority: i32,
    ) -> String {
        let id = generate_uuid();
        let interceptor = InterceptorEntry {
            handler: Arc::new(handler),
            priority,
            id: id.clone(),
        };
        
        let mut interceptors = self.message_interceptors.write().unwrap();
        let topic_interceptors = interceptors
            .entry(topic.to_string())
            .or_insert_with(BTreeMap::new);
            
        // Use negative priority as key for reverse ordering (highest first)
        topic_interceptors.insert(-priority, Box::new(interceptor));
        
        id
    }
    
    // Register a method interceptor
    fn register_method_interceptor<T: 'static + Send, A: 'static + Send, R: 'static + Send>(
        &self,
        method_name: &str,
        handler: impl Fn(&T, &A) -> Option<R> + Send + Sync + 'static,
        priority: i32,
    ) -> String {
        let id = generate_uuid();
        let type_id = TypeId::of::<T>();
        
        let mut interceptors = self.method_interceptors.write().unwrap();
        let type_interceptors = interceptors
            .entry(type_id)
            .or_insert_with(HashMap::new);
            
        let method_interceptors = type_interceptors
            .entry(method_name.to_string())
            .or_insert_with(BTreeMap::new);
            
        // Use negative priority for reverse order (highest first)
        method_interceptors.insert(-priority, Box::new(handler));
        
        id
    }
    
    // Publish a message and allow for interception
    fn publish<T: 'static + Send, R: 'static + Send>(
        &self,
        topic: &str,
        data: T,
        metadata: HashMap<String, String>,
    ) -> Option<R> {
        let message = Message {
            topic: topic.to_string(),
            data,
            metadata,
            sender_id: "current-sender".to_string(), // In real impl, get from context
            timestamp: current_time_millis(),
        };
        
        // Check for interceptors
        if let Some(result) = self.try_intercept_message(&message) {
            return Some(result);
        }
        
        // If not intercepted and we have a parent, try there
        if let Some(parent) = &self.parent_hub {
            return parent.lock().unwrap().publish(topic, message.data, message.metadata);
        }
        
        None
    }
    
    // Try to intercept a message with registered handlers
    fn try_intercept_message<T: 'static + Send, R: 'static + Send>(
        &self,
        message: &Message<T>,
    ) -> Option<R> {
        let interceptors = self.message_interceptors.read().unwrap();
        
        // Check for exact topic match
        if let Some(topic_interceptors) = interceptors.get(&message.topic) {
            // Iterate through interceptors by priority (highest first due to negative key)
            for (_neg_priority, interceptor_box) in topic_interceptors {
                // Try to downcast to the correct interceptor type
                if let Some(interceptor) = interceptor_box.downcast_ref::<InterceptorEntry<T, R>>() {
                    // Try this interceptor
                    if let Some(result) = (interceptor.handler)(message) {
                        return Some(result);
                    }
                }
            }
        }
        
        // Check for wildcard patterns
        for (pattern, topic_interceptors) in interceptors.iter() {
            if pattern.ends_with('*') && message.topic.starts_with(&pattern[0..pattern.len()-1]) {
                for (_neg_priority, interceptor_box) in topic_interceptors {
                    if let Some(interceptor) = interceptor_box.downcast_ref::<InterceptorEntry<T, R>>() {
                        if let Some(result) = (interceptor.handler)(message) {
                            return Some(result);
                        }
                    }
                }
            }
        }
        
        None
    }
    
    // Create a method proxy that can be intercepted
    fn create_method_proxy<T: 'static + Send, A: 'static + Send, R: 'static + Send>(
        &self,
        target: Arc<T>,
        method_name: &str,
        original_method: fn(&T, A) -> R,
    ) -> impl Fn(A) -> R {
        let hub = Arc::new(self.clone());
        let method_name = method_name.to_string();
        
        move |args: A| {
            // Try to intercept the method call
            if let Some(result) = hub.try_intercept_method(&target, &method_name, &args) {
                return result;
            }
            
            // If not intercepted, call the original method
            original_method(&target, args)
        }
    }
    
    // Try to intercept a method call
    fn try_intercept_method<T: 'static + Send, A: 'static + Send, R: 'static + Send>(
        &self,
        target: &T,
        method_name: &str,
        args: &A,
    ) -> Option<R> {
        let interceptors = self.method_interceptors.read().unwrap();
        let type_id = TypeId::of::<T>();
        
        if let Some(type_interceptors) = interceptors.get(&type_id) {
            if let Some(method_interceptors) = type_interceptors.get(method_name) {
                // Try interceptors by priority (highest first)
                for (_neg_priority, handler_box) in method_interceptors {
                    if let Some(handler) = handler_box.downcast_ref::<fn(&T, &A) -> Option<R>>() {
                        if let Some(result) = handler(target, args) {
                            return Some(result);
                        }
                    }
                }
            }
        }
        
        // If not intercepted and we have a parent, try there
        if let Some(parent) = &self.parent_hub {
            return parent.lock().unwrap().try_intercept_method(target, method_name, args);
        }
        
        None
    }
}

// Helper functions
fn generate_uuid() -> String {
    // In real impl, use uuid crate
    "random-uuid".to_string()
}

fn current_time_millis() -> u64 {
    // In real impl, get system time
    0
}