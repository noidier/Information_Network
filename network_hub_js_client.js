// network_hub_js_client.js - JavaScript client for network-hub-rs

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

// ========================
// Network Client Implementation
// ========================

/**
 * Client to connect to the Rust network hub
 */
class NetworkHubClient {
  /**
   * Initialize a new network hub client
   * @param {Object} options - Connection options
   * @param {string} [options.host="localhost"] - Host where the network hub is running
   * @param {number} [options.port=8443] - Port the network hub is listening on
   * @param {string} [options.clientId] - Client ID, generated if not provided
   * @param {boolean} [options.useTls=true] - Whether to use TLS for the connection
   * @param {boolean} [options.rejectUnauthorized=true] - Whether to reject unauthorized certificates
   * @param {Object} [options.certificates] - TLS certificates
   * @param {string} [options.certificates.cert] - Client certificate
   * @param {string} [options.certificates.key] - Client key
   * @param {string} [options.certificates.ca] - CA certificate for verification
   */
  constructor(options = {}) {
    this.host = options.host || 'localhost';
    this.port = options.port || 8443;
    this.clientId = options.clientId || generateUuid();
    this.useTls = options.useTls !== false;
    this.rejectUnauthorized = options.rejectUnauthorized !== false;
    this.certificates = options.certificates || {};
    
    this.reconnectInterval = 5000; // 5 seconds
    this.socket = null;
    this.connected = false;
    this.connecting = false;
    this.reconnectTimer = null;
    
    // WebSocket URL
    this.wsProtocol = this.useTls ? 'wss' : 'ws';
    this.wsUrl = `${this.wsProtocol}://${this.host}:${this.port}`;
    
    // Callbacks for messages
    this.subscriptionHandlers = new Map();
    this.pendingRequests = new Map();
    this.nextRequestId = 1;
  }
  
  /**
   * Connect to the network hub
   * @returns {Promise<boolean>} True if connected, false otherwise
   */
  async connect() {
    if (this.connected) {
      return true;
    }
    
    if (this.connecting) {
      // Wait for the current connection attempt to finish
      return new Promise(resolve => {
        const checkInterval = setInterval(() => {
          if (!this.connecting) {
            clearInterval(checkInterval);
            resolve(this.connected);
          }
        }, 100);
      });
    }
    
    this.connecting = true;
    
    try {
      // Use WebSocket to connect
      return await new Promise((resolve, reject) => {
        console.log(`Connecting to network hub at ${this.wsUrl}`);
        
        // Create WebSocket
        this.socket = new WebSocket(this.wsUrl);
        
        // Set up event handlers
        this.socket.onopen = () => {
          console.log('Connected to network hub');
          this.connected = true;
          this.connecting = false;
          
          // Send client identification
          this._sendMessage({
            type: 'identify',
            clientId: this.clientId
          });
          
          resolve(true);
        };
        
        this.socket.onclose = (event) => {
          console.log(`Disconnected from network hub: code=${event.code}, reason=${event.reason}`);
          this.connected = false;
          this.connecting = false;
          
          // Fail any pending requests
          for (const [id, { reject }] of this.pendingRequests) {
            reject(new Error('Connection closed'));
            this.pendingRequests.delete(id);
          }
          
          // Try to reconnect
          if (!this.reconnectTimer) {
            this.reconnectTimer = setTimeout(() => {
              this.reconnectTimer = null;
              this.connect();
            }, this.reconnectInterval);
          }
          
          resolve(false);
        };
        
        this.socket.onerror = (error) => {
          console.error('WebSocket error:', error);
          this.connecting = false;
          reject(error);
        };
        
        this.socket.onmessage = (event) => {
          try {
            const message = JSON.parse(event.data);
            this._handleMessage(message);
          } catch (error) {
            console.error('Error handling message:', error);
          }
        };
      });
    } catch (error) {
      console.error('Connection error:', error);
      this.connected = false;
      this.connecting = false;
      return false;
    }
  }
  
  /**
   * Disconnect from the network hub
   */
  disconnect() {
    if (this.socket) {
      this.socket.close();
      this.socket = null;
    }
    
    this.connected = false;
    
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }
  
  /**
   * Send a message to the network hub
   * @param {Object} message - Message to send
   * @private
   */
  _sendMessage(message) {
    if (!this.socket || this.socket.readyState !== WebSocket.OPEN) {
      throw new Error('Not connected to the network hub');
    }
    
    this.socket.send(JSON.stringify(message));
  }
  
  /**
   * Handle a message from the network hub
   * @param {Object} message - Message received
   * @private
   */
  _handleMessage(message) {
    // Handle different message types
    switch (message.type) {
      case 'response':
        // Handle response to a request
        if (message.requestId && this.pendingRequests.has(message.requestId)) {
          const { resolve, reject } = this.pendingRequests.get(message.requestId);
          this.pendingRequests.delete(message.requestId);
          
          if (message.error) {
            reject(new Error(message.error));
          } else {
            resolve(message.data);
          }
        }
        break;
        
      case 'subscription':
        // Handle published message for a subscription
        this._dispatchToSubscribers(message.pattern, message.data);
        break;
        
      default:
        console.warn(`Unknown message type: ${message.type}`);
    }
  }
  
  /**
   * Dispatch a message to subscribers
   * @param {string} pattern - Message pattern
   * @param {Object} messageData - Message data
   * @private
   */
  _dispatchToSubscribers(pattern, messageData) {
    // Convert to Message object
    const message = new Message(
      messageData.topic,
      messageData.data,
      messageData.metadata,
      messageData.senderId
    );
    message.timestamp = messageData.timestamp;
    
    // Find and notify subscribers
    for (const [subPattern, handlers] of this.subscriptionHandlers.entries()) {
      if (this._patternMatches(subPattern, message.topic)) {
        for (const handler of handlers) {
          try {
            handler.callback(message);
          } catch (error) {
            console.error(`Error in subscription handler for ${subPattern}:`, error);
          }
        }
      }
    }
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
  
  /**
   * Send a request to the network hub and wait for a response
   * @param {string} type - Request type
   * @param {Object} data - Request data
   * @returns {Promise<Object>} Response data
   * @private
   */
  async _sendRequest(type, data) {
    if (!await this.connect()) {
      throw new Error('Failed to connect to the network hub');
    }
    
    const requestId = this.nextRequestId++;
    
    // Create promise that will be resolved when the response arrives
    const promise = new Promise((resolve, reject) => {
      this.pendingRequests.set(requestId, { resolve, reject });
      
      // Set timeout for the request
      const timeout = setTimeout(() => {
        if (this.pendingRequests.has(requestId)) {
          this.pendingRequests.delete(requestId);
          reject(new Error('Request timed out'));
        }
      }, 30000); // 30 seconds timeout
      
      // Send the request
      try {
        this._sendMessage({
          type,
          requestId,
          ...data
        });
      } catch (error) {
        clearTimeout(timeout);
        this.pendingRequests.delete(requestId);
        reject(error);
      }
    });
    
    return promise;
  }
  
  /**
   * Register an API endpoint with the hub
   * @param {string} path - API path
   * @param {Function} handler - API handler
   * @param {Object} metadata - API metadata
   * @returns {Promise<boolean>} True if registration succeeded
   */
  async registerApi(path, handler, metadata = {}) {
    // Store handler locally
    // In a real implementation, the hub would call back to us
    this._apiHandlers = this._apiHandlers || new Map();
    this._apiHandlers.set(path, {
      handler,
      metadata
    });
    
    // Register with the hub
    try {
      const result = await this._sendRequest('register_api', {
        path,
        metadata,
        clientId: this.clientId
      });
      
      return result.success;
    } catch (error) {
      console.error(`Error registering API ${path}:`, error);
      return false;
    }
  }
  
  /**
   * Call an API endpoint on the network
   * @param {string} path - API path
   * @param {any} data - Request data
   * @param {Object} metadata - Request metadata
   * @returns {Promise<ApiResponse>} API response
   */
  async callApi(path, data = null, metadata = {}) {
    try {
      const result = await this._sendRequest('api_request', {
        path,
        data,
        metadata,
        senderId: this.clientId
      });
      
      // Convert status
      const statusMap = {
        0: ResponseStatus.SUCCESS,
        1: ResponseStatus.NOT_FOUND,
        2: ResponseStatus.ERROR,
        3: ResponseStatus.INTERCEPTED,
        4: ResponseStatus.APPROXIMATED
      };
      
      const status = statusMap[result.status] || ResponseStatus.ERROR;
      
      return new ApiResponse(
        result.data,
        result.metadata || {},
        status
      );
    } catch (error) {
      console.error(`Error calling API ${path}:`, error);
      return new ApiResponse(
        null,
        { error: error.message },
        ResponseStatus.ERROR
      );
    }
  }
  
  /**
   * Subscribe to messages matching a pattern
   * @param {string} pattern - Message pattern
   * @param {Function} callback - Callback function
   * @param {number} priority - Subscription priority
   * @returns {Promise<string>} Subscription ID
   */
  async subscribe(pattern, callback, priority = 0) {
    const subscriptionId = generateUuid();
    
    // Store handler locally
    if (!this.subscriptionHandlers.has(pattern)) {
      this.subscriptionHandlers.set(pattern, []);
    }
    
    this.subscriptionHandlers.get(pattern).push({
      id: subscriptionId,
      callback,
      priority
    });
    
    // Sort by priority (descending)
    this.subscriptionHandlers.get(pattern).sort((a, b) => b.priority - a.priority);
    
    // Register with the hub
    try {
      const result = await this._sendRequest('subscribe', {
        pattern,
        subscriptionId,
        priority,
        clientId: this.clientId
      });
      
      if (!result.success) {
        console.warn(`Subscription registration failed: ${result.error || 'Unknown error'}`);
      }
    } catch (error) {
      console.error(`Error registering subscription to ${pattern}:`, error);
    }
    
    return subscriptionId;
  }
  
  /**
   * Publish a message to the network
   * @param {string} topic - Message topic
   * @param {any} data - Message data
   * @param {Object} metadata - Message metadata
   * @returns {Promise<any>} Result from any interceptors
   */
  async publish(topic, data, metadata = {}) {
    try {
      const result = await this._sendRequest('publish', {
        message: {
          topic,
          data,
          metadata,
          senderId: this.clientId,
          timestamp: Date.now()
        }
      });
      
      return result.intercepted ? result.result : null;
    } catch (error) {
      console.error(`Error publishing message to ${topic}:`, error);
      return null;
    }
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
   * @param {Object} options - Connection options
   * @param {string} [options.host="localhost"] - Host where the network hub is running
   * @param {number} [options.port=8443] - Port the network hub is listening on
   * @param {string} [options.nodeId] - Node ID, generated if not provided
   * @param {boolean} [options.useTls=true] - Whether to use TLS for the connection
   * @param {boolean} [options.rejectUnauthorized=true] - Whether to reject unauthorized certificates
   * @param {Object} [options.certificates] - TLS certificates
   */
  constructor(options = {}) {
    this.nodeId = options.nodeId || generateUuid();
    
    // Create the hub client
    this.hubClient = new NetworkHubClient({
      host: options.host || 'localhost',
      port: options.port || 8443,
      clientId: this.nodeId,
      useTls: options.useTls !== false,
      rejectUnauthorized: options.rejectUnauthorized !== false,
      certificates: options.certificates
    });
  }
  
  /**
   * Connect to the network hub
   * @returns {Promise<boolean>} True if connected
   */
  async connect() {
    return this.hubClient.connect();
  }
  
  /**
   * Register an API endpoint with the node
   * @param {string} path - API path
   * @param {Function} handler - API handler function
   * @param {Object} [metadata={}] - API metadata
   * @returns {Promise<boolean>} True if registration succeeded
   */
  async registerApi(path, handler, metadata = {}) {
    // Wrap handler to match expected signature
    const wrappedHandler = (request) => {
      return handler(request);
    };
    
    return this.hubClient.registerApi(path, wrappedHandler, metadata);
  }
  
  /**
   * Call an API endpoint on the network
   * @param {string} path - API path
   * @param {any} [data=null] - Request data
   * @param {Object} [metadata={}] - Request metadata
   * @returns {Promise<ApiResponse>} Response from the API
   */
  async callApi(path, data = null, metadata = {}) {
    return this.hubClient.callApi(path, data, metadata);
  }
  
  /**
   * Subscribe to messages matching a pattern
   * @param {string} pattern - Message pattern to match
   * @param {Function} callback - Callback function
   * @param {number} [priority=0] - Subscription priority
   * @returns {Promise<string>} Subscription ID
   */
  async subscribe(pattern, callback, priority = 0) {
    // Wrap callback to match expected signature
    const wrappedCallback = (message) => {
      return callback(message);
    };
    
    return this.hubClient.subscribe(pattern, wrappedCallback, priority);
  }
  
  /**
   * Publish a message to the network
   * @param {string} topic - Message topic
   * @param {any} data - Message data
   * @param {Object} [metadata={}] - Message metadata
   * @returns {Promise<any>} Result from any interceptors
   */
  async publish(topic, data, metadata = {}) {
    return this.hubClient.publish(topic, data, metadata);
  }
  
  /**
   * Disconnect from the network hub
   */
  disconnect() {
    this.hubClient.disconnect();
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
 * @param {Object} [options={}] - Node options
 * @returns {Node} New node
 */
function createNode(options = {}) {
  return new Node(options);
}

// ========================
// Exports
// ========================

if (typeof module !== 'undefined') {
  // Node.js or CommonJS environment
  module.exports = {
    // Core classes
    Node,
    Message,
    ApiRequest,
    ApiResponse,
    NetworkHubClient,
    
    // Enums
    HubScope,
    ResponseStatus,
    
    // Factory functions
    createNode,
    generateUuid
  };
} else if (typeof window !== 'undefined') {
  // Browser environment
  window.NetworkHub = {
    Node,
    Message,
    ApiRequest,
    ApiResponse,
    NetworkHubClient,
    HubScope,
    ResponseStatus,
    createNode,
    generateUuid
  };
}

// ========================
// Usage Example
// ========================

/**
 * Example usage of the JavaScript client
 */
async function example() {
  // Create a node
  const node = createNode({
    host: 'localhost',
    port: 8443,
    useTls: true,
    rejectUnauthorized: false, // For testing only
    certificates: {
      cert: 'path/to/client.pem',
      key: 'path/to/client_key.pem'
    }
  });
  
  // Connect to the hub
  if (!await node.connect()) {
    console.error('Failed to connect to the hub');
    return;
  }
  
  try {
    // Call an API
    const response = await node.callApi('/hub1/greeting');
    console.log('API response:', response.data);
    
    // Register a callback for message subscription
    function messageHandler(message) {
      console.log(`Received message on ${message.topic}:`, message.data);
    }
    
    // Subscribe to messages
    const subId = await node.subscribe('test/*', messageHandler);
    console.log('Subscription ID:', subId);
    
    // Publish a message
    const result = await node.publish('test/hello', 'Hello from JavaScript client');
    console.log('Publish result:', result);
    
    // Keep the program running for a while
    console.log('Waiting for messages...');
    await new Promise(resolve => setTimeout(resolve, 10000));
  } finally {
    // Disconnect when done
    node.disconnect();
  }
}

// If running directly (not imported)
if (typeof module !== 'undefined' && require.main === module) {
  // Run the example
  example().catch(console.error);
}