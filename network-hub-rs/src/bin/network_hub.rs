use std::net::SocketAddr;
use std::str::FromStr;
use clap::{Command, Arg, ArgAction};

use network_hub::{Hub, HubScope, NetworkTransport, TlsConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up command line parsing
    let matches = Command::new("Network Hub")
        .version("0.1.0")
        .author("Anthropic Claude")
        .about("Secure network hub for the Information Network")
        .arg(
            Arg::new("bind")
                .short('b')
                .long("bind")
                .help("Address to bind to")
                .default_value("127.0.0.1:8443")
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("cert")
                .short('c')
                .long("cert")
                .help("Path to TLS certificate")
                .required(true)
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("key")
                .short('k')
                .long("key")
                .help("Path to TLS key")
                .required(true)
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
            Arg::new("scope")
                .short('s')
                .long("scope")
                .help("Hub scope (thread, process, machine, network)")
                .default_value("network")
                .action(ArgAction::Set),
        )
        .get_matches();

    // Get command line arguments
    let bind_address = matches.get_one::<String>("bind").unwrap();
    let cert_path = matches.get_one::<String>("cert").unwrap();
    let key_path = matches.get_one::<String>("key").unwrap();
    let ca_path = matches.get_one::<String>("ca").cloned();
    let scope_str = matches.get_one::<String>("scope").unwrap();

    // Parse bind address
    let bind_addr = SocketAddr::from_str(bind_address)?;

    // Parse scope
    let scope = match scope_str.to_lowercase().as_str() {
        "thread" => HubScope::Thread,
        "process" => HubScope::Process,
        "machine" => HubScope::Machine,
        _ => HubScope::Network,
    };

    // Initialize hub
    let hub = Hub::initialize(scope);
    println!("Hub initialized with ID: {} and scope: {:?}", hub.id, hub.scope);

    // Configure TLS
    let tls_config = TlsConfig {
        cert_path: cert_path.clone(),
        key_path: key_path.clone(),
        ca_path,
    };

    // Create and start network transport
    let transport = NetworkTransport::new(hub, bind_addr, tls_config);
    println!("Starting network transport on {}", bind_addr);
    transport.start()?;

    Ok(())
}