use std::net::SocketAddr;
use std::str::FromStr;
use clap::{Command, Arg, ArgAction};

use network_hub::{Hub, HubScope, HttpReverseProxy, TlsConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up command line parsing
    let matches = Command::new("Reverse Proxy")
        .version("0.1.0")
        .author("Anthropic Claude")
        .about("Secure reverse proxy for the Information Network")
        .arg(
            Arg::new("bind")
                .short('b')
                .long("bind")
                .help("Address to bind to")
                .default_value("127.0.0.1:8080")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("cert")
                .short('c')
                .long("cert")
                .help("Path to TLS certificate")
                .default_value("certs/cert.pem")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("key")
                .short('k')
                .long("key")
                .help("Path to TLS key")
                .default_value("certs/key.pem")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("ca")
                .long("ca")
                .help("Path to CA certificate for client authentication")
                .required(false)
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("add-route")
                .long("add-route")
                .help("Add a proxy route (format: /path=https://target.example.com)")
                .required(false)
                .action(ArgAction::Set)
                .num_args(1..)
        )
        .get_matches();

    // Get command line arguments
    let bind_address = matches.get_one::<String>("bind").unwrap();
    let cert_path = matches.get_one::<String>("cert").unwrap();
    let key_path = matches.get_one::<String>("key").unwrap();
    let ca_path = matches.get_one::<String>("ca").cloned();

    // Parse bind address
    let bind_addr = SocketAddr::from_str(bind_address)?;

    // Initialize hub
    let hub = Hub::initialize(HubScope::Network);
    println!("Hub initialized with ID: {} and scope: {:?}", hub.id, hub.scope);

    // Configure TLS - Note: This is a placeholder implementation
    println!("\n==============================================================");
    println!("WARNING: This is using a PLACEHOLDER TLS implementation!");
    println!("In a real application, you would need proper TLS setup.");
    println!("This proxy will accept plain HTTP connections on {}", bind_addr);
    println!("==============================================================\n");

    let tls_config = TlsConfig {
        cert_path: cert_path.clone(),
        key_path: key_path.clone(),
        ca_path,
    };

    // Create reverse proxy
    let proxy = HttpReverseProxy::new(hub, bind_addr, tls_config);

    // Add default routes
    proxy.add_route("/", "https://example.com");
    
    // Add routes from command line
    if let Some(routes) = matches.get_many::<String>("add-route") {
        for route_str in routes {
            if let Some((path, target)) = route_str.split_once('=') {
                proxy.add_route(path, target);
                println!("Added route: {} -> {}", path, target);
            } else {
                eprintln!("Invalid route format: {}", route_str);
                eprintln!("Expected format: /path=https://target.example.com");
            }
        }
    }

    println!("\nConfigured routes:");
    println!("/ -> https://example.com (default)");
    
    // Start the proxy
    println!("\nStarting reverse proxy on {}", bind_addr);
    println!("Using TLS certificate (placeholder): {}", cert_path);
    println!("Using TLS key (placeholder): {}", key_path);
    println!("\nPress Ctrl+C to exit");
    
    proxy.start()?;

    Ok(())
}