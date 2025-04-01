# message_system.py - Enhanced VirtualNode with message interception

import uuid
import time
import inspect
import functools
from typing import Dict, List, Callable, Optional, Any, TypeVar, Generic, Union, Type

# Type variables for generic handling
T = TypeVar('T')
R = TypeVar('R')
A = TypeVar('A')

class Message(Generic[T]):
    """Typed message container"""
    
    def __init__(self, topic: str, data: T, metadata: Dict[str, str] = None):
        self.topic = topic
        self.data = data
        self.metadata = metadata or {}
        self.sender_id = str(uuid.uuid4())
        self.timestamp = time.time()

class MethodContext:
    """Context for method interception"""
    
    def __init__(self, instance: Any, method_name: str, args: Any, kwargs: Any):
        self.instance = instance
        self.method_name = method_name
        self.args = args
        self.kwargs = kwargs
        self.timestamp = time.time()

class InterceptorEntry(Generic[T, R]):
    """Entry for a message interceptor"""
    
    def __init__(self, handler: Callable[[Message[T]], Optional[R]], priority: int):
        self.handler = handler
        self.priority = priority
        self.id = str(uuid.uuid4())

class MethodInterceptorEntry(Generic[T, R]):
    """Entry for a method interceptor"""
    
    def __init__(self, handler: Callable[[MethodContext], Optional[R]], priority: int):
        self.handler = handler
        self.priority = priority
        self.id = str(uuid.uuid4())

class EnhancedVirtualNode:
    """Enhanced VirtualNode with message passing and interception capabilities"""
    
    def __init__(self):
        self.node_id = str(uuid.uuid4())
        self.thread_hub = EnhancedThreadHub.get_instance()
        self._connect_to_process_hub()
    
    def register_api(self, path: str, handler: Callable, **metadata) -> None:
        """Register an API endpoint (from original VirtualNode)"""
        self.thread_hub.register_api(path, handler, metadata)
    
    def register_interceptor(self, topic: str, handler: Callable[[Message], Optional[Any]], 
                             priority: int = 0) -> str:
        """Register a message interceptor for a specific topic"""
        return self.thread_hub.register_interceptor(topic, handler, priority)
    
    def register_method_interceptor(self, cls: Type, method_name: str, 
                                    handler: Callable[[MethodContext], Optional[Any]], 
                                    priority: int = 0) -> str:
        """Register a method interceptor"""
        return self.thread_hub.register_method_interceptor(cls, method_name, handler, priority)
    
    def publish(self, topic: str, data: Any, metadata: Dict[str, str] = None) -> Optional[Any]:
        """Publish a message, allowing for interception"""
        message = Message(topic, data, metadata)
        return self.thread_hub.publish_message(message)
    
    def create_proxy(self, instance: Any, method_name: str) -> Callable:
        """Create a proxy for a method that can be intercepted"""
        return self.thread_hub.create_method_proxy(instance, method_name)
    
    def intercept(self, cls: Type = None, method_name: str = None):
        """Decorator to make a method interceptable"""
        def decorator(func):
            @functools.wraps(func)
            def wrapper(self, *args, **kwargs):
                # Create method context
                ctx = MethodContext(self, func.__name__, args, kwargs)
                
                # Try to intercept
                result = EnhancedThreadHub.get_instance().try_intercept_method(
                    type(self), func.__name__, ctx)
                
                if result is not None:
                    return result
                
                # If not intercepted, call original method
                return func(self, *args, **kwargs)
            return wrapper
        return decorator

class EnhancedThreadHub:
    """Enhanced ThreadHub with message passing and interception"""
    
    # Class variable to ensure singleton-per-thread behavior
    _instance = None
    
    @classmethod
    def get_instance(cls):
        """Get or create the thread hub for the current thread"""
        if cls._instance is None:
            cls._instance = EnhancedThreadHub()
        return cls._instance
    
    def __init__(self):
        # Original ThreadHub attributes
        self.apis = {}
        self.parent_hub = None
        
        # New message system attributes
        self.message_interceptors = {}  # topic -> [(priority, interceptor)]
        self.method_interceptors = {}   # (cls, method_name) -> [(priority, interceptor)]
    
    def register_interceptor(self, topic: str, handler: Callable[[Message], Optional[Any]], 
                            priority: int = 0) -> str:
        """Register a message interceptor for a specific topic"""
        interceptor = InterceptorEntry(handler, priority)
        
        if topic not in self.message_interceptors:
            self.message_interceptors[topic] = []
            
        self.message_interceptors[topic].append((priority, interceptor))
        # Sort by priority (highest first)
        self.message_interceptors[topic].sort(key=lambda x: -x[0])
        
        return interceptor.id
    
    def register_method_interceptor(self, cls: Type, method_name: str, 
                                   handler: Callable[[MethodContext], Optional[Any]], 
                                   priority: int = 0) -> str:
        """Register a method interceptor"""
        interceptor = MethodInterceptorEntry(handler, priority)
        key = (cls, method_name)
        
        if key not in self.method_interceptors:
            self.method_interceptors[key] = []
            
        self.method_interceptors[key].append((priority, interceptor))
        # Sort by priority (highest first)
        self.method_interceptors[key].sort(key=lambda x: -x[0])
        
        return interceptor.id
    
    def publish_message(self, message: Message) -> Optional[Any]:
        """Publish a message and allow for interception"""
        # Check for interceptors with exact topic match
        if message.topic in self.message_interceptors:
            for _, interceptor in self.message_interceptors[message.topic]:
                result = interceptor.handler(message)
                if result is not None:
                    return result
        
        # Check for wildcard patterns
        for topic, interceptors in self.message_interceptors.items():
            if topic.endswith('*') and message.topic.startswith(topic[:-1]):
                for _, interceptor in interceptors:
                    result = interceptor.handler(message)
                    if result is not None:
                        return result
        
        # If not intercepted and we have a parent, try there
        if self.parent_hub:
            return self.parent_hub.publish_message(message)
        
        return None
    
    def try_intercept_method(self, cls: Type, method_name: str, ctx: MethodContext) -> Optional[Any]:
        """Try to intercept a method call"""
        # Check for direct class match
        key = (cls, method_name)
        if key in self.method_interceptors:
            for _, interceptor in self.method_interceptors[key]:
                result = interceptor.handler(ctx)
                if result is not None:
                    return result
        
        # Check for parent classes (inheritance)
        for parent_cls in cls.__mro__[1:]:  # Skip the class itself
            key = (parent_cls, method_name)
            if key in self.method_interceptors:
                for _, interceptor in self.method_interceptors[key]:
                    result = interceptor.handler(ctx)
                    if result is not None:
                        return result
        
        # If not intercepted and we have a parent hub, try there
        if self.parent_hub:
            return self.parent_hub.try_intercept_method(cls, method_name, ctx)
        
        return None
    
    def create_method_proxy(self, instance: Any, method_name: str) -> Callable:
        """Create a proxy for a method that can be intercepted"""
        original_method = getattr(instance, method_name)
        
        @functools.wraps(original_method)
        def proxy(*args, **kwargs):
            # Create method context
            ctx = MethodContext(instance, method_name, args, kwargs)
            
            # Try to intercept
            result = self.try_intercept_method(type(instance), method_name, ctx)
            
            if result is not None:
                return result
            
            # If not intercepted, call original method
            return original_method(*args, **kwargs)
            
        return proxy


# Example usage of interception mechanism
def example_usage():
    # Create virtual node
    node = EnhancedVirtualNode()
    
    # Regular API registration
    def file_search(request):
        query = request['query']
        return {'results': [f'file:{query}_result1', f'file:{query}_result2']}
    
    node.register_api('/search/files', file_search)
    
    # Define a web search interceptor
    def web_search_interceptor(message):
        data = message.data
        if 'web_required' in message.metadata and message.metadata['web_required'] == 'true':
            # This is a higher priority web search
            return {'results': [f'web:{data["query"]}_result1', f'web:{data["query"]}_result2']}
        return None  # Not intercepted
    
    # Register as a high-priority interceptor
    node.register_interceptor('/search/files', web_search_interceptor, priority=10)
    
    # Now when someone calls with web_required metadata, it will use web search instead
    result = node.publish('/search/files', {'query': 'example'}, {'web_required': 'true'})
    # result would be the web search results, not file search
    
    # Method interception example
    class FileSearcher:
        @node.intercept()
        def search(self, query):
            return [f'file:{query}_result1', f'file:{query}_result2']
    
    # Register a method interceptor
    def web_method_interceptor(ctx):
        # ctx.instance is the FileSearcher instance
        # ctx.args[0] is the query
        query = ctx.args[0]
        
        # Check if this is a web-related query
        if query.startswith('web:'):
            return [f'web:{query[4:]}_result1', f'web:{query[4:]}_result2']
        return None  # Not intercepted
    
    # Register the method interceptor
    node.register_method_interceptor(FileSearcher, 'search', web_method_interceptor, priority=10)
    
    # Create FileSearcher and test
    searcher = FileSearcher()
    
    # This will use the original method
    result1 = searcher.search('example')  # ['file:example_result1', 'file:example_result2']
    
    # This will be intercepted
    result2 = searcher.search('web:example')  # ['web:example_result1', 'web:example_result2']