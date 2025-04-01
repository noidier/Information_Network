# node_library.py - Python Node Library for the Virtual Network

import uuid
import time
import json
import threading
import os
import asyncio
import inspect
from typing import Any, Dict, List, Optional, Callable, TypeVar, Generic, Union, Type, cast
from functools import wraps
from dataclasses import dataclass
from enum import Enum

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

@dataclass
class InterceptContext:
    """Context for method interception."""
    instance: Any
    method_name: str
    args: tuple
    kwargs: dict
    original_method: Callable

# ========================
# Node Implementation
# ========================

class Node:
    """A node in the virtual network that connects to the hub system."""
    
    def __init__(self, node_id: Optional[str] = None):
        """Initialize a new node with an optional ID."""
        self.node_id = node_id or str(uuid.uuid4())
        self.hub_connector = HubConnector()
        
        # Local interceptors for when no hub is available
        self._local_interceptors = {}
        self._local_method_interceptors = {}
        
    def register_api(self, path: str, handler: Callable[[ApiRequest], ApiResponse], 
                    metadata: Optional[Dict[str, str]] = None) -> None:
        """Register an API endpoint with the node."""
        if metadata is None:
            metadata = {}
            
        # Ensure handler is properly wrapped
        @wraps(handler)
        def wrapped_handler(request: ApiRequest) -> ApiResponse:
            return handler(request)
            
        self.hub_connector.register_api(path, wrapped_handler, metadata)
        
    def call_api(self, path: str, data: Any = None, 
                metadata: Optional[Dict[str, str]] = None) -> ApiResponse:
        """Call an API endpoint on the network."""
        if metadata is None:
            metadata = {}
            
        request = ApiRequest(
            path=path,
            data=data,
            metadata=metadata,
            sender_id=self.node_id
        )
        
        return self.hub_connector.handle_request(request)
        
    def subscribe(self, pattern: str, callback: Callable[[Message], Optional[Any]], 
                priority: int = 0) -> str:
        """Subscribe to messages matching a pattern."""
        subscription_id = str(uuid.uuid4())
        
        @wraps(callback)
        def wrapped_callback(message: Message) -> Optional[Any]:
            return callback(message)
            
        self.hub_connector.subscribe(pattern, wrapped_callback, priority)
        return subscription_id
        
    def publish(self, topic: str, data: Any, 
                metadata: Optional[Dict[str, str]] = None) -> Optional[Any]:
        """Publish a message to the network."""
        if metadata is None:
            metadata = {}
            
        message = Message(
            topic=topic,
            data=data,
            metadata=metadata,
            sender_id=self.node_id
        )
        
        return self.hub_connector.publish(message)
        
    def register_interceptor(self, pattern: str, 
                            handler: Callable[[Message], Optional[Any]],
                            priority: int = 0) -> str:
        """Register a message interceptor for a pattern."""
        interceptor_id = str(uuid.uuid4())
        
        @wraps(handler)
        def wrapped_handler(message: Message) -> Optional[Any]:
            return handler(message)
            
        self.hub_connector.register_interceptor(pattern, wrapped_handler, priority)
        return interceptor_id
        
    def register_method_interceptor(self, class_type: Type, method_name: str,
                                  handler: Callable[[InterceptContext], Optional[Any]],
                                  priority: int = 0) -> str:
        """Register a method interceptor for a class method."""
        interceptor_id = str(uuid.uuid4())
        
        if class_type not in self._local_method_interceptors:
            self._local_method_interceptors[class_type] = {}
            
        if method_name not in self._local_method_interceptors[class_type]:
            self._local_method_interceptors[class_type][method_name] = []
            
        self._local_method_interceptors[class_type][method_name].append({
            'id': interceptor_id,
            'handler': handler,
            'priority': priority
        })
        
        # Sort by priority (descending)
        self._local_method_interceptors[class_type][method_name].sort(
            key=lambda x: x['priority'], reverse=True
        )
        
        return interceptor_id
        
    def intercept(self, method_or_class=None):
        """Decorator to make a method interceptable."""
        def decorator(func_or_class):
            if inspect.isclass(func_or_class):
                # Apply to all methods in class
                for name, method in inspect.getmembers(func_or_class, predicate=inspect.isfunction):
                    if not name.startswith('_'):  # Skip private methods
                        setattr(func_or_class, name, self._make_interceptable(method))
                return func_or_class
            else:
                # Apply to a single function
                return self._make_interceptable(func_or_class)
                
        if method_or_class is None:
            return decorator
        return decorator(method_or_class)
        
    def _make_interceptable(self, method):
        """Make a method interceptable by wrapping it with a proxy."""
        @wraps(method)
        def wrapper(instance, *args, **kwargs):
            # Build context
            context = InterceptContext(
                instance=instance,
                method_name=method.__name__,
                args=args,
                kwargs=kwargs,
                original_method=method
            )
            
            # Try local interception first
            result = self._try_intercept_method_locally(instance, method.__name__, context)
            if result is not None:
                return result
                
            # Try hub interception next
            result = self.hub_connector.try_intercept_method(instance, method.__name__, context)
            if result is not None:
                return result
                
            # If not intercepted, call the original method
            return method(instance, *args, **kwargs)
            
        return wrapper
        
    def _try_intercept_method_locally(self, instance, method_name, context):
        """Try to intercept a method call using local interceptors."""
        class_type = instance.__class__
        
        if class_type in self._local_method_interceptors:
            if method_name in self._local_method_interceptors[class_type]:
                for interceptor in self._local_method_interceptors[class_type][method_name]:
                    result = interceptor['handler'](context)
                    if result is not None:
                        return result
                        
        return None
        
    def create_proxy(self, target, method_name):
        """Create a proxy for a specific method that can be intercepted."""
        original_method = getattr(target, method_name)
        
        @wraps(original_method)
        def proxy(*args, **kwargs):
            # Build context
            context = InterceptContext(
                instance=target,
                method_name=method_name,
                args=args,
                kwargs=kwargs,
                original_method=original_method
            )
            
            # Try local interception first
            result = self._try_intercept_method_locally(target, method_name, context)
            if result is not None:
                return result
                
            # Try hub interception next
            result = self.hub_connector.try_intercept_method(target, method_name, context)
            if result is not None:
                return result
                
            # If not intercepted, call the original method
            return original_method(*args, **kwargs)
            
        return proxy

# ========================
# Hub Connector Implementation
# ========================

class HubConnector:
    """Connector to the Rust hub implementation."""
    
    def __init__(self):
        """Initialize the hub connector."""
        # In a real implementation, would connect to the Rust hub
        # For pseudocode, we'll use a local dict to simulate the hub
        self._local_registry = {}
        self._local_subscriptions = {}
        self._local_interceptors = {}
        self._local_method_interceptors = {}
        
    def register_api(self, path, handler, metadata):
        """Register an API endpoint with the hub."""
        # In a real implementation, would call the Rust hub
        # For pseudocode, store locally
        self._local_registry[path] = {
            'handler': handler,
            'metadata': metadata
        }
        
    def handle_request(self, request):
        """Handle an API request."""
        # In a real implementation, would call the Rust hub
        # For pseudocode, handle locally
        
        # Check for interception
        # (not implemented in this simplified version)
        
        # Check local registry
        if request.path in self._local_registry:
            api = self._local_registry[request.path]
            return api['handler'](request)
            
        # Try fallback
        for path, entry in self._local_registry.items():
            if 'fallback' in entry['metadata'] and entry['metadata']['fallback'] == request.path:
                fallback_request = ApiRequest(
                    path=entry['metadata']['fallback'],
                    data=request.data,
                    metadata={**request.metadata, 'original_path': request.path},
                    sender_id=request.sender_id
                )
                return self.handle_request(fallback_request)
                
        # Not found
        return ApiResponse(
            data=None,
            metadata={},
            status=ResponseStatus.NOT_FOUND
        )
        
    def subscribe(self, pattern, callback, priority):
        """Subscribe to messages matching a pattern."""
        # In a real implementation, would call the Rust hub
        # For pseudocode, store locally
        if pattern not in self._local_subscriptions:
            self._local_subscriptions[pattern] = []
            
        self._local_subscriptions[pattern].append({
            'callback': callback,
            'priority': priority
        })
        
        # Sort by priority (descending)
        self._local_subscriptions[pattern].sort(
            key=lambda x: x['priority'], reverse=True
        )
        
    def register_interceptor(self, pattern, handler, priority):
        """Register a message interceptor for a pattern."""
        # In a real implementation, would call the Rust hub
        # For pseudocode, store locally
        if pattern not in self._local_interceptors:
            self._local_interceptors[pattern] = []
            
        self._local_interceptors[pattern].append({
            'handler': handler,
            'priority': priority
        })
        
        # Sort by priority (descending)
        self._local_interceptors[pattern].sort(
            key=lambda x: x['priority'], reverse=True
        )
        
    def publish(self, message):
        """Publish a message and allow for interception."""
        # In a real implementation, would call the Rust hub
        # For pseudocode, handle locally
        
        # Check for interceptors
        for pattern, interceptors in self._local_interceptors.items():
            if self._pattern_matches(pattern, message.topic):
                for interceptor in interceptors:
                    result = interceptor['handler'](message)
                    if result is not None:
                        return result
                        
        # Notify subscribers
        for pattern, subscribers in self._local_subscriptions.items():
            if self._pattern_matches(pattern, message.topic):
                for subscriber in subscribers:
                    subscriber['callback'](message)
                    
        return None
        
    def try_intercept_method(self, instance, method_name, context):
        """Try to intercept a method call."""
        # In a real implementation, would call the Rust hub
        # For pseudocode, return None (not intercepted)
        return None
        
    def _pattern_matches(self, pattern, topic):
        """Check if a topic matches a pattern."""
        if pattern == topic:
            return True
            
        if pattern.endswith('*') and topic.startswith(pattern[:-1]):
            return True
            
        return False

# ========================
# Helper Functions
# ========================

def create_node(node_id=None):
    """Create a new node in the virtual network."""
    return Node(node_id)

# ========================
# Usage Examples
# ========================

def register_with_fallback(node, primary_path, fallback_path, handler):
    """Register an API endpoint with a fallback."""
    # Register primary
    node.register_api(primary_path, handler, {'fallback': fallback_path})
    
    # Also register at fallback path
    node.register_api(fallback_path, handler, {})

def register_static(node, path, static_response):
    """Register a static mock API."""
    def static_handler(request):
        return ApiResponse(
            data=static_response,
            metadata={'is_static': 'true'},
            status=ResponseStatus.SUCCESS
        )
        
    node.register_api(path, static_handler, {'is_static': 'true'})

def setup_file_web_interception(node):
    """Set up interception between file and web APIs."""
    # Register file search API
    def file_search_handler(request):
        query = request.data.get('query', '')
        return ApiResponse(
            data={'results': [f'file:{query}_result1', f'file:{query}_result2']},
            metadata={},
            status=ResponseStatus.SUCCESS
        )
        
    node.register_api('/search/files', file_search_handler, {})
    
    # Set up interceptor for web search
    def web_search_interceptor(message):
        if message.metadata.get('source') == 'web' or 'web:' in message.data.get('query', ''):
            query = message.data.get('query', '')
            return {'results': [f'web:{query}_result1', f'web:{query}_result2']}
        return None  # Not intercepted
        
    node.register_interceptor('/search/files', web_search_interceptor, 10)  # High priority

# ========================
# Main Example
# ========================

if __name__ == "__main__":
    # Create a node
    node = create_node()
    
    # Register an API
    def echo_handler(request):
        return ApiResponse(
            data=request.data,
            metadata=request.metadata,
            status=ResponseStatus.SUCCESS
        )
    
    node.register_api('/echo', echo_handler, {})
    
    # Call the API
    response = node.call_api('/echo', {'message': 'Hello, World!'})
    print(f"Response: {response.data}")
    
    # Set up file/web search interception example
    setup_file_web_interception(node)
    
    # Test interception
    print("\nTesting interception:")
    
    # Regular file search
    response = node.call_api('/search/files', {'query': 'test'})
    print(f"File search: {response.data}")
    
    # Web search (intercepted)
    response = node.call_api('/search/files', {'query': 'web:test'}, {'source': 'web'})
    print(f"Web search: {response.data}")
    
    # Test method interception
    print("\nTesting method interception:")
    
    # Define a class with interceptable methods
    class FileSearcher:
        @node.intercept
        def search(self, query):
            return [f'file:{query}_result1', f'file:{query}_result2']
    
    # Register method interceptor
    def web_method_interceptor(context):
        query = context.args[0]
        if query.startswith('web:'):
            return [f'web:{query}_result1', f'web:{query}_result2']
        return None  # Not intercepted
        
    node.register_method_interceptor(FileSearcher, 'search', web_method_interceptor, 10)
    
    # Test method interception
    searcher = FileSearcher()
    print(f"Regular search: {searcher.search('test')}")
    print(f"Web search: {searcher.search('web:test')}")