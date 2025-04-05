use std::io::{Read, Write};
use std::net::TcpStream;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use std::path::Path;

use rustls::{Certificate, PrivateKey, ServerConfig, ClientConfig};
use rustls::server::AllowAnyAuthenticatedClient;
use rustls_pemfile::{certs, pkcs8_private_keys};

use crate::error::{HubError, Result};

/// TLS configuration for secure communication
#[derive(Clone)]
pub struct TlsConfig {
    /// Path to certificate file
    pub cert_path: String,
    /// Path to private key file
    pub key_path: String,
    /// Optional path to CA certificate file for client authentication
    pub ca_path: Option<String>,
}

/// TLS stream wrapper
pub struct TlsStream {
    /// The underlying stream (client or server)
    inner: Box<dyn StreamLike>,
}

/// Trait for common stream operations
pub trait StreamLike: Read + Write + Send + Sync {}

// Implement StreamLike for TcpStream
impl StreamLike for TcpStream {}

// Implement Read for TlsStream by delegating to inner
impl Read for TlsStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

// Implement Write for TlsStream by delegating to inner
impl Write for TlsStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }
    
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// Load certificates from a file
fn load_certs(path: &Path) -> Result<Vec<Certificate>> {
    let file = File::open(path).map_err(|e| HubError::Io(e))?;
    let mut reader = BufReader::new(file);
    let certs = certs(&mut reader)
        .map_err(|e| HubError::Tls(format!("Failed to load certificates: {}", e)))?
        .iter()
        .map(|v| Certificate(v.clone()))
        .collect();
    
    Ok(certs)
}

/// Load private keys from a file
fn load_keys(path: &Path) -> Result<Vec<PrivateKey>> {
    let file = File::open(path).map_err(|e| HubError::Io(e))?;
    let mut reader = BufReader::new(file);
    let keys = pkcs8_private_keys(&mut reader)
        .map_err(|e| HubError::Tls(format!("Failed to load private keys: {}", e)))?
        .iter()
        .map(|v| PrivateKey(v.clone()))
        .collect();
    
    Ok(keys)
}

/// Create a server TLS configuration
fn create_server_config(config: &TlsConfig) -> Result<ServerConfig> {
    let certs = load_certs(Path::new(&config.cert_path))?;
    let mut keys = load_keys(Path::new(&config.key_path))?;
    
    if keys.is_empty() {
        return Err(HubError::Tls("No private keys found".to_string()));
    }
    
    let server_config = if let Some(ca_path) = &config.ca_path {
        // Set up client authentication
        let client_auth_roots = load_certs(Path::new(ca_path))?;
        let mut root_store = rustls::RootCertStore::empty();
        for cert in client_auth_roots {
            root_store.add(&cert)
                .map_err(|e| HubError::Tls(format!("Failed to add CA certificate: {}", e)))?;
        }
        
        let client_auth = AllowAnyAuthenticatedClient::new(root_store);
        
        ServerConfig::builder()
            .with_safe_defaults()
            .with_client_cert_verifier(Arc::new(client_auth))
            .with_single_cert(certs, keys.remove(0))
            .map_err(|e| HubError::Tls(format!("Failed to create server config: {}", e)))?
    } else {
        // No client authentication
        ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, keys.remove(0))
            .map_err(|e| HubError::Tls(format!("Failed to create server config: {}", e)))?
    };
    
    Ok(server_config)
}

/// Create a client TLS configuration
fn create_client_config(config: &TlsConfig) -> Result<ClientConfig> {
    let certs = load_certs(Path::new(&config.cert_path))?;
    let mut keys = load_keys(Path::new(&config.key_path))?;
    
    if keys.is_empty() {
        return Err(HubError::Tls("No private keys found".to_string()));
    }
    
    let mut root_store = rustls::RootCertStore::empty();
    if let Some(ca_path) = &config.ca_path {
        let ca_certs = load_certs(Path::new(ca_path))?;
        for cert in ca_certs {
            root_store.add(&cert)
                .map_err(|e| HubError::Tls(format!("Failed to add CA certificate: {}", e)))?;
        }
    }
    
    let client_config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_single_cert(certs, keys.remove(0))
        .map_err(|e| HubError::Tls(format!("Failed to create client config: {}", e)))?;
    
    Ok(client_config)
}

/// Create a server TLS stream
pub fn create_server_tls_stream(stream: TcpStream, config: &TlsConfig) -> Result<TlsStream> {
    // Create server config
    let server_config = create_server_config(config)?;
    let acceptor = rustls::ServerConnection::new(Arc::new(server_config))
        .map_err(|e| HubError::Tls(format!("Failed to create TLS acceptor: {}", e)))?;
    
    // Create rustls stream
    let tls_stream = rustls::StreamOwned::new(acceptor, stream);
    
    // Convert to our TlsStream type
    struct ServerTlsStream<T> {
        stream: rustls::StreamOwned<rustls::ServerConnection, T>,
    }
    
    impl<T: Read + Write + Send + Sync> Read for ServerTlsStream<T> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.stream.read(buf)
        }
    }
    
    impl<T: Read + Write + Send + Sync> Write for ServerTlsStream<T> {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.stream.write(buf)
        }
        
        fn flush(&mut self) -> std::io::Result<()> {
            self.stream.flush()
        }
    }
    
    impl<T: Read + Write + Send + Sync> StreamLike for ServerTlsStream<T> {}
    
    let server_stream = ServerTlsStream { stream: tls_stream };
    
    Ok(TlsStream {
        inner: Box::new(server_stream),
    })
}

/// Create a client TLS stream
pub fn create_client_tls_stream(stream: TcpStream, config: &TlsConfig) -> Result<TlsStream> {
    // Create client config
    let client_config = create_client_config(config)?;
    
    // Get server name from config for SNI or use localhost as default
    let server_name = rustls::ServerName::try_from("localhost")
        .map_err(|e| HubError::Tls(format!("Invalid server name: {}", e)))?;
    
    // Create TLS connector
    let connector = rustls::ClientConnection::new(Arc::new(client_config), server_name)
        .map_err(|e| HubError::Tls(format!("Failed to create TLS connector: {}", e)))?;
    
    // Create rustls stream
    let tls_stream = rustls::StreamOwned::new(connector, stream);
    
    // Convert to our TlsStream type
    struct ClientTlsStream<T> {
        stream: rustls::StreamOwned<rustls::ClientConnection, T>,
    }
    
    impl<T: Read + Write + Send + Sync> Read for ClientTlsStream<T> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.stream.read(buf)
        }
    }
    
    impl<T: Read + Write + Send + Sync> Write for ClientTlsStream<T> {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.stream.write(buf)
        }
        
        fn flush(&mut self) -> std::io::Result<()> {
            self.stream.flush()
        }
    }
    
    impl<T: Read + Write + Send + Sync> StreamLike for ClientTlsStream<T> {}
    
    let client_stream = ClientTlsStream { stream: tls_stream };
    
    Ok(TlsStream {
        inner: Box::new(client_stream),
    })
}