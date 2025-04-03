use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use network_hub::hub::{Hub, HubScope, ApiRequest, ApiResponse, ResponseStatus};
use network_hub::utils::generate_uuid;

/// A simple calculator service that will register with the hub
struct CalculatorService {
    hub: Arc<Hub>,
    service_id: String,
}

impl CalculatorService {
    fn new(hub: Arc<Hub>) -> Self {
        let service_id = format!("calculator-{}", generate_uuid());
        
        // Register APIs with the hub
        Self::register_apis(Arc::clone(&hub), service_id.clone());
        
        CalculatorService {
            hub,
            service_id,
        }
    }
    
    fn register_apis(hub: Arc<Hub>, service_id: String) {
        // Add API
        let service_id_clone = service_id.clone();
        hub.register_api("/calculator/add", move |request| {
            println!("[{}] Handling add request", service_id_clone);
            
            // Try to extract numbers from the request
            if let Some((a, b)) = request.data.downcast_ref::<(i32, i32)>() {
                let result = a + b;
                println!("[{}] Calculated {} + {} = {}", service_id_clone, a, b, result);
                
                ApiResponse {
                    data: Box::new(result),
                    metadata: HashMap::new(),
                    status: ResponseStatus::Success,
                }
            } else {
                println!("[{}] Invalid data format for add request", service_id_clone);
                
                ApiResponse {
                    data: Box::new("Invalid input - expected (i32, i32)"),
                    metadata: HashMap::new(),
                    status: ResponseStatus::Error,
                }
            }
        }, HashMap::new());
        
        // Subtract API
        let service_id_clone = service_id.clone();
        hub.register_api("/calculator/subtract", move |request| {
            println!("[{}] Handling subtract request", service_id_clone);
            
            // Try to extract numbers from the request
            if let Some((a, b)) = request.data.downcast_ref::<(i32, i32)>() {
                let result = a - b;
                println!("[{}] Calculated {} - {} = {}", service_id_clone, a, b, result);
                
                ApiResponse {
                    data: Box::new(result),
                    metadata: HashMap::new(),
                    status: ResponseStatus::Success,
                }
            } else {
                println!("[{}] Invalid data format for subtract request", service_id_clone);
                
                ApiResponse {
                    data: Box::new("Invalid input - expected (i32, i32)"),
                    metadata: HashMap::new(),
                    status: ResponseStatus::Error,
                }
            }
        }, HashMap::new());
        
        // Multiply API
        let service_id_clone = service_id;
        hub.register_api("/calculator/multiply", move |request| {
            println!("[{}] Handling multiply request", service_id_clone);
            
            // Try to extract numbers from the request
            if let Some((a, b)) = request.data.downcast_ref::<(i32, i32)>() {
                let result = a * b;
                println!("[{}] Calculated {} * {} = {}", service_id_clone, a, b, result);
                
                ApiResponse {
                    data: Box::new(result),
                    metadata: HashMap::new(),
                    status: ResponseStatus::Success,
                }
            } else {
                println!("[{}] Invalid data format for multiply request", service_id_clone);
                
                ApiResponse {
                    data: Box::new("Invalid input - expected (i32, i32)"),
                    metadata: HashMap::new(),
                    status: ResponseStatus::Error,
                }
            }
        }, HashMap::new());
    }
}

/// A service that will use other services through the hub
struct MathService {
    hub: Arc<Hub>,
    service_id: String,
}

impl MathService {
    fn new(hub: Arc<Hub>) -> Self {
        let service_id = format!("math-{}", generate_uuid());
        
        // Register APIs with the hub
        Self::register_apis(Arc::clone(&hub), service_id.clone());
        
        MathService {
            hub,
            service_id,
        }
    }
    
    fn register_apis(hub: Arc<Hub>, service_id: String) {
        // Square API - uses multiply
        let hub_clone = Arc::clone(&hub);
        let service_id_clone = service_id.clone();
        hub.register_api("/math/square", move |request| {
            println!("[{}] Handling square request", service_id_clone);
            
            // Try to extract number from the request
            if let Some(n) = request.data.downcast_ref::<i32>() {
                println!("[{}] Squaring {} by calling calculator/multiply", service_id_clone, n);
                
                // Create a request to the calculator's multiply API
                let multiply_request = ApiRequest {
                    path: "/calculator/multiply".to_string(),
                    data: Box::new((*n, *n)),
                    metadata: HashMap::new(),
                    sender_id: service_id_clone.clone(),
                };
                
                // Send the request through the hub
                let response = hub_clone.handle_request(multiply_request);
                
                // Forward the result
                response
            } else {
                println!("[{}] Invalid data format for square request", service_id_clone);
                
                ApiResponse {
                    data: Box::new("Invalid input - expected i32"),
                    metadata: HashMap::new(),
                    status: ResponseStatus::Error,
                }
            }
        }, HashMap::new());
        
        // Calculate expression API - uses multiple calculator APIs
        let hub_clone = Arc::clone(&hub);
        let service_id_clone = service_id;
        hub.register_api("/math/evaluate", move |request| {
            println!("[{}] Handling expression evaluation request", service_id_clone);
            
            // Try to extract expression from the request
            // Format: ("OPERATION", num1, num2, ...)
            if let Some((op, nums)) = request.data.downcast_ref::<(String, Vec<i32>)>() {
                println!("[{}] Evaluating expression: {} {:?}", service_id_clone, op, nums);
                
                if nums.is_empty() {
                    return ApiResponse {
                        data: Box::new("No numbers provided for operation"),
                        metadata: HashMap::new(),
                        status: ResponseStatus::Error,
                    };
                }
                
                // Start with the first number
                let mut result = nums[0];
                
                // Apply the operation to each subsequent number
                for &num in &nums[1..] {
                    let calculator_path = match op.as_str() {
                        "ADD" => "/calculator/add",
                        "SUBTRACT" => "/calculator/subtract",
                        "MULTIPLY" => "/calculator/multiply",
                        _ => {
                            return ApiResponse {
                                data: Box::new(format!("Unknown operation: {}", op)),
                                metadata: HashMap::new(),
                                status: ResponseStatus::Error,
                            };
                        }
                    };
                    
                    // Create a request to the calculator API
                    let calc_request = ApiRequest {
                        path: calculator_path.to_string(),
                        data: Box::new((result, num)),
                        metadata: HashMap::new(),
                        sender_id: service_id_clone.clone(),
                    };
                    
                    // Send the request through the hub
                    let response = hub_clone.handle_request(calc_request);
                    
                    // Process the result
                    if response.status == ResponseStatus::Success {
                        if let Some(&r) = response.data.downcast_ref::<i32>() {
                            result = r;
                        } else {
                            return ApiResponse {
                                data: Box::new("Unexpected response type from calculator"),
                                metadata: HashMap::new(),
                                status: ResponseStatus::Error,
                            };
                        }
                    } else {
                        return response;
                    }
                }
                
                println!("[{}] Expression result: {}", service_id_clone, result);
                
                ApiResponse {
                    data: Box::new(result),
                    metadata: HashMap::new(),
                    status: ResponseStatus::Success,
                }
            } else {
                println!("[{}] Invalid data format for expression evaluation", service_id_clone);
                
                ApiResponse {
                    data: Box::new("Invalid input - expected (String, Vec<i32>)"),
                    metadata: HashMap::new(),
                    status: ResponseStatus::Error,
                }
            }
        }, HashMap::new());
    }
}

/// A client that will use the services through the hub
struct Client {
    hub: Arc<Hub>,
    client_id: String,
    results: Arc<Mutex<Vec<String>>>,
}

impl Client {
    fn new(hub: Arc<Hub>) -> Self {
        let client_id = format!("client-{}", generate_uuid());
        
        Client {
            hub,
            client_id,
            results: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    fn run(&self) {
        let hub = Arc::clone(&self.hub);
        let client_id = self.client_id.clone();
        let results = Arc::clone(&self.results);
        
        thread::spawn(move || {
            // Perform several operations through the hub
            
            // 1. Simple addition
            println!("[{}] Requesting 5 + 3", client_id);
            let add_request = ApiRequest {
                path: "/calculator/add".to_string(),
                data: Box::new((5, 3)),
                metadata: HashMap::new(),
                sender_id: client_id.clone(),
            };
            
            let add_response = hub.handle_request(add_request);
            if add_response.status == ResponseStatus::Success {
                if let Some(&result) = add_response.data.downcast_ref::<i32>() {
                    println!("[{}] 5 + 3 = {}", client_id, result);
                    results.lock().unwrap().push(format!("5 + 3 = {}", result));
                }
            }
            
            // A short delay between operations
            thread::sleep(Duration::from_millis(100));
            
            // 2. Square a number
            println!("[{}] Requesting square of 7", client_id);
            let square_request = ApiRequest {
                path: "/math/square".to_string(),
                data: Box::new(7),
                metadata: HashMap::new(),
                sender_id: client_id.clone(),
            };
            
            let square_response = hub.handle_request(square_request);
            if square_response.status == ResponseStatus::Success {
                if let Some(&result) = square_response.data.downcast_ref::<i32>() {
                    println!("[{}] 7² = {}", client_id, result);
                    results.lock().unwrap().push(format!("7² = {}", result));
                }
            }
            
            // A short delay between operations
            thread::sleep(Duration::from_millis(100));
            
            // 3. Evaluate a complex expression
            println!("[{}] Requesting evaluation of (10 * 5) - 8", client_id);
            
            // First, calculate 10 * 5
            let mult_request = ApiRequest {
                path: "/calculator/multiply".to_string(),
                data: Box::new((10, 5)),
                metadata: HashMap::new(),
                sender_id: client_id.clone(),
            };
            
            let mult_response = hub.handle_request(mult_request);
            let mut intermediate_result = 0;
            
            if mult_response.status == ResponseStatus::Success {
                if let Some(&result) = mult_response.data.downcast_ref::<i32>() {
                    intermediate_result = result;
                    println!("[{}] Intermediate: 10 * 5 = {}", client_id, result);
                }
            }
            
            // Then subtract 8
            let sub_request = ApiRequest {
                path: "/calculator/subtract".to_string(),
                data: Box::new((intermediate_result, 8)),
                metadata: HashMap::new(),
                sender_id: client_id.clone(),
            };
            
            let sub_response = hub.handle_request(sub_request);
            if sub_response.status == ResponseStatus::Success {
                if let Some(&result) = sub_response.data.downcast_ref::<i32>() {
                    println!("[{}] (10 * 5) - 8 = {}", client_id, result);
                    results.lock().unwrap().push(format!("(10 * 5) - 8 = {}", result));
                }
            }
            
            // 4. Use the expression evaluator for a complex calculation
            println!("[{}] Requesting evaluation of expression: 2 + 3 + 4", client_id);
            let expr_request = ApiRequest {
                path: "/math/evaluate".to_string(),
                data: Box::new(("ADD".to_string(), vec![2, 3, 4])),
                metadata: HashMap::new(),
                sender_id: client_id.clone(),
            };
            
            let expr_response = hub.handle_request(expr_request);
            if expr_response.status == ResponseStatus::Success {
                if let Some(&result) = expr_response.data.downcast_ref::<i32>() {
                    println!("[{}] 2 + 3 + 4 = {}", client_id, result);
                    results.lock().unwrap().push(format!("2 + 3 + 4 = {}", result));
                }
            }
            
            // 5. Another expression evaluation
            println!("[{}] Requesting evaluation of expression: 10 * 2 * 3", client_id);
            let expr_request = ApiRequest {
                path: "/math/evaluate".to_string(),
                data: Box::new(("MULTIPLY".to_string(), vec![10, 2, 3])),
                metadata: HashMap::new(),
                sender_id: client_id.clone(),
            };
            
            let expr_response = hub.handle_request(expr_request);
            if expr_response.status == ResponseStatus::Success {
                if let Some(&result) = expr_response.data.downcast_ref::<i32>() {
                    println!("[{}] 10 * 2 * 3 = {}", client_id, result);
                    results.lock().unwrap().push(format!("10 * 2 * 3 = {}", result));
                }
            }
        });
    }
    
    fn get_results(&self) -> Vec<String> {
        self.results.lock().unwrap().clone()
    }
}

fn main() {
    println!("Starting Thread Hub Demo");
    
    // Create a thread-level hub
    let hub = Arc::new(Hub::new(HubScope::Thread));
    println!("Created thread hub with ID: {}", hub.id);
    
    // Initialize services
    println!("Initializing Calculator Service...");
    let _calculator_service = CalculatorService::new(Arc::clone(&hub));
    
    println!("Initializing Math Service...");
    let _math_service = MathService::new(Arc::clone(&hub));
    
    // Create multiple clients
    println!("Creating clients...");
    let client1 = Client::new(Arc::clone(&hub));
    let client2 = Client::new(Arc::clone(&hub));
    
    // Run clients
    println!("Starting clients...");
    client1.run();
    client2.run();
    
    // Allow time for clients to complete their operations
    println!("Waiting for clients to complete operations...");
    thread::sleep(Duration::from_secs(1));
    
    // Display results
    println!("\n--- Client 1 Results ---");
    for result in client1.get_results() {
        println!("{}", result);
    }
    
    println!("\n--- Client 2 Results ---");
    for result in client2.get_results() {
        println!("{}", result);
    }
    
    println!("\nThread Hub Demo completed successfully!");
}