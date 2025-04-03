#!/usr/bin/env python3
# shallow_hub_python.py - Lightweight in-process hub implementation for Python

import uuid
import time
import json
import threading
import multiprocessing
from multiprocessing import Process, Manager, Queue, Event
from enum import Enum
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Callable, TypeVar, Generic, Union, Tuple, Set
import inspect
import re
import traceback
from functools import partial

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
            
    def to_dict(self):
        return {
            "topic": self.topic,
            "data": self.data,
            "metadata": self.metadata,
            "sender_id": self.sender_id,
            "timestamp": self.timestamp
        }
    
    @classmethod
    def from_dict(cls, data):
        return cls(
            topic=data["topic"],
            data=data["data"],
            metadata=data["metadata"],
            sender_id=data["sender_id"],
            timestamp=data["timestamp"]
        )

@dataclass
class ApiRequest:
    """A request to an API endpoint."""
    path: str
    data: Any
    metadata: Dict[str, str]
    sender_id: str
    
    def to_dict(self):
        return {
            "path": self.path,
            "data": self.data,
            "metadata": self.metadata,
            "sender_id": self.sender_id
        }
    
    @classmethod
    def from_dict(cls, data):
        return cls(
            path=data["path"],
            data=data["data"],
            metadata=data["metadata"],
            sender_id=data["sender_id"]
        )

@dataclass
class ApiResponse:
    """A response from an API endpoint."""
    data: Any
    metadata: Dict[str, str]
    status: ResponseStatus
    
    def to_dict(self):
        return {
            "data": self.data,
            "metadata": self.metadata,
            "status": self.status.value
        }
    
    @classmethod
    def from_dict(cls, data):
        return cls(
            data=data["data"],
            metadata=data["metadata"],
            status=ResponseStatus(data["status"])
        )

@dataclass
class ApiEntry:
    """A registered API endpoint."""
    path: str
    handler: Callable[[ApiRequest], ApiResponse]
    metadata: Dict[str, str]
    client_id: str
    
    def to_dict(self):
        return {
            "path": self.path,
            "metadata": self.metadata,
            "client_id": self.client_id
        }

@dataclass
class Subscription:
    """A subscription to messages."""
    pattern: str
    client_id: str
    subscription_id: str
    priority: int = 0
    
    def to_dict(self):
        return {
            "pattern": self.pattern,
            "client_id": self.client_id,
            "subscription_id": self.subscription_id,
            "priority": self.priority
        }

@dataclass
class Interceptor:
    """An interceptor for messages."""
    pattern: str
    client_id: str
    interceptor_id: str
    priority: int = 0
    
    def to_dict(self):
        return {
            "pattern": self.pattern,
            "client_id": self.client_id,
            "interceptor_id": self.interceptor_id,
            "priority": self.priority
        }

# ========================
# Message Queue Implementation
# ========================

class HubMessage:
    """Internal message format for hub communication."""
    
    # Message types
    TYPE_API_REQUEST = 1
    TYPE_API_RESPONSE = 2
    TYPE_PUBLISH = 3
    TYPE_SUBSCRIBE = 4
    TYPE_REGISTER_API = 5
    TYPE_REGISTER_INTERCEPTOR = 6
    TYPE_SHUTDOWN = 99
    
    def __init__(self, msg_type, data, client_id, message_id=None):
        self.type = msg_type
        self.data = data
        self.client_id = client_id
        self.message_id = message_id or str(uuid.uuid4())
        self.timestamp = int(time.time() * 1000)
    
    def to_dict(self):
        return {
            "type": self.type,
            "data": self.data,
            "client_id": self.client_id,
            "message_id": self.message_id,
            "timestamp": self.timestamp
        }
    
    @classmethod
    def from_dict(cls, data):
        return cls(
            msg_type=data["type"],
            data=data["data"],
            client_id=data["client_id"],
            message_id=data["message_id"]
        )

# ========================
# Hub Implementation
# ========================

class ShallowHub:
    """A lightweight hub implementation for in-process communication."""
    
    def __init__(self, scope=HubScope.PROCESS):
        """Initialize a new shallow hub."""
        self.hub_id = str(uuid.uuid4())
        self.scope = scope
        
        # Use Manager for shared data across processes
        self.manager = Manager()
        
        # Shared data structures
        self.api_registry = self.manager.dict()  # path -> ApiEntry
        self.subscriptions = self.manager.dict()  # pattern -> list of Subscription
        self.interceptors = self.manager.dict()  # pattern -> list of Interceptor
        self.clients = self.manager.dict()  # client_id -> {"queue": Queue, "active": bool}
        
        # Message queues
        self.hub_queue = Queue()  # For messages to the hub
        self.running = Event()
        self.running.set()
        
        # Start the hub process
        self.hub_process = Process(target=self._run_hub)
        self.hub_process.daemon = True
        self.hub_process.start()
        
        print(f"Shallow Hub started with ID: {self.hub_id}")
    
    def shutdown(self):
        """Shutdown the hub and all client connections."""
        print(f"Shutting down hub {self.hub_id}...")
        self.running.clear()
        
        # Send shutdown message to hub
        shutdown_msg = HubMessage(
            HubMessage.TYPE_SHUTDOWN,
            {"reason": "shutdown requested"},
            self.hub_id
        )
        self.hub_queue.put(shutdown_msg.to_dict())
        
        # Wait for hub to process shutdown
        self.hub_process.join(timeout=5)
        if self.hub_process.is_alive():
            print("Hub process did not shut down gracefully, terminating...")
            self.hub_process.terminate()
    
    def _run_hub(self):
        """Main hub process loop."""
        print(f"Hub process {self.hub_id} started")
        
        while self.running.is_set():
            try:
                # Get message with timeout to allow checking if running
                try:
                    message_dict = self.hub_queue.get(timeout=1)
                except:
                    continue
                
                # Process the message
                self._process_hub_message(message_dict)
            except Exception as e:
                print(f"Error in hub process: {e}")
                traceback.print_exc()
        
        print(f"Hub process {self.hub_id} shutting down")
    
    def _process_hub_message(self, message_dict):
        """Process a message received by the hub."""
        message = HubMessage.from_dict(message_dict)
        
        if message.type == HubMessage.TYPE_SHUTDOWN:
            print(f"Hub received shutdown message: {message.data.get('reason', 'No reason provided')}")
            self.running.clear()
            return
        
        elif message.type == HubMessage.TYPE_REGISTER_API:
            # Register an API endpoint
            api_data = message.data
            self.api_registry[api_data["path"]] = api_data
            
            # Send acknowledgment to client
            response = {
                "success": True,
                "api_path": api_data["path"]
            }
            self._send_response_to_client(message.client_id, message.message_id, response)
        
        elif message.type == HubMessage.TYPE_SUBSCRIBE:
            # Register a subscription
            sub_data = message.data
            pattern = sub_data["pattern"]
            
            if pattern not in self.subscriptions:
                self.subscriptions[pattern] = []
            
            # Add subscription
            subscription = Subscription(
                pattern=pattern,
                client_id=message.client_id,
                subscription_id=sub_data["subscription_id"],
                priority=sub_data.get("priority", 0)
            )
            
            self.subscriptions[pattern].append(subscription.to_dict())
            
            # Sort by priority (descending)
            self.subscriptions[pattern] = sorted(
                self.subscriptions[pattern],
                key=lambda x: x["priority"],
                reverse=True
            )
            
            # Send acknowledgment to client
            response = {
                "success": True,
                "subscription_id": sub_data["subscription_id"]
            }
            self._send_response_to_client(message.client_id, message.message_id, response)
        
        elif message.type == HubMessage.TYPE_REGISTER_INTERCEPTOR:
            # Register an interceptor
            int_data = message.data
            pattern = int_data["pattern"]
            
            if pattern not in self.interceptors:
                self.interceptors[pattern] = []
            
            # Add interceptor
            interceptor = Interceptor(
                pattern=pattern,
                client_id=message.client_id,
                interceptor_id=int_data["interceptor_id"],
                priority=int_data.get("priority", 0)
            )
            
            self.interceptors[pattern].append(interceptor.to_dict())
            
            # Sort by priority (descending)
            self.interceptors[pattern] = sorted(
                self.interceptors[pattern],
                key=lambda x: x["priority"],
                reverse=True
            )
            
            # Send acknowledgment to client
            response = {
                "success": True,
                "interceptor_id": int_data["interceptor_id"]
            }
            self._send_response_to_client(message.client_id, message.message_id, response)
        
        elif message.type == HubMessage.TYPE_API_REQUEST:
            # Process an API request
            request_data = message.data
            api_path = request_data["path"]
            
            # Look for matching API
            if api_path in self.api_registry:
                api_entry = self.api_registry[api_path]
                
                # Forward request to the registered handler's client
                forward_msg = HubMessage(
                    HubMessage.TYPE_API_REQUEST,
                    request_data,
                    api_entry["client_id"],
                    message.message_id
                )
                
                # Store the original client id for the response
                self._store_request_origin(message.message_id, message.client_id)
                
                # Forward to API handler's client
                self._send_message_to_client(api_entry["client_id"], forward_msg.to_dict())
            else:
                # API not found
                response = ApiResponse(
                    data=None,
                    metadata={"error": f"API not found: {api_path}"},
                    status=ResponseStatus.NOT_FOUND
                )
                
                self._send_response_to_client(message.client_id, message.message_id, response.to_dict())
        
        elif message.type == HubMessage.TYPE_API_RESPONSE:
            # Forward API response to the original requester
            response_data = message.data
            original_client = self._get_request_origin(message.message_id)
            
            if original_client:
                self._send_response_to_client(original_client, message.message_id, response_data)
                self._clear_request_origin(message.message_id)
        
        elif message.type == HubMessage.TYPE_PUBLISH:
            # Process a publish request
            message_data = message.data
            topic = message_data["topic"]
            
            # Check for interceptors first
            intercepted = False
            
            for pattern, interceptors in self.interceptors.items():
                if self._pattern_matches(pattern, topic):
                    for interceptor in interceptors:
                        # Forward to interceptor's client
                        intercept_msg = HubMessage(
                            HubMessage.TYPE_INTERCEPT,
                            message_data,
                            interceptor["client_id"],
                            message.message_id
                        )
                        
                        # Store the original client for the response
                        self._store_request_origin(message.message_id, message.client_id)
                        
                        # Send to interceptor
                        self._send_message_to_client(interceptor["client_id"], intercept_msg.to_dict())
                        intercepted = True
                        break
                    
                    if intercepted:
                        break
            
            # If not intercepted, publish to all subscribers
            if not intercepted:
                for pattern, subs in self.subscriptions.items():
                    if self._pattern_matches(pattern, topic):
                        for sub in subs:
                            # Forward to subscriber's client
                            sub_msg = HubMessage(
                                HubMessage.TYPE_PUBLISH,
                                message_data,
                                sub["client_id"],
                                None  # Don't need a response for publications
                            )
                            
                            self._send_message_to_client(sub["client_id"], sub_msg.to_dict())
                
                # Send acknowledgment to publisher
                response = {
                    "success": True,
                    "intercepted": False
                }
                self._send_response_to_client(message.client_id, message.message_id, response)
    
    def _send_message_to_client(self, client_id, message):
        """Send a message to a client."""
        if client_id in self.clients and self.clients[client_id]["active"]:
            try:
                self.clients[client_id]["queue"].put(message)
                return True
            except Exception as e:
                print(f"Error sending message to client {client_id}: {e}")
        return False
    
    def _send_response_to_client(self, client_id, message_id, response_data):
        """Send a response to a client's request."""
        response_msg = HubMessage(
            HubMessage.TYPE_API_RESPONSE,
            response_data,
            self.hub_id,
            message_id
        )
        return self._send_message_to_client(client_id, response_msg.to_dict())
    
    # Request origin tracking (for request/response correlation)
    def _store_request_origin(self, message_id, client_id):
        """Store the original client for a request."""
        self.manager.dict()[f"origin_{message_id}"] = client_id
    
    def _get_request_origin(self, message_id):
        """Get the original client for a request."""
        return self.manager.dict().get(f"origin_{message_id}")
    
    def _clear_request_origin(self, message_id):
        """Clear the original client for a request."""
        key = f"origin_{message_id}"
        if key in self.manager.dict():
            del self.manager.dict()[key]
    
    def _pattern_matches(self, pattern, topic):
        """Check if a topic matches a pattern."""
        # Simple pattern matching (exact match or wildcard)
        if pattern == topic:
            return True
        
        if pattern.endswith('*'):
            prefix = pattern[:-1]
            return topic.startswith(prefix)
        
        return False
    
    def register_client(self, client_id):
        """Register a new client with the hub."""
        if client_id in self.clients:
            print(f"Client {client_id} already registered")
            return self.clients[client_id]["queue"]
        
        client_queue = Queue()
        self.clients[client_id] = {
            "queue": client_queue,
            "active": True
        }
        print(f"Client {client_id} registered with hub {self.hub_id}")
        return client_queue
    
    def unregister_client(self, client_id):
        """Unregister a client from the hub."""
        if client_id in self.clients:
            self.clients[client_id]["active"] = False
            print(f"Client {client_id} unregistered from hub {self.hub_id}")
            return True
        return False

# ========================
# Client Implementation
# ========================

class ShallowHubClient:
    """Client to connect to the shallow hub."""
    
    def __init__(self, hub, client_id=None):
        """
        Initialize a new shallow hub client.
        
        Args:
            hub: The ShallowHub instance to connect to.
            client_id: Client ID, generated if not provided.
        """
        self.hub = hub
        self.client_id = client_id or str(uuid.uuid4())
        self.client_queue = None
        self.connected = False
        self.running = True
        
        # Callbacks for handling requests
        self.api_handlers = {}
        self.subscription_handlers = {}
        self.interceptor_handlers = {}
        
        # Pending requests waiting for responses
        self.pending_requests = {}
        self.request_lock = threading.Lock()
        
        # Message listening thread
        self.listener_thread = None
    
    def connect(self):
        """Connect to the shallow hub."""
        if self.connected:
            return True
        
        try:
            # Register with hub and get our message queue
            self.client_queue = self.hub.register_client(self.client_id)
            self.connected = True
            
            # Start listener thread
            self.listener_thread = threading.Thread(target=self._message_listener)
            self.listener_thread.daemon = True
            self.listener_thread.start()
            
            print(f"Client {self.client_id} connected to hub {self.hub.hub_id}")
            return True
        except Exception as e:
            print(f"Error connecting to hub: {e}")
            return False
    
    def disconnect(self):
        """Disconnect from the shallow hub."""
        if not self.connected:
            return
        
        self.running = False
        self.hub.unregister_client(self.client_id)
        
        # Wait for listener thread to terminate
        if self.listener_thread and self.listener_thread.is_alive():
            self.listener_thread.join(timeout=2)
        
        self.connected = False
        print(f"Client {self.client_id} disconnected from hub {self.hub.hub_id}")
    
    def _message_listener(self):
        """Listen for messages from the hub."""
        while self.running and self.connected:
            try:
                # Get message with timeout to allow checking if running
                try:
                    message_dict = self.client_queue.get(timeout=1)
                except:
                    continue
                
                # Process the message
                self._process_client_message(message_dict)
            except Exception as e:
                print(f"Error in client message listener: {e}")
    
    def _process_client_message(self, message_dict):
        """Process a message received by the client."""
        message = HubMessage.from_dict(message_dict)
        
        if message.type == HubMessage.TYPE_API_REQUEST:
            # Handle API request
            request_data = message.data
            api_path = request_data["path"]
            
            if api_path in self.api_handlers:
                handler = self.api_handlers[api_path]
                
                # Create API request
                request = ApiRequest.from_dict(request_data)
                
                try:
                    # Call handler
                    response = handler(request)
                    
                    # Send response
                    self._send_api_response(message.message_id, response.to_dict())
                except Exception as e:
                    print(f"Error handling API request {api_path}: {e}")
                    
                    # Send error response
                    error_response = ApiResponse(
                        data=None,
                        metadata={"error": str(e)},
                        status=ResponseStatus.ERROR
                    )
                    self._send_api_response(message.message_id, error_response.to_dict())
            else:
                # API not registered with this client
                print(f"API {api_path} not registered with client {self.client_id}")
                
                error_response = ApiResponse(
                    data=None,
                    metadata={"error": f"API {api_path} not registered with client {self.client_id}"},
                    status=ResponseStatus.NOT_FOUND
                )
                self._send_api_response(message.message_id, error_response.to_dict())
        
        elif message.type == HubMessage.TYPE_API_RESPONSE:
            # Handle API response
            response_data = message.data
            
            with self.request_lock:
                if message.message_id in self.pending_requests:
                    # Get the response handler
                    response_event, response_dict = self.pending_requests[message.message_id]
                    
                    # Store the response
                    response_dict.update(response_data)
                    
                    # Notify the waiting thread
                    response_event.set()
                    
                    # Remove from pending requests
                    del self.pending_requests[message.message_id]
                else:
                    print(f"Received response for unknown request: {message.message_id}")
        
        elif message.type == HubMessage.TYPE_PUBLISH:
            # Handle published message
            message_data = message.data
            topic = message_data["topic"]
            
            # Find matching subscription handlers
            for pattern, handlers in self.subscription_handlers.items():
                if self._pattern_matches(pattern, topic):
                    # Create message object
                    msg = Message.from_dict(message_data)
                    
                    # Call handlers in order of priority
                    for handler in sorted(handlers, key=lambda h: h["priority"], reverse=True):
                        try:
                            handler["callback"](msg)
                        except Exception as e:
                            print(f"Error in subscription handler for {pattern}: {e}")
        
        elif message.type == HubMessage.TYPE_INTERCEPT:
            # Handle interception request
            message_data = message.data
            topic = message_data["topic"]
            
            # Find matching interceptor handlers
            for pattern, handlers in self.interceptor_handlers.items():
                if self._pattern_matches(pattern, topic):
                    # Create message object
                    msg = Message.from_dict(message_data)
                    
                    # Call handlers in order of priority
                    for handler in sorted(handlers, key=lambda h: h["priority"], reverse=True):
                        try:
                            result = handler["callback"](msg)
                            if result is not None:
                                # Send interception result
                                self._send_intercept_result(message.message_id, result)
                                return
                        except Exception as e:
                            print(f"Error in interceptor handler for {pattern}: {e}")
            
            # No interception
            self._send_intercept_result(message.message_id, None)
    
    def _send_message(self, msg_type, data, message_id=None):
        """Send a message to the hub."""
        if not self.connected:
            raise ConnectionError("Not connected to the hub")
        
        message = HubMessage(
            msg_type,
            data,
            self.client_id,
            message_id or str(uuid.uuid4())
        )
        
        self.hub.hub_queue.put(message.to_dict())
        return message.message_id
    
    def _send_api_response(self, message_id, response_data):
        """Send an API response to the hub."""
        return self._send_message(HubMessage.TYPE_API_RESPONSE, response_data, message_id)
    
    def _send_intercept_result(self, message_id, result):
        """Send an interception result to the hub."""
        return self._send_message(
            HubMessage.TYPE_API_RESPONSE,
            {"intercepted": result is not None, "result": result},
            message_id
        )
    
    def _send_request_and_wait(self, msg_type, data, timeout=30):
        """Send a request to the hub and wait for the response."""
        message_id = self._send_message(msg_type, data)
        
        # Create event and dict for the response
        response_event = threading.Event()
        response_dict = {}
        
        # Store in pending requests
        with self.request_lock:
            self.pending_requests[message_id] = (response_event, response_dict)
        
        # Wait for the response
        if not response_event.wait(timeout):
            with self.request_lock:
                if message_id in self.pending_requests:
                    del self.pending_requests[message_id]
            raise TimeoutError(f"Request timed out after {timeout} seconds")
        
        return response_dict
    
    def _pattern_matches(self, pattern, topic):
        """Check if a topic matches a pattern."""
        # Simple pattern matching (exact match or wildcard)
        if pattern == topic:
            return True
        
        if pattern.endswith('*'):
            prefix = pattern[:-1]
            return topic.startswith(prefix)
        
        return False
    
    def register_api(self, path, handler, metadata=None):
        """
        Register an API endpoint with the hub.
        
        Args:
            path: API path to register.
            handler: Function to handle requests to this API.
            metadata: Optional metadata for the API.
            
        Returns:
            True if registration succeeded, False otherwise.
        """
        metadata = metadata or {}
        
        # Store handler locally
        self.api_handlers[path] = handler
        
        # Register with hub
        api_data = {
            "path": path,
            "metadata": metadata,
            "client_id": self.client_id
        }
        
        try:
            response = self._send_request_and_wait(HubMessage.TYPE_REGISTER_API, api_data)
            return response.get("success", False)
        except Exception as e:
            print(f"Error registering API {path}: {e}")
            return False
    
    def call_api(self, path, data=None, metadata=None):
        """
        Call an API endpoint on the hub.
        
        Args:
            path: API path to call.
            data: Request data.
            metadata: Request metadata.
            
        Returns:
            API response.
        """
        metadata = metadata or {}
        
        # Create request
        request_data = {
            "path": path,
            "data": data,
            "metadata": metadata,
            "sender_id": self.client_id
        }
        
        try:
            response_data = self._send_request_and_wait(HubMessage.TYPE_API_REQUEST, request_data)
            
            # Convert status
            status_value = response_data.get("status", "error")
            if isinstance(status_value, str):
                status = ResponseStatus(status_value)
            else:
                status_map = {
                    0: ResponseStatus.SUCCESS,
                    1: ResponseStatus.NOT_FOUND,
                    2: ResponseStatus.ERROR,
                    3: ResponseStatus.INTERCEPTED,
                    4: ResponseStatus.APPROXIMATED
                }
                status = status_map.get(status_value, ResponseStatus.ERROR)
            
            return ApiResponse(
                data=response_data.get("data"),
                metadata=response_data.get("metadata", {}),
                status=status
            )
        except Exception as e:
            print(f"Error calling API {path}: {e}")
            return ApiResponse(
                data=None,
                metadata={"error": str(e)},
                status=ResponseStatus.ERROR
            )
    
    def subscribe(self, pattern, callback, priority=0):
        """
        Subscribe to messages matching a pattern.
        
        Args:
            pattern: Message pattern to match.
            callback: Function to call when a matching message is received.
            priority: Subscription priority.
            
        Returns:
            Subscription ID.
        """
        subscription_id = str(uuid.uuid4())
        
        # Store handler locally
        if pattern not in self.subscription_handlers:
            self.subscription_handlers[pattern] = []
        
        self.subscription_handlers[pattern].append({
            "id": subscription_id,
            "callback": callback,
            "priority": priority
        })
        
        # Sort by priority (descending)
        self.subscription_handlers[pattern].sort(key=lambda x: x["priority"], reverse=True)
        
        # Register with hub
        subscription_data = {
            "pattern": pattern,
            "subscription_id": subscription_id,
            "priority": priority
        }
        
        try:
            response = self._send_request_and_wait(HubMessage.TYPE_SUBSCRIBE, subscription_data)
            if not response.get("success", False):
                print(f"Warning: Subscription registration failed: {response.get('error', 'Unknown error')}")
        except Exception as e:
            print(f"Error registering subscription: {e}")
        
        return subscription_id
    
    def register_interceptor(self, pattern, handler, priority=0):
        """
        Register a message interceptor for a pattern.
        
        Args:
            pattern: Message pattern to intercept.
            handler: Function to handle interception.
            priority: Interceptor priority.
            
        Returns:
            Interceptor ID.
        """
        interceptor_id = str(uuid.uuid4())
        
        # Store handler locally
        if pattern not in self.interceptor_handlers:
            self.interceptor_handlers[pattern] = []
        
        self.interceptor_handlers[pattern].append({
            "id": interceptor_id,
            "callback": handler,
            "priority": priority
        })
        
        # Sort by priority (descending)
        self.interceptor_handlers[pattern].sort(key=lambda x: x["priority"], reverse=True)
        
        # Register with hub
        interceptor_data = {
            "pattern": pattern,
            "interceptor_id": interceptor_id,
            "priority": priority
        }
        
        try:
            response = self._send_request_and_wait(HubMessage.TYPE_REGISTER_INTERCEPTOR, interceptor_data)
            if not response.get("success", False):
                print(f"Warning: Interceptor registration failed: {response.get('error', 'Unknown error')}")
        except Exception as e:
            print(f"Error registering interceptor: {e}")
        
        return interceptor_id
    
    def publish(self, topic, data, metadata=None):
        """
        Publish a message to the hub.
        
        Args:
            topic: Message topic.
            data: Message data.
            metadata: Message metadata.
            
        Returns:
            Result from any interceptors, or None.
        """
        metadata = metadata or {}
        
        # Create message
        message_data = {
            "topic": topic,
            "data": data,
            "metadata": metadata,
            "sender_id": self.client_id,
            "timestamp": int(time.time() * 1000)
        }
        
        try:
            response = self._send_request_and_wait(HubMessage.TYPE_PUBLISH, message_data)
            
            if response.get("intercepted", False):
                return response.get("result")
            
            return None
        except Exception as e:
            print(f"Error publishing message to {topic}: {e}")
            return None

# ========================
# Node Implementation
# ========================

class Node:
    """A node in the virtual network that connects to the hub system."""
    
    def __init__(self, hub, node_id=None):
        """Initialize a new node with an optional ID."""
        self.node_id = node_id or str(uuid.uuid4())
        self.hub_client = ShallowHubClient(hub, self.node_id)
        
    def connect(self):
        """Connect to the hub."""
        return self.hub_client.connect()
    
    def disconnect(self):
        """Disconnect from the hub."""
        self.hub_client.disconnect()
        
    def register_api(self, path, handler, metadata=None):
        """Register an API endpoint with the node."""
        return self.hub_client.register_api(path, handler, metadata)
        
    def call_api(self, path, data=None, metadata=None):
        """Call an API endpoint on the network."""
        return self.hub_client.call_api(path, data, metadata)
        
    def subscribe(self, pattern, callback, priority=0):
        """Subscribe to messages matching a pattern."""
        return self.hub_client.subscribe(pattern, callback, priority)
        
    def register_interceptor(self, pattern, handler, priority=0):
        """Register a message interceptor for a pattern."""
        return self.hub_client.register_interceptor(pattern, handler, priority)
        
    def publish(self, topic, data, metadata=None):
        """Publish a message to the network."""
        return self.hub_client.publish(topic, data, metadata)

# ========================
# Helper Functions
# ========================

def create_shallow_hub(scope=HubScope.PROCESS):
    """Create a new shallow hub for in-process communication."""
    return ShallowHub(scope)

def create_node(hub, node_id=None):
    """Create a new node connected to the shallow hub."""
    node = Node(hub, node_id)
    node.connect()
    return node

# ========================
# Usage Example
# ========================

def example():
    """Example usage of the shallow hub."""
    # Create a shallow hub
    hub = create_shallow_hub()
    
    try:
        # Create two nodes
        node1 = create_node(hub, "node1")
        node2 = create_node(hub, "node2")
        
        # Register APIs on each node
        node1.register_api("/node1/greeting", lambda request: ApiResponse(
            data="Hello from Node 1!",
            metadata={},
            status=ResponseStatus.SUCCESS
        ))
        
        node2.register_api("/node2/greeting", lambda request: ApiResponse(
            data="Hello from Node 2!",
            metadata={},
            status=ResponseStatus.SUCCESS
        ))
        
        # Node 1 calls Node 2's API
        response1 = node1.call_api("/node2/greeting")
        print(f"Node 1 received: {response1.data}")
        
        # Node 2 calls Node 1's API
        response2 = node2.call_api("/node1/greeting")
        print(f"Node 2 received: {response2.data}")
        
        # Set up message subscription
        messages_received = []
        
        def message_handler(message):
            print(f"Received message: {message.topic} -> {message.data}")
            messages_received.append(message.data)
        
        node2.subscribe("test/*", message_handler)
        
        # Publish a message
        node1.publish("test/hello", "Hello from Node 1!")
        
        # Wait a bit for the message to be processed
        time.sleep(1)
        
        print(f"Messages received: {messages_received}")
        
        # Test with a more complex example
        # Register an echo API
        def echo_handler(request):
            return ApiResponse(
                data=f"Echo: {request.data}",
                metadata=request.metadata,
                status=ResponseStatus.SUCCESS
            )
        
        node1.register_api("/echo", echo_handler)
        
        # Call the echo API with data
        response = node2.call_api("/echo", "Testing echo")
        print(f"Echo response: {response.data}")
        
        # Test interception
        def intercept_handler(message):
            if "intercept" in message.data:
                return f"Intercepted: {message.data}"
            return None
        
        node2.register_interceptor("test/*", intercept_handler)
        
        # Publish a message that should be intercepted
        result = node1.publish("test/intercept", "Please intercept this message")
        print(f"Interception result: {result}")
        
        # Publish a message that should not be intercepted
        result = node1.publish("test/normal", "Normal message")
        print(f"Normal publish result: {result}")
        
    finally:
        # Disconnect nodes and shut down hub
        if 'node1' in locals():
            node1.disconnect()
        if 'node2' in locals():
            node2.disconnect()
        
        hub.shutdown()

if __name__ == "__main__":
    example()