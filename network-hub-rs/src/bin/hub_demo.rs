use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use network_hub::hub::{Hub, HubScope, ResponseStatus};
use network_hub::transport::{TlsConfig};

/// Simple standalone demo for connecting two hubs within a single process
/// This demonstrates the hub architecture without relying on network serialization
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Simplified Hub Connection Demo");
    println!("======================================");
    
    // Create two hubs at the Thread scope level
    let hub1 = Arc::new(Hub::new(HubScope::Thread));
    let hub2 = Arc::new(Hub::new(HubScope::Thread));
    
    println!("Created two hubs:");
    println!("- Hub 1 ID: {}", hub1.id);
    println!("- Hub 2 ID: {}", hub2.id);
    
    // Register APIs on hub1
    println!("\nRegistering APIs on Hub 1...");
    hub1.register_api("/hub1/greeting", |_| {
        println!("Hub 1's greeting API was called");
        let message = "Hello from Hub 1!";
        println!("Responding with: {}", message);
        
        network_hub::hub::ApiResponse {
            data: Box::new(message),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    hub1.register_api("/hub1/echo", |request| {
        println!("Hub 1's echo API was called");
        
        // Try to downcast the request data to a string
        let echo_data = if let Some(s) = request.data.downcast_ref::<&str>() {
            *s
        } else if let Some(s) = request.data.downcast_ref::<String>() {
            s
        } else {
            "Unknown data format"
        };
        
        println!("Echoing: {}", echo_data);
        
        network_hub::hub::ApiResponse {
            data: Box::new(format!("Hub 1 echoes: {}", echo_data)),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    // Register APIs on hub2
    println!("\nRegistering APIs on Hub 2...");
    hub2.register_api("/hub2/greeting", |_| {
        println!("Hub 2's greeting API was called");
        let message = "Hello from Hub 2!";
        println!("Responding with: {}", message);
        
        network_hub::hub::ApiResponse {
            data: Box::new(message),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    hub2.register_api("/hub2/time", |_| {
        println!("Hub 2's time API was called");
        let current_time = chrono::Local::now().format("%H:%M:%S").to_string();
        println!("Current time is: {}", current_time);
        
        network_hub::hub::ApiResponse {
            data: Box::new(format!("Current time is: {}", current_time)),
            metadata: HashMap::new(),
            status: ResponseStatus::Success,
        }
    }, HashMap::new());
    
    println!("\n--- Testing Direct API Calls ---");
    
    // Create requests directly
    println!("\nSending request to Hub 1's greeting API...");
    let request1 = network_hub::hub::ApiRequest {
        path: "/hub1/greeting".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "test-client".to_string(),
    };
    
    // Call the API and get the response
    let response1 = hub1.handle_request(request1);
    
    // Extract the response data
    if let Some(message) = response1.data.downcast_ref::<&str>() {
        println!("Response from Hub 1: {}", message);
    } else if let Some(message) = response1.data.downcast_ref::<String>() {
        println!("Response from Hub 1: {}", message);
    } else {
        println!("Received response from Hub 1 with unknown data format");
    }
    
    println!("\nSending request to Hub 2's time API...");
    let request2 = network_hub::hub::ApiRequest {
        path: "/hub2/time".to_string(),
        data: Box::new(()),
        metadata: HashMap::new(),
        sender_id: "test-client".to_string(),
    };
    
    // Call the API and get the response
    let response2 = hub2.handle_request(request2);
    
    // Extract the response data
    if let Some(message) = response2.data.downcast_ref::<String>() {
        println!("Response from Hub 2: {}", message);
    } else {
        println!("Received response from Hub 2 with unknown data format");
    }
    
    println!("\n--- Testing Cross-Hub Communication ---");
    println!("\nSimulating hub-to-hub communication:");
    
    // Create a client that uses hub1 to communicate with hub2
    let client = ClientUsingHub1 {
        hub1: Arc::clone(&hub1),
        hub2: Arc::clone(&hub2), // In a real network setup, hub1 wouldn't have direct reference to hub2
    };
    
    // Start the client in a separate thread
    client.run_in_thread();
    
    // Allow time for the client to complete its operations
    thread::sleep(Duration::from_secs(2));
    
    println!("\n--- Hub Connection Demo Summary ---");
    println!("- Demonstrated cross-hub API calls");
    println!("- Each hub can host multiple APIs with different functionalities");
    println!("- Clients can use one hub to communicate with other hubs");
    
    println!("\nDemo completed.");
    
    Ok(())
}

// A client that uses hub1 to communicate with hub2
struct ClientUsingHub1 {
    hub1: Arc<Hub>,
    hub2: Arc<Hub>, // In a real network setup, this would be accessed through hub1
}

impl ClientUsingHub1 {
    fn run_in_thread(self) {
        thread::spawn(move || {
            self.run_operations();
        });
    }
    
    fn run_operations(&self) {
        println!("Client started - will use Hub 1 to call APIs on Hub 2");
        
        // In a real network application, we would not have direct access to hub2
        // Instead, we'd send the request through hub1's network transport
        // For demonstration, we're directly sending to hub2 to simulate
        // what would happen when the network serialization works
        
        // Call hub2's greeting API
        let request1 = network_hub::hub::ApiRequest {
            path: "/hub2/greeting".to_string(),
            data: Box::new(()),
            metadata: HashMap::new(),
            sender_id: self.hub1.id.clone(), // We're sending on behalf of hub1
        };
        
        println!("\nClient sending request from Hub 1 to Hub 2's greeting API...");
        let response1 = self.hub2.handle_request(request1);
        
        // Extract and display the response
        if let Some(message) = response1.data.downcast_ref::<&str>() {
            println!("Client received response via Hub 1 from Hub 2: {}", message);
        } else if let Some(message) = response1.data.downcast_ref::<String>() {
            println!("Client received response via Hub 1 from Hub 2: {}", message);
        } else {
            println!("Client received response from Hub 2 with unknown data format");
        }
        
        // Wait a bit between requests
        thread::sleep(Duration::from_millis(500));
        
        // Call hub1's echo API with data
        let echo_data = "This is a message from Hub 2 to Hub 1";
        println!("\nClient sending request from Hub 2 to Hub 1's echo API...");
        println!("Data being sent: '{}'", echo_data);
        
        let request2 = network_hub::hub::ApiRequest {
            path: "/hub1/echo".to_string(),
            data: Box::new(echo_data),
            metadata: HashMap::new(),
            sender_id: self.hub2.id.clone(), // We're sending on behalf of hub2
        };
        
        let response2 = self.hub1.handle_request(request2);
        
        // Extract and display the response
        if let Some(message) = response2.data.downcast_ref::<String>() {
            println!("Client received response via Hub 2 from Hub 1: {}", message);
        } else {
            println!("Client received response from Hub 1 with unknown data format");
        }
    }
}