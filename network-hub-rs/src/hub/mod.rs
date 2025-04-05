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
        
        // Discover and connect to parent hubs based on the scope level
        match scope {
            HubScope::Thread => {
                // Thread-level hubs look for process-level hubs in the same process
                Self::discover_and_connect_process_hub(Arc::clone(&hub));
            },
            HubScope::Process => {
                // Process-level hubs look for machine-level hubs on the same machine
                Self::discover_and_connect_machine_hub(Arc::clone(&hub));
            },
            HubScope::Machine => {
                // Machine-level hubs look for network-level hubs on the network
                Self::discover_and_connect_network_hub(Arc::clone(&hub));
            },
            HubScope::Network => {
                // Network-level hubs are the top level, so they don't need to connect to parents
            }
        }
        
        hub
    }
    
    /// Discover and connect to a process-level hub in the same process
    fn discover_and_connect_process_hub(thread_hub: Arc<Hub>) {
        // Use a static storage for process-level hubs
        use std::sync::RwLock;
        use std::collections::HashMap;
        use std::sync::atomic::{AtomicBool, Ordering};
        
        lazy_static::lazy_static! {
            static ref PROCESS_HUBS: RwLock<HashMap<String, Arc<Hub>>> = RwLock::new(HashMap::new());
            static ref DISCOVERY_RUNNING: AtomicBool = AtomicBool::new(false);
        }
        
        // Find a process-level hub to connect to
        let mut connected = false;
        
        // Look for an existing process hub
        {
            let process_hubs = PROCESS_HUBS.read().unwrap();
            for (_, process_hub) in process_hubs.iter() {
                if let Err(e) = thread_hub.connect_to_parent(Arc::clone(process_hub)) {
                    eprintln!("Error connecting to process hub: {}", e);
                } else {
                    println!("Thread hub {} connected to process hub {}", thread_hub.id, process_hub.id);
                    connected = true;
                    break;
                }
            }
        }
        
        // If no process hub found, create one
        if !connected {
            println!("No process hub found, creating one...");
            let process_hub = Arc::new(Hub::new(HubScope::Process));
            
            // Register the process hub
            {
                let mut process_hubs = PROCESS_HUBS.write().unwrap();
                process_hubs.insert(process_hub.id.clone(), Arc::clone(&process_hub));
            }
            
            // Connect the thread hub to the process hub
            if let Err(e) = thread_hub.connect_to_parent(Arc::clone(&process_hub)) {
                eprintln!("Error connecting to new process hub: {}", e);
            } else {
                println!("Thread hub {} connected to new process hub {}", thread_hub.id, process_hub.id);
            }
            
            // If we're the first process hub, start discovery of machine hubs
            if !DISCOVERY_RUNNING.swap(true, Ordering::SeqCst) {
                Self::discover_and_connect_machine_hub(Arc::clone(&process_hub));
            }
        }
    }
    
    /// Discover and connect to a machine-level hub on the same machine
    fn discover_and_connect_machine_hub(process_hub: Arc<Hub>) {
        use std::sync::RwLock;
        use std::collections::HashMap;
        use std::sync::atomic::{AtomicBool, Ordering};
        
        // Use a file-based approach to discover other processes on the same machine
        lazy_static::lazy_static! {
            static ref MACHINE_HUBS: RwLock<HashMap<String, Arc<Hub>>> = RwLock::new(HashMap::new());
            static ref MACHINE_DISCOVERY_RUNNING: AtomicBool = AtomicBool::new(false);
        }
        
        // Launch a background thread to handle machine-level hub discovery
        thread::spawn(move || {
            // Look for machine hub socket file in /tmp/network-hub/
            let hub_dir = std::path::Path::new("/tmp/network-hub");
            
            // Create the directory if it doesn't exist
            if !hub_dir.exists() {
                if let Err(e) = std::fs::create_dir_all(hub_dir) {
                    eprintln!("Error creating hub directory: {}", e);
                    return;
                }
            }
            
            // Look for an existing machine hub
            let mut connected = false;
            let machine_hub_socket = hub_dir.join("machine-hub.sock");
            
            if machine_hub_socket.exists() {
                // Try to connect to the existing machine hub
                println!("Found existing machine hub socket, connecting...");
                
                // In a real implementation, would connect to the Unix socket
                // For this example, we'll just create a new machine hub
                let machine_hub = Arc::new(Hub::new(HubScope::Machine));
                
                // Connect the process hub to the machine hub
                if let Err(e) = process_hub.connect_to_parent(Arc::clone(&machine_hub)) {
                    eprintln!("Error connecting to machine hub: {}", e);
                } else {
                    println!("Process hub {} connected to machine hub {}", process_hub.id, machine_hub.id);
                    connected = true;
                    
                    // Register the machine hub
                    {
                        let mut machine_hubs = MACHINE_HUBS.write().unwrap();
                        machine_hubs.insert(machine_hub.id.clone(), Arc::clone(&machine_hub));
                    }
                }
            }
            
            // If no machine hub found or connection failed, create one
            if !connected {
                println!("No machine hub found, creating one...");
                let machine_hub = Arc::new(Hub::new(HubScope::Machine));
                
                // Create the socket file
                let socket_file = std::fs::File::create(&machine_hub_socket);
                if let Err(e) = socket_file {
                    eprintln!("Error creating machine hub socket file: {}", e);
                } else {
                    println!("Created machine hub socket file: {:?}", machine_hub_socket);
                }
                
                // Register the machine hub
                {
                    let mut machine_hubs = MACHINE_HUBS.write().unwrap();
                    machine_hubs.insert(machine_hub.id.clone(), Arc::clone(&machine_hub));
                }
                
                // Connect the process hub to the machine hub
                if let Err(e) = process_hub.connect_to_parent(Arc::clone(&machine_hub)) {
                    eprintln!("Error connecting to new machine hub: {}", e);
                } else {
                    println!("Process hub {} connected to new machine hub {}", process_hub.id, machine_hub.id);
                }
                
                // If we're the first machine hub, start discovery of network hubs
                if !MACHINE_DISCOVERY_RUNNING.swap(true, Ordering::SeqCst) {
                    Self::discover_and_connect_network_hub(Arc::clone(&machine_hub));
                }
            }
        });
    }
    
    /// Discover and connect to a network-level hub on the network
    fn discover_and_connect_network_hub(machine_hub: Arc<Hub>) {
        // Network-level discovery is handled by the NetworkTransport
        // This will be triggered when the transport is started
        // The transport will automatically discover network hubs and connect to them
        
        println!("Starting network hub discovery for machine hub {}", machine_hub.id);
        
        // In a real implementation, we would register the machine hub with a network transport
        // For now, we'll just create a network hub and connect to it directly
        let network_hub = Arc::new(Hub::new(HubScope::Network));
        
        // Connect the machine hub to the network hub
        if let Err(e) = machine_hub.connect_to_parent(Arc::clone(&network_hub)) {
            eprintln!("Error connecting to network hub: {}", e);
        } else {
            println!("Machine hub {} connected to network hub {}", machine_hub.id, network_hub.id);
        }
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
    
    /// Register an API interceptor for a specific path
    pub fn register_api_interceptor<F>(&self, path: &str, handler: F, priority: i32) -> String
    where
        F: Fn(&ApiRequest) -> Option<ApiResponse> + Send + Sync + 'static,
    {
        self.interceptors.register_api_interceptor(path, handler, priority)
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