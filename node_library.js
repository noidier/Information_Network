// node_library.js - JavaScript Node Library for the Virtual Network

// ========================
// Data Structures
// ========================

/**
 * Hub scope levels
 * @enum {string}
 */
const HubScope = {
  THREAD: 'thread',
  PROCESS: 'process',
  MACHINE: 'machine',
  NETWORK: 'network'
};

/**
 * API response status
 * @enum {string}
 */
const ResponseStatus = {
  SUCCESS: 'success',
  NOT_FOUND: 'not_found',
  ERROR: 'error',
  INTERCEPTED: 'intercepted',
  APPROXIMATED: 'approximated'
};

/**
 * Message with typed data
 */
class Message {
  /**
   * Create a new message
   * @param {string} topic - Message topic
   * @param {any} data - Message data
   * @param {Object<string,string>} metadata - Message metadata
   * @param {string} senderId - ID of the sender
   */
  constructor(topic, data, metadata = {}, senderId = null) {
    this.topic = topic;
    this.data = data;
    this.metadata = metadata || {};
    this.senderId = senderId;
    this.timestamp = Date.now();
  }
}

/**
 * Request to an API endpoint
 */
class ApiRequest {
  /**
   * Create a new API request
   * @param {string} path - API path
   * @param {any} data - Request data
   * @param {Object<string,string>} metadata - Request metadata
   * @param {string} senderId - ID of the sender
   */
  constructor(path, data, metadata = {}, senderId = null) {
    this.path = path;
    this.data = data;
    this.metadata = metadata || {};
    this.senderId = senderId;
  }
}

/**
 * Response from an API endpoint
 */
class ApiResponse {
  /**
   * Create a new API response
   * @param {any} data - Response data
   * @param {Object<string,string>} metadata - Response metadata
   * @param {string} status - Response status
   */
  constructor(data, metadata = {}, status = ResponseStatus.SUCCESS) {
    this.data = data;
    this.metadata = metadata || {};
    this.status = status;
  }
}

/**
 * Context for method interception
 */
class InterceptContext {
  /**
   * Create a new interception context
   * @param {Object} instance - Instance the method is called on
   * @param {string} methodName - Name of the method
   * @param {Array} args - Method arguments
   * @param {Function} originalMethod - Original method implementation
   */
  constructor(instance, methodName, args, originalMethod) {
    this.instance = instance;
    this.methodName = methodName;
    this.args = args;
    this.originalMethod = originalMethod;
  }
}

// ========================
// Node Implementation
// ========================

/**
 * A node in the virtual network that connects to the hub system
 */
class Node {
  /**
   * Create a new node
   * @param {string} [nodeId] - Optional node ID
   */
  constructor(nodeId = null) {
    this.nodeId = nodeId || generateUuid();
    this.hubConnector = new HubConnector();
    
    // Local interceptors for when no hub is available
    this._localInterceptors = new Map();
    this._localMethodInterceptors = new Map();
  }
  
  /**
   * Register an API endpoint with the node
   * @param {string} path - API path
   * @param {Function} handler - API handler function
   * @param {Object<string,string>} [metadata] - API metadata
   */
  registerApi(path, handler, metadata = {}) {
    // Ensure handler is properly wrapped
    const wrappedHandler = (request) => {
      return handler(request);
    };
    
    this.hubConnector.registerApi(path, wrappedHandler, metadata);
  }
  
  /**
   * Call an API endpoint on the network
   * @param {string} path - API path
   * @param {any} [data] - Request data
   * @param {Object<string,string>} [metadata] - Request metadata
   * @returns {ApiResponse} Response from the API
   */
  callApi(path, data = null, metadata = {}) {
    const request = new ApiRequest(
      path,
      data,
      metadata,
      this.nodeId
    );
    
    return this.hubConnector.handleRequest(request);
  }
  
  /**
   * Subscribe to messages matching a pattern
   * @param {string} pattern - Message pattern to match
   * @param {Function} callback - Callback function
   * @param {number} [priority] - Subscription priority
   * @returns {string} Subscription ID
   */
  subscribe(pattern, callback, priority = 0) {
    const subscriptionId = generateUuid();
    
    // Wrap callback
    const wrappedCallback = (message) => {
      return callback(message);
    };
    
    this.hubConnector.subscribe(pattern, wrappedCallback, priority);
    return subscriptionId;
  }
  
  /**
   * Publish a message to the network
   * @param {string} topic - Message topic
   * @param {any} data - Message data
   * @param {Object<string,string>} [metadata] - Message metadata
   * @returns {any} Result from any interceptors
   */
  publish(topic, data, metadata = {}) {
    const message = new Message(
      topic,
      data,
      metadata,
      this.nodeId
    );
    
    return this.hubConnector.publish(message);
  }
  
  /**
   * Register a message interceptor for a pattern
   * @param {string} pattern - Message pattern to intercept
   * @param {Function} handler - Interceptor handler function
   * @param {number} [priority] - Interceptor priority
   * @returns {string} Interceptor ID
   */
  registerInterceptor(pattern, handler, priority = 0) {
    const interceptorId = generateUuid();
    
    // Wrap handler
    const wrappedHandler = (message) => {
      return handler(message);
    };
    
    this.hubConnector.registerInterceptor(pattern, wrappedHandler, priority);
    return interceptorId;
  }
  
  /**
   * Register a method interceptor for a class method
   * @param {Function} classType - Class constructor
   * @param {string} methodName - Method name
   * @param {Function} handler - Interceptor handler
   * @param {number} [priority] - Interceptor priority
   * @returns {string} Interceptor ID
   */
  registerMethodInterceptor(classType, methodName, handler, priority = 0) {
    const interceptorId = generateUuid();
    
    if (!this._localMethodInterceptors.has(classType)) {
      this._localMethodInterceptors.set(classType, new Map());
    }
    
    const classMethods = this._localMethodInterceptors.get(classType);
    
    if (!classMethods.has(methodName)) {
      classMethods.set(methodName, []);
    }
    
    const methodInterceptors = classMethods.get(methodName);
    
    methodInterceptors.push({
      id: interceptorId,
      handler,
      priority
    });
    
    // Sort by priority (descending)
    methodInterceptors.sort((a, b) => b.priority - a.priority);
    
    return interceptorId;
  }
  
  /**
   * Create a decorator to make methods interceptable
   * @param {Function|Object} targetOrOptions - Target class or options
   * @param {string} [propertyKey] - Property key (for property decorators)
   * @param {PropertyDescriptor} [descriptor] - Property descriptor (for method decorators)
   * @returns {Function|void} Decorator function or void
   */
  intercept(targetOrOptions, propertyKey, descriptor) {
    // Direct usage as method decorator
    if (propertyKey && descriptor) {
      const originalMethod = descriptor.value;
      descriptor.value = this._makeInterceptable(originalMethod);
      return descriptor;
    }
    
    // Usage as class decorator
    if (typeof targetOrOptions === 'function') {
      const classConstructor = targetOrOptions;
      const proto = classConstructor.prototype;
      
      // Apply to all methods
      Object.getOwnPropertyNames(proto).forEach(key => {
        if (key !== 'constructor' && typeof proto[key] === 'function') {
          const originalMethod = proto[key];
          proto[key] = this._makeInterceptable(originalMethod);
        }
      });
      
      return classConstructor;
    }
    
    // Usage as function that returns a decorator
    return (target, key, descriptor) => {
      if (key && descriptor) {
        // Method decorator
        const originalMethod = descriptor.value;
        descriptor.value = this._makeInterceptable(originalMethod);
        return descriptor;
      }
      
      // Class decorator
      if (typeof target === 'function') {
        const classConstructor = target;
        const proto = classConstructor.prototype;
        
        // Apply to all methods
        Object.getOwnPropertyNames(proto).forEach(key => {
          if (key !== 'constructor' && typeof proto[key] === 'function') {
            const originalMethod = proto[key];
            proto[key] = this._makeInterceptable(originalMethod);
          }
        });
        
        return classConstructor;
      }
    };
  }
  
  /**
   * Make a method interceptable by wrapping it with a proxy
   * @param {Function} method - Original method
   * @returns {Function} Interceptable proxy method
   * @private
   */
  _makeInterceptable(method) {
    const self = this;
    
    return function(...args) {
      const instance = this;
      
      // Build context
      const context = new InterceptContext(
        instance,
        method.name,
        args,
        method
      );
      
      // Try local interception first
      const localResult = self._tryInterceptMethodLocally(instance, method.name, context);
      if (localResult !== undefined) {
        return localResult;
      }
      
      // Try hub interception next
      const hubResult = self.hubConnector.tryInterceptMethod(instance, method.name, context);
      if (hubResult !== undefined) {
        return hubResult;
      }
      
      // If not intercepted, call the original method
      return method.apply(instance, args);
    };
  }
  
  /**
   * Try to intercept a method call using local interceptors
   * @param {Object} instance - Object instance
   * @param {string} methodName - Method name
   * @param {InterceptContext} context - Interception context
   * @returns {any} Interception result or undefined
   * @private
   */
  _tryInterceptMethodLocally(instance, methodName, context) {
    const classType = instance.constructor;
    
    if (this._localMethodInterceptors.has(classType)) {
      const classMethods = this._localMethodInterceptors.get(classType);
      
      if (classMethods.has(methodName)) {
        const interceptors = classMethods.get(methodName);
        
        for (const interceptor of interceptors) {
          const result = interceptor.handler(context);
          if (result !== undefined) {
            return result;
          }
        }
      }
    }
    
    return undefined;
  }
  
  /**
   * Create a proxy for a specific method that can be intercepted
   * @param {Object} target - Target object
   * @param {string} methodName - Method name
   * @returns {Function} Proxy method
   */
  createProxy(target, methodName) {
    const originalMethod = target[methodName];
    const self = this;
    
    return function(...args) {
      // Build context
      const context = new InterceptContext(
        target,
        methodName,
        args,
        originalMethod
      );
      
      // Try local interception first
      const localResult = self._tryInterceptMethodLocally(target, methodName, context);
      if (localResult !== undefined) {
        return localResult;
      }
      
      // Try hub interception next
      const hubResult = self.hubConnector.tryInterceptMethod(target, methodName, context);
      if (hubResult !== undefined) {
        return hubResult;
      }
      
      // If not intercepted, call the original method
      return originalMethod.apply(target, args);
    };
  }
}

// ========================
// Hub Connector Implementation
// ========================

/**
 * Connector to the Rust hub implementation
 */
class HubConnector {
  /**
   * Initialize the hub connector
   */
  constructor() {
    // In a real implementation, would connect to the Rust hub
    // For pseudocode, we'll use local Maps to simulate the hub
    this._localRegistry = new Map();
    this._localSubscriptions = new Map();
    this._localInterceptors = new Map();
  }
  
  /**
   * Register an API endpoint with the hub
   * @param {string} path - API path
   * @param {Function} handler - API handler
   * @param {Object<string,string>} metadata - API metadata
   */
  registerApi(path, handler, metadata) {
    // In a real implementation, would call the Rust hub
    // For pseudocode, store locally
    this._localRegistry.set(path, {
      handler,
      metadata
    });
  }
  
  /**
   * Handle an API request
   * @param {ApiRequest} request - API request
   * @returns {ApiResponse} API response
   */
  handleRequest(request) {
    // In a real implementation, would call the Rust hub
    // For pseudocode, handle locally
    
    // Check for interception
    // (not implemented in this simplified version)
    
    // Check local registry
    if (this._localRegistry.has(request.path)) {
      const api = this._localRegistry.get(request.path);
      return api.handler(request);
    }
    
    // Try fallback
    for (const [path, entry] of this._localRegistry.entries()) {
      if (entry.metadata.fallback === request.path) {
        const fallbackRequest = new ApiRequest(
          entry.metadata.fallback,
          request.data,
          { ...request.metadata, originalPath: request.path },
          request.senderId
        );
        return this.handleRequest(fallbackRequest);
      }
    }
    
    // Not found
    return new ApiResponse(
      null,
      {},
      ResponseStatus.NOT_FOUND
    );
  }
  
  /**
   * Subscribe to messages matching a pattern
   * @param {string} pattern - Message pattern
   * @param {Function} callback - Callback function
   * @param {number} priority - Subscription priority
   */
  subscribe(pattern, callback, priority) {
    // In a real implementation, would call the Rust hub
    // For pseudocode, store locally
    if (!this._localSubscriptions.has(pattern)) {
      this._localSubscriptions.set(pattern, []);
    }
    
    const subscribers = this._localSubscriptions.get(pattern);
    
    subscribers.push({
      callback,
      priority
    });
    
    // Sort by priority (descending)
    subscribers.sort((a, b) => b.priority - a.priority);
  }
  
  /**
   * Register a message interceptor for a pattern
   * @param {string} pattern - Message pattern
   * @param {Function} handler - Interceptor handler
   * @param {number} priority - Interceptor priority
   */
  registerInterceptor(pattern, handler, priority) {
    // In a real implementation, would call the Rust hub
    // For pseudocode, store locally
    if (!this._localInterceptors.has(pattern)) {
      this._localInterceptors.set(pattern, []);
    }
    
    const interceptors = this._localInterceptors.get(pattern);
    
    interceptors.push({
      handler,
      priority
    });
    
    // Sort by priority (descending)
    interceptors.sort((a, b) => b.priority - a.priority);
  }
  
  /**
   * Publish a message and allow for interception
   * @param {Message} message - Message to publish
   * @returns {any} Result from any interceptors
   */
  publish(message) {
    // In a real implementation, would call the Rust hub
    // For pseudocode, handle locally
    
    // Check for interceptors
    for (const [pattern, interceptors] of this._localInterceptors.entries()) {
      if (this._patternMatches(pattern, message.topic)) {
        for (const interceptor of interceptors) {
          const result = interceptor.handler(message);
          if (result !== undefined) {
            return result;
          }
        }
      }
    }
    
    // Notify subscribers
    for (const [pattern, subscribers] of this._localSubscriptions.entries()) {
      if (this._patternMatches(pattern, message.topic)) {
        for (const subscriber of subscribers) {
          subscriber.callback(message);
        }
      }
    }
    
    return undefined;
  }
  
  /**
   * Try to intercept a method call
   * @param {Object} instance - Object instance
   * @param {string} methodName - Method name
   * @param {InterceptContext} context - Interception context
   * @returns {any} Interception result or undefined
   */
  tryInterceptMethod(instance, methodName, context) {
    // In a real implementation, would call the Rust hub
    // For pseudocode, return undefined (not intercepted)
    return undefined;
  }
  
  /**
   * Check if a topic matches a pattern
   * @param {string} pattern - Pattern to match
   * @param {string} topic - Topic to check
   * @returns {boolean} True if the topic matches the pattern
   * @private
   */
  _patternMatches(pattern, topic) {
    if (pattern === topic) {
      return true;
    }
    
    if (pattern.endsWith('*') && topic.startsWith(pattern.slice(0, -1))) {
      return true;
    }
    
    return false;
  }
}

// ========================
// Helper Functions
// ========================

/**
 * Generate a random UUID
 * @returns {string} Random UUID
 */
function generateUuid() {
  // Simple UUID implementation
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) {
    const r = Math.random() * 16 | 0;
    const v = c === 'x' ? r : (r & 0x3 | 0x8);
    return v.toString(16);
  });
}

/**
 * Create a new node in the virtual network
 * @param {string} [nodeId] - Optional node ID
 * @returns {Node} New node
 */
function createNode(nodeId = null) {
  return new Node(nodeId);
}

// ========================
// Utility Functions
// ========================

/**
 * Register an API endpoint with a fallback
 * @param {Node} node - Node to register with
 * @param {string} primaryPath - Primary API path
 * @param {string} fallbackPath - Fallback API path
 * @param {Function} handler - API handler
 */
function registerWithFallback(node, primaryPath, fallbackPath, handler) {
  // Register primary
  node.registerApi(primaryPath, handler, { fallback: fallbackPath });
  
  // Also register at fallback path
  node.registerApi(fallbackPath, handler, {});
}

/**
 * Register a static mock API
 * @param {Node} node - Node to register with
 * @param {string} path - API path
 * @param {any} staticResponse - Static response data
 */
function registerStatic(node, path, staticResponse) {
  node.registerApi(path, (request) => {
    return new ApiResponse(
      staticResponse,
      { isStatic: 'true' },
      ResponseStatus.SUCCESS
    );
  }, { isStatic: 'true' });
}

/**
 * Set up interception between file and web APIs
 * @param {Node} node - Node to set up with
 */
function setupFileWebInterception(node) {
  // Register file search API
  node.registerApi('/search/files', (request) => {
    const query = request.data.query || '';
    return new ApiResponse(
      { results: [`file:${query}_result1`, `file:${query}_result2`] },
      {},
      ResponseStatus.SUCCESS
    );
  });
  
  // Set up interceptor for web search
  node.registerInterceptor('/search/files', (message) => {
    if (message.metadata.source === 'web' || (message.data.query && message.data.query.includes('web:'))) {
      const query = message.data.query || '';
      return { results: [`web:${query}_result1`, `web:${query}_result2`] };
    }
    return undefined;  // Not intercepted
  }, 10);  // High priority
}

// ========================
// Exports
// ========================

module.exports = {
  // Core classes
  Node,
  Message,
  ApiRequest,
  ApiResponse,
  InterceptContext,
  
  // Enums
  HubScope,
  ResponseStatus,
  
  // Factory functions
  createNode,
  
  // Utility functions
  registerWithFallback,
  registerStatic,
  setupFileWebInterception
};

// ========================
// Usage Example
// ========================

/*
// Example usage
const { createNode, setupFileWebInterception } = require('./node_library');

// Create a node
const node = createNode();

// Register an API
node.registerApi('/echo', (request) => {
  return {
    data: request.data,
    metadata: request.metadata,
    status: 'success'
  };
});

// Call the API
const response = node.callApi('/echo', { message: 'Hello, World!' });
console.log(`Response: ${JSON.stringify(response.data)}`);

// Set up file/web search interception example
setupFileWebInterception(node);

// Test interception
console.log("\nTesting interception:");

// Regular file search
const fileResponse = node.callApi('/search/files', { query: 'test' });
console.log(`File search: ${JSON.stringify(fileResponse.data)}`);

// Web search (intercepted)
const webResponse = node.callApi('/search/files', { query: 'web:test' }, { source: 'web' });
console.log(`Web search: ${JSON.stringify(webResponse.data)}`);

// Test method interception
console.log("\nTesting method interception:");

// Define a class with interceptable methods
class FileSearcher {
  @node.intercept
  search(query) {
    return [`file:${query}_result1`, `file:${query}_result2`];
  }
}

// Register method interceptor
node.registerMethodInterceptor(FileSearcher, 'search', (context) => {
  const query = context.args[0];
  if (query.startsWith('web:')) {
    return [`web:${query}_result1`, `web:${query}_result2`];
  }
  return undefined;  // Not intercepted
}, 10);

// Test method interception
const searcher = new FileSearcher();
console.log(`Regular search: ${JSON.stringify(searcher.search('test'))}`);
console.log(`Web search: ${JSON.stringify(searcher.search('web:test'))}`);
*/