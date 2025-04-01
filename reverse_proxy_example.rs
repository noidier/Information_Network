// reverse_proxy_example.rs - Example of using the hub as a reverse proxy

extern crate clap;

use clap::{App, Arg, SubCommand};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

// In a real implementation, would use proper paths
mod hub_core;
mod network_transport;

use hub_core::{Hub, HubScope, ApiRequest, ApiResponse, ResponseStatus};
use network_transport::{create_network_hub, create_http_reverse_proxy};

fn main() {
    let matches = App::new("Information Network Reverse Proxy")
        .version("0.1.0")
        .author("Developed with Claude")
        .about("Secure reverse proxy using the Information Network Hub")
        .arg(Arg::with_name("bind")
            .short("b")
            .long("bind")
            .help("Address to bind to")
            .takes_value(true)
            .default_value("127.0.0.1:8443"))
        .arg(Arg::with_name("cert")
            .short("c")
            .long("cert")
            .help("Path to TLS certificate")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("key")
            .short("k")
            .long("key")
            .help("Path to TLS key")
            .takes_value(true)
            .required(true))
        .subcommand(SubCommand::with_name("route")
            .about("Add a proxy route")
            .arg(Arg::with_name("path")
                .help("Path to route")
                .required(true))
            .arg(Arg::with_name("target")
                .help("Target URL")
                .required(true)))
        .get_matches();

    let bind_address = matches.value_of("bind").unwrap();
    let cert_path = matches.value_of("cert").unwrap();
    let key_path = matches.value_of("key").unwrap();

    // Create the network hub
    println!("Creating network hub and reverse proxy...");
    let (hub, transport) = create_network_hub(bind_address, cert_path, key_path);
    let proxy = create_http_reverse_proxy(Arc::clone(&hub), bind_address, cert_path, key_path);

    // Add default routes
    proxy.add_proxy_route("/", "https://example.com");
    proxy.add_proxy_route("/api", "https://api.example.com");

    // Add routes from command line if specified
    if let Some(route_matches) = matches.subcommand_matches("route") {
        let path = route_matches.value_of("path").unwrap();
        let target = route_matches.value_of("target").unwrap();
        proxy.add_proxy_route(path, target);
    }

    // Start network transport in a separate thread
    let transport_thread = thread::spawn(move || {
        transport.start().expect("Failed to start network transport");
    });

    // Start HTTP reverse proxy in the main thread
    println!("Starting reverse proxy on {}", bind_address);
    println!("Using TLS certificate: {}", cert_path);
    println!("Press Ctrl+C to exit");
    
    proxy.start().expect("Failed to start HTTP reverse proxy");

    // Wait for transport thread to finish (which it never will)
    transport_thread.join().unwrap();
}

// Example of how to register a custom API that proxies through the system
fn register_custom_api(hub: &Arc<Mutex<Hub>>) {
    hub.lock().unwrap().register_api("/api/custom", |request: &ApiRequest| {
        // This could be a proxied API that adds security or transforms data
        println!("Received request to /api/custom: {:?}", request.path);

        // In a real implementation, might forward this to another service
        let mut metadata = HashMap::new();
        metadata.insert("proxied".to_string(), "true".to_string());
        metadata.insert("secure".to_string(), "true".to_string());

        ApiResponse {
            data: Box::new("Custom API Response"),
            metadata,
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
}

// Example of how to create an API endpoint that dynamically forwards to different backends
fn register_dynamic_router(hub: &Arc<Mutex<Hub>>) {
    // This might be populated from a database or configuration file in a real implementation
    let mut route_map = HashMap::new();
    route_map.insert("users".to_string(), "https://users-api.example.com".to_string());
    route_map.insert("products".to_string(), "https://products-api.example.com".to_string());
    route_map.insert("orders".to_string(), "https://orders-api.example.com".to_string());

    hub.lock().unwrap().register_api("/router/*", move |request: &ApiRequest| {
        // Extract the path component after /router/
        let path = &request.path[8..]; // Skip "/router/"
        let segments: Vec<&str> = path.split('/').collect();
        
        if !segments.is_empty() {
            let service = segments[0];
            
            if let Some(target) = route_map.get(service) {
                println!("Routing request to {}: {}", service, target);
                
                // In a real implementation, would forward the request to the target
                let mut metadata = HashMap::new();
                metadata.insert("routed_to".to_string(), target.clone());
                
                return ApiResponse {
                    data: Box::new(format!("Routed to {}", target)),
                    metadata,
                    status: ResponseStatus::Success,
                };
            }
        }
        
        // Service not found
        ApiResponse {
            data: Box::new("Service not found"),
            metadata: HashMap::new(),
            status: ResponseStatus::NotFound,
        }
    }, HashMap::new());
}

// Example of a load balancer using the hub
fn register_load_balancer(hub: &Arc<Mutex<Hub>>) {
    // This would be a list of backend servers
    let backends = vec![
        "https://server1.example.com".to_string(),
        "https://server2.example.com".to_string(),
        "https://server3.example.com".to_string(),
    ];
    
    // Simple round-robin counter
    let counter = Arc::new(Mutex::new(0usize));
    
    hub.lock().unwrap().register_api("/balance/*", move |request: &ApiRequest| {
        let mut current = counter.lock().unwrap();
        
        // Get next backend using round-robin
        let backend = &backends[*current % backends.len()];
        *current = (*current + 1) % backends.len();
        
        println!("Load balancing to: {}", backend);
        
        // In a real implementation, would forward the request to the backend
        let mut metadata = HashMap::new();
        metadata.insert("balanced_to".to_string(), backend.clone());
        
        ApiResponse {
            data: Box::new(format!("Balanced to {}", backend)),
            metadata,
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
}