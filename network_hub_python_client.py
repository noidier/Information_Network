#!/usr/bin/env python3
# network_hub_python_client.py - Python client for network-hub-rs

import uuid
import time
import json
import socket
import ssl
import threading
import os
from enum import Enum
from dataclasses import dataclass
from typing import Any, Dict, List, Optional, Callable, TypeVar, Generic, Union, Tuple

# Type variables for generic types
T = TypeVar('T')
R = TypeVar('R')

# ========================
# Data Structures
# ========================

class HubScope(Enum):
    THREAD = "thread"
    PROCESS = "process"
    MACHINE = "machine"
    NETWORK = "network"

class ResponseStatus(Enum):
    SUCCESS = "success"
    NOT_FOUND = "not_found"
    ERROR = "error"
    INTERCEPTED = "intercepted"
    APPROXIMATED = "approximated"

@dataclass
class Message(Generic[T]):
    """A message with typed data."""
    topic: str
    data: T
    metadata: Dict[str, str]
    sender_id: str
    timestamp: int = 0

    def __post_init__(self):
        if self.timestamp == 0:
            self.timestamp = int(time.time() * 1000)

@dataclass
class ApiRequest:
    """A request to an API endpoint."""
    path: str
    data: Any
    metadata: Dict[str, str]
    sender_id: str

@dataclass
class ApiResponse:
    """A response from an API endpoint."""
    data: Any
    metadata: Dict[str, str]
    status: ResponseStatus

# ========================
# Network Client Implementation
# ========================

class NetworkHubClient:
    """Client to connect to the Rust network hub."""

    def __init__(self, host: str = "localhost", port: int = 8443, 
                 cert_path: Optional[str] = None, key_path: Optional[str] = None,
                 verify_ssl: bool = True, client_id: Optional[str] = None):
        """
        Initialize a new network hub client.
        
        Args:
            host: Host where the network hub is running.
            port: Port the network hub is listening on.
            cert_path: Path to the client certificate file.
            key_path: Path to the client key file.
            verify_ssl: Whether to verify SSL certificate.
            client_id: Client ID, generated if not provided.
        """
        self.host = host
        self.port = port
        self.cert_path = cert_path
        self.key_path = key_path
        self.verify_ssl = verify_ssl
        self.client_id = client_id or str(uuid.uuid4())
        self.reconnect_interval = 5  # seconds
        self.connect_lock = threading.Lock()
        self._connection = None

        # Message handlers for subscriptions
        self._subscription_handlers = {}
        self._subscription_thread = None
        self._running = False

    def connect(self) -> bool:
        """
        Connect to the network hub.
        
        Returns:
            True if connected, False otherwise.
        """
        with self.connect_lock:
            if self._connection is not None:
                return True

            try:
                # Create SSL context
                context = ssl.create_default_context(ssl.Purpose.SERVER_AUTH)
                if not self.verify_ssl:
                    context.check_hostname = False
                    context.verify_mode = ssl.CERT_NONE

                if self.cert_path and self.key_path:
                    context.load_cert_chain(certfile=self.cert_path, keyfile=self.key_path)

                # Create socket and connect
                sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                self._connection = context.wrap_socket(sock, server_hostname=self.host)
                self._connection.connect((self.host, self.port))
                
                print(f"Connected to network hub at {self.host}:{self.port}")
                
                # Start subscription listener if needed
                if self._subscription_handlers and not self._subscription_thread:
                    self._running = True
                    self._subscription_thread = threading.Thread(
                        target=self._subscription_listener, daemon=True)
                    self._subscription_thread.start()
                
                return True
            except (socket.error, ssl.SSLError) as e:
                print(f"Failed to connect to network hub: {e}")
                self._connection = None
                return False

    def disconnect(self):
        """Disconnect from the network hub."""
        with self.connect_lock:
            if self._connection:
                try:
                    self._running = False
                    if self._subscription_thread:
                        self._subscription_thread.join(timeout=2)
                    self._connection.close()
                finally:
                    self._connection = None

    def reconnect(self) -> bool:
        """
        Try to reconnect to the network hub.
        
        Returns:
            True if reconnected, False otherwise.
        """
        self.disconnect()
        return self.connect()

    def _ensure_connected(self) -> bool:
        """
        Ensure that the client is connected to the hub.
        
        Returns:
            True if connected, False if connection failed.
        """
        if self._connection is None:
            return self.connect()
        return True

    def _send_request(self, message_type: int, data: dict) -> Tuple[int, dict]:
        """
        Send a request to the network hub.
        
        Args:
            message_type: Type of message (1=ApiRequest, 3=Message).
            data: Data to send.
            
        Returns:
            Tuple of (response_type, response_data).
            
        Raises:
            ConnectionError: If not connected to the hub.
        """
        if not self._ensure_connected():
            raise ConnectionError("Not connected to the network hub")

        try:
            # Serialize request
            request_bytes = bytes([message_type]) + json.dumps(data).encode('utf-8')
            
            # Send request
            with self.connect_lock:  # Lock for thread safety
                self._connection.sendall(request_bytes)
                
                # Read response (assuming first byte is message type)
                response_type_bytes = self._connection.recv(1)
                if not response_type_bytes:
                    raise ConnectionError("Connection closed by server")
                
                response_type = response_type_bytes[0]
                
                # Read response data
                buffer = bytearray()
                while True:
                    chunk = self._connection.recv(4096)
                    if not chunk:
                        break
                    buffer.extend(chunk)
                    # In a real implementation, we would need a proper framing protocol
                    # This simplified version assumes one message per connection
                    if chunk[-1] == 0:  # Assuming 0 marks end of message
                        break
                
                # Parse response
                response_data = json.loads(buffer.decode('utf-8', errors='ignore'))
                return response_type, response_data
                
        except (socket.error, ssl.SSLError, json.JSONDecodeError) as e:
            self._connection = None
            raise ConnectionError(f"Error communicating with network hub: {e}")

    def register_api(self, path: str, handler: Callable[[ApiRequest], ApiResponse], 
                    metadata: Optional[Dict[str, str]] = None) -> bool:
        """
        Register an API endpoint with the network hub.
        
        In the Python client, API handlers are registered locally, and the hub
        forwards requests to us rather than us implementing the handlers on the hub.
        
        Args:
            path: API path to register.
            handler: Function to handle requests to this API.
            metadata: Optional metadata for the API.
            
        Returns:
            True if registration succeeded, False otherwise.
        """
        metadata = metadata or {}
        
        try:
            # Store handler locally
            self._api_handlers[path] = {
                'handler': handler,
                'metadata': metadata
            }
            
            # Register the path with the hub
            request_data = {
                'path': path,
                'metadata': metadata,
                'client_id': self.client_id
            }
            
            message_type, response_data = self._send_request(5, request_data)  # 5 = API registration
            
            return message_type == 6 and response_data.get('success', False)  # 6 = Registration response
        except Exception as e:
            print(f"Error registering API {path}: {e}")
            return False

    def call_api(self, path: str, data: Any = None, 
                metadata: Optional[Dict[str, str]] = None) -> ApiResponse:
        """
        Call an API endpoint on the network hub.
        
        Args:
            path: API path to call.
            data: Request data.
            metadata: Request metadata.
            
        Returns:
            API response.
            
        Raises:
            ConnectionError: If not connected to the hub.
        """
        metadata = metadata or {}
        
        # Create request
        request = {
            'path': path,
            'data': data,
            'metadata': metadata,
            'sender_id': self.client_id
        }
        
        # Send request
        try:
            message_type, response_data = self._send_request(1, request)  # 1 = API request
            
            if message_type != 2:  # 2 = API response
                raise ValueError(f"Unexpected response type: {message_type}")
            
            # Convert status
            status_map = {
                0: ResponseStatus.SUCCESS,
                1: ResponseStatus.NOT_FOUND,
                2: ResponseStatus.ERROR,
                3: ResponseStatus.INTERCEPTED,
                4: ResponseStatus.APPROXIMATED
            }
            
            status = status_map.get(response_data.get('status', 0), ResponseStatus.ERROR)
            
            return ApiResponse(
                data=response_data.get('data'),
                metadata=response_data.get('metadata', {}),
                status=status
            )
        except Exception as e:
            print(f"Error calling API {path}: {e}")
            return ApiResponse(
                data=None,
                metadata={'error': str(e)},
                status=ResponseStatus.ERROR
            )

    def subscribe(self, pattern: str, callback: Callable[[Message], None], 
                priority: int = 0) -> str:
        """
        Subscribe to messages matching a pattern.
        
        Args:
            pattern: Message pattern to match (can use * wildcard).
            callback: Function to call when a matching message is received.
            priority: Subscription priority (higher is checked first).
            
        Returns:
            Subscription ID.
        """
        subscription_id = str(uuid.uuid4())
        
        # Store handler locally
        if pattern not in self._subscription_handlers:
            self._subscription_handlers[pattern] = []
            
        self._subscription_handlers[pattern].append({
            'id': subscription_id,
            'callback': callback,
            'priority': priority
        })
        
        # Sort by priority (descending)
        self._subscription_handlers[pattern].sort(
            key=lambda x: x['priority'], reverse=True
        )
        
        # Register subscription with hub if we're connected
        if self._connection:
            try:
                request_data = {
                    'pattern': pattern,
                    'subscription_id': subscription_id,
                    'client_id': self.client_id,
                    'priority': priority
                }
                
                message_type, response_data = self._send_request(7, request_data)  # 7 = Subscribe
                
                if message_type != 8 or not response_data.get('success', False):  # 8 = Subscribe response
                    print(f"Warning: Subscription registration failed: {response_data.get('error', 'Unknown error')}")
            except Exception as e:
                print(f"Error registering subscription: {e}")
        
        # Start subscription listener if not already running
        if not self._subscription_thread and self._connection:
            self._running = True
            self._subscription_thread = threading.Thread(
                target=self._subscription_listener, daemon=True)
            self._subscription_thread.start()
        
        return subscription_id

    def _subscription_listener(self):
        """Listen for messages from the hub and dispatch to subscribers."""
        while self._running:
            try:
                if not self._ensure_connected():
                    time.sleep(self.reconnect_interval)
                    continue
                
                # In a real implementation, we'd have a proper protocol for reading messages
                # This simplified version just sleeps and periodically checks for reconnection
                time.sleep(1)
                
                # TODO: Implement real message reception and dispatch to subscribers
                
            except Exception as e:
                print(f"Error in subscription listener: {e}")
                time.sleep(self.reconnect_interval)

    def publish(self, topic: str, data: Any, 
                metadata: Optional[Dict[str, str]] = None) -> Optional[Any]:
        """
        Publish a message to the network.
        
        Args:
            topic: Message topic.
            data: Message data.
            metadata: Message metadata.
            
        Returns:
            Result from any interceptors, or None.
            
        Raises:
            ConnectionError: If not connected to the hub.
        """
        metadata = metadata or {}
        
        # Create message
        message = {
            'topic': topic,
            'data': data,
            'metadata': metadata,
            'sender_id': self.client_id,
            'timestamp': int(time.time() * 1000)
        }
        
        # Send message
        try:
            message_type, response_data = self._send_request(3, message)  # 3 = Message
            
            if message_type == 4:  # 4 = Intercepted message
                return response_data.get('result')
            
            return None  # Not intercepted
        except Exception as e:
            print(f"Error publishing message to {topic}: {e}")
            return None

# ========================
# Node Implementation
# ========================

class Node:
    """A node in the virtual network that connects to the hub system."""
    
    def __init__(self, host: str = "localhost", port: int = 8443, 
                cert_path: Optional[str] = None, key_path: Optional[str] = None,
                verify_ssl: bool = True, node_id: Optional[str] = None):
        """Initialize a new node with an optional ID."""
        self.node_id = node_id or str(uuid.uuid4())
        self.hub_client = NetworkHubClient(
            host=host, port=port, cert_path=cert_path, key_path=key_path,
            verify_ssl=verify_ssl, client_id=self.node_id
        )
        
    def connect(self) -> bool:
        """Connect to the network hub."""
        return self.hub_client.connect()
        
    def register_api(self, path: str, handler: Callable[[ApiRequest], ApiResponse], 
                    metadata: Optional[Dict[str, str]] = None) -> bool:
        """Register an API endpoint with the node."""
        return self.hub_client.register_api(path, handler, metadata)
        
    def call_api(self, path: str, data: Any = None, 
                metadata: Optional[Dict[str, str]] = None) -> ApiResponse:
        """Call an API endpoint on the network."""
        return self.hub_client.call_api(path, data, metadata)
        
    def subscribe(self, pattern: str, callback: Callable[[Message], None], 
                priority: int = 0) -> str:
        """Subscribe to messages matching a pattern."""
        return self.hub_client.subscribe(pattern, callback, priority)
        
    def publish(self, topic: str, data: Any, 
                metadata: Optional[Dict[str, str]] = None) -> Optional[Any]:
        """Publish a message to the network."""
        return self.hub_client.publish(topic, data, metadata)
        
    def disconnect(self):
        """Disconnect from the network hub."""
        self.hub_client.disconnect()

# ========================
# Helper Functions
# ========================

def create_node(host: str = "localhost", port: int = 8443, 
               cert_path: Optional[str] = None, key_path: Optional[str] = None,
               verify_ssl: bool = True, node_id: Optional[str] = None) -> Node:
    """Create a new node in the virtual network."""
    return Node(host, port, cert_path, key_path, verify_ssl, node_id)

# ========================
# Usage Example
# ========================

def example():
    """Example usage of the Python client."""
    
    # Create a node
    node = create_node(
        host="localhost", 
        port=8443,
        cert_path="certs/client.pem",
        key_path="certs/client_key.pem",
        verify_ssl=False  # For testing only
    )
    
    # Connect to the hub
    if not node.connect():
        print("Failed to connect to the hub")
        return
    
    try:
        # Call an API
        response = node.call_api("/hub1/greeting")
        print(f"API response: {response.data}")
        
        # Register a callback for message subscription
        def message_handler(message):
            print(f"Received message on {message.topic}: {message.data}")
        
        # Subscribe to messages
        sub_id = node.subscribe("test/*", message_handler)
        print(f"Subscription ID: {sub_id}")
        
        # Publish a message
        result = node.publish("test/hello", "Hello from Python client")
        print(f"Publish result: {result}")
        
        # Keep the program running to receive messages
        print("Press Ctrl+C to exit")
        try:
            while True:
                time.sleep(1)
        except KeyboardInterrupt:
            print("Exiting...")
    finally:
        # Disconnect when done
        node.disconnect()

if __name__ == "__main__":
    example()