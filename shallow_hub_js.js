// shallow_hub_js.js - Lightweight in-process hub implementation for JavaScript

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
  
  /**
   * Convert to plain object
   * @returns {Object} Plain object representation
   */
  toObject() {
    return {
      topic: this.topic,
      data: this.data,
      metadata: this.metadata,
      senderId: this.senderId,
      timestamp: this.timestamp
    };
  }
  
  /**
   * Create from plain object
   * @param {Object} obj - Object to create from
   * @returns {Message} New message
   */
  static fromObject(obj) {
    const message = new Message(
      obj.topic,
      obj.data,
      obj.metadata,
      obj.senderId
    );
    message.timestamp = obj.timestamp;
    return message;
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
  
  /**
   * Convert to plain object
   * @returns {Object} Plain object representation
   */
  toObject() {
    return {
      path: this.path,
      data: this.data,
      metadata: this.metadata,
      senderId: this.senderId
    };
  }
  
  /**
   * Create from plain object
   * @param {Object} obj - Object to create from
   * @returns {ApiRequest} New request
   */
  static fromObject(obj) {
    return new ApiRequest(
      obj.path,
      obj.data,
      obj.metadata,
      obj.senderId
    );
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
  
  /**
   * Convert to plain object
   * @returns {Object} Plain object representation
   */
  toObject() {
    return {
      data: this.data,
      metadata: this.metadata,
      status: this.status
    };
  }
  
  /**
   * Create from plain object
   * @param {Object} obj - Object to create from
   * @returns {ApiResponse} New response
   */
  static fromObject(obj) {
    return new ApiResponse(
      obj.data,
      obj.metadata,
      obj.status
    );
  }
}

/**
 * Internal hub message format
 */
class HubMessage {
  /**
   * Create a new hub message
   * @param {number} type - Message type
   * @param {Object} data - Message data
   * @param {string} clientId - Sender client ID
   * @param {string} [messageId] - Message ID (generated if not provided)
   */
  constructor(type, data, clientId, messageId = null) {
    this.type = type;
    this.data = data;
    this.clientId = clientId;
    this.messageId = messageId || generateUuid();
    this.timestamp = Date.now();
  }
  
  /**
   * Convert to plain object
   * @returns {Object} Plain object representation
   */
  toObject() {
    return {
      type: this.type,
      data: this.data,
      clientId: this.clientId,
      messageId: this.messageId,
      timestamp: this.timestamp
    };
  }
  
  /**
   * Create from plain object
   * @param {Object} obj - Object to create from
   * @returns {HubMessage} New message
   */
  static fromObject(obj) {
    const message = new HubMessage(
      obj.type,
      obj.data,
      obj.clientId,
      obj.messageId
    );
    message.timestamp = obj.timestamp;
    return message;
  }
  
  // Message types
  static get TYPE_API_REQUEST() { return 1; }
  static get TYPE_API_RESPONSE() { return 2; }
  static get TYPE_PUBLISH() { return 3; }
  static get TYPE_SUBSCRIBE() { return 4; }
  static get TYPE_REGISTER_API() { return 5; }
  static get TYPE_REGISTER_INTERCEPTOR() { return 6; }
  static get TYPE_INTERCEPT() { return 7; }
  static get TYPE_SHUTDOWN() { return 99; }
}

// ========================
// Shallow Hub Implementation
// ========================

/**
 * A lightweight hub implementation for in-process communication
 */
class ShallowHub {
  /**
   * Create a new shallow hub
   * @param {string} [scope=HubScope.PROCESS] - Hub scope
   */
  constructor(scope = HubScope.PROCESS) {
    this.hubId = generateUuid();
    this.scope = scope;
    
    // Hub data structures
    this.apiRegistry = new Map();  // path -> { handler, metadata, clientId }
    this.subscriptions = new Map();  // pattern -> [{ pattern, clientId, subscriptionId, priority }]
    this.interceptors = new Map();  // pattern -> [{ pattern, clientId, interceptorId, priority }]
    this.clients = new Map();  // clientId -> { queue: Queue, active: boolean, worker: Worker }
    
    // Message origin tracking (for request/response correlation)
    this.requestOrigins = new Map();  // messageId -> clientId
    
    // Hub message queue
    this.messageQueue = [];
    this.processing = false;
    
    // Hub status
    this.running = true;
    
    // Start hub message processing
    this.processInterval = setInterval(() => this._processMessages(), 10);
    
    console.log(`Shallow Hub started with ID: ${this.hubId}`);
  }
  
  /**
   * Shut down the hub and all client connections
   */
  shutdown() {
    console.log(`Shutting down hub ${this.hubId}...`);
    this.running = false;
    
    // Clear process interval
    clearInterval(this.processInterval);
    
    // Close all client connections
    for (const [clientId, client] of this.clients.entries()) {
      client.active = false;
      
      // Send shutdown notification to client
      const shutdownMsg = new HubMessage(
        HubMessage.TYPE_SHUTDOWN,
        { reason: 'Hub shutdown' },
        this.hubId
      );
      this._sendMessageToClient(clientId, shutdownMsg.toObject());
    }
    
    // Clear data structures
    this.apiRegistry.clear();
    this.subscriptions.clear();
    this.interceptors.clear();
    this.clients.clear();
    this.requestOrigins.clear();
    this.messageQueue = [];
    
    console.log(`Hub ${this.hubId} shutdown complete`);
  }
  
  /**
   * Process messages in the queue
   * @private
   */
  _processMessages() {
    // Don't process if already processing or not running
    if (this.processing || !this.running) {
      return;
    }
    
    // Don't process if queue is empty
    if (this.messageQueue.length === 0) {
      return;
    }
    
    this.processing = true;
    
    try {
      // Process up to 10 messages at a time
      const count = Math.min(10, this.messageQueue.length);
      
      for (let i = 0; i < count; i++) {
        const messageObj = this.messageQueue.shift();
        this._processHubMessage(messageObj);
      }
    } catch (error) {
      console.error(`Error processing hub messages: ${error}`);
    } finally {
      this.processing = false;
    }
  }
  
  /**
   * Process a single hub message
   * @param {Object} messageObj - Message object
   * @private
   */
  _processHubMessage(messageObj) {
    const message = HubMessage.fromObject(messageObj);
    
    if (message.type === HubMessage.TYPE_SHUTDOWN) {
      console.log(`Hub received shutdown message: ${message.data.reason || 'No reason provided'}`);
      this.running = false;
      return;
    }
    
    else if (message.type === HubMessage.TYPE_REGISTER_API) {
      // Register an API endpoint
      const apiData = message.data;
      this.apiRegistry.set(apiData.path, apiData);
      
      // Send acknowledgment to client
      const response = {
        success: true,
        apiPath: apiData.path
      };
      this._sendResponseToClient(message.clientId, message.messageId, response);
    }
    
    else if (message.type === HubMessage.TYPE_SUBSCRIBE) {
      // Register a subscription
      const subData = message.data;
      const pattern = subData.pattern;
      
      if (!this.subscriptions.has(pattern)) {
        this.subscriptions.set(pattern, []);
      }
      
      // Add subscription
      const subscription = {
        pattern,
        clientId: message.clientId,
        subscriptionId: subData.subscriptionId,
        priority: subData.priority || 0
      };
      
      this.subscriptions.get(pattern).push(subscription);
      
      // Sort by priority (descending)
      this.subscriptions.get(pattern).sort((a, b) => b.priority - a.priority);
      
      // Send acknowledgment to client
      const response = {
        success: true,
        subscriptionId: subData.subscriptionId
      };
      this._sendResponseToClient(message.clientId, message.messageId, response);
    }
    
    else if (message.type === HubMessage.TYPE_REGISTER_INTERCEPTOR) {
      // Register an interceptor
      const intData = message.data;
      const pattern = intData.pattern;
      
      if (!this.interceptors.has(pattern)) {
        this.interceptors.set(pattern, []);
      }
      
      // Add interceptor
      const interceptor = {
        pattern,
        clientId: message.clientId,
        interceptorId: intData.interceptorId,
        priority: intData.priority || 0
      };
      
      this.interceptors.get(pattern).push(interceptor);
      
      // Sort by priority (descending)
      this.interceptors.get(pattern).sort((a, b) => b.priority - a.priority);
      
      // Send acknowledgment to client
      const response = {
        success: true,
        interceptorId: intData.interceptorId
      };
      this._sendResponseToClient(message.clientId, message.messageId, response);
    }
    
    else if (message.type === HubMessage.TYPE_API_REQUEST) {
      // Process an API request
      const requestData = message.data;
      const apiPath = requestData.path;
      
      // Look for matching API
      if (this.apiRegistry.has(apiPath)) {
        const apiEntry = this.apiRegistry.get(apiPath);
        
        // Store the original client for the response
        this._storeRequestOrigin(message.messageId, message.clientId);
        
        // Forward request to the registered handler's client
        const forwardMsg = new HubMessage(
          HubMessage.TYPE_API_REQUEST,
          requestData,
          apiEntry.clientId,
          message.messageId
        );
        
        // Forward to API handler's client
        this._sendMessageToClient(apiEntry.clientId, forwardMsg.toObject());
      } else {
        // API not found
        const response = new ApiResponse(
          null,
          { error: `API not found: ${apiPath}` },
          ResponseStatus.NOT_FOUND
        );
        
        this._sendResponseToClient(message.clientId, message.messageId, response.toObject());
      }
    }
    
    else if (message.type === HubMessage.TYPE_API_RESPONSE) {
      // Forward API response to the original requester
      const responseData = message.data;
      const originalClient = this._getRequestOrigin(message.messageId);
      
      if (originalClient) {
        this._sendResponseToClient(originalClient, message.messageId, responseData);
        this._clearRequestOrigin(message.messageId);
      }
    }
    
    else if (message.type === HubMessage.TYPE_PUBLISH) {
      // Process a publish request
      const messageData = message.data;
      const topic = messageData.topic;
      
      // Check for interceptors first
      let intercepted = false;
      
      for (const [pattern, interceptors] of this.interceptors.entries()) {
        if (this._patternMatches(pattern, topic)) {
          for (const interceptor of interceptors) {
            // Forward to interceptor's client
            const interceptMsg = new HubMessage(
              HubMessage.TYPE_INTERCEPT,
              messageData,
              interceptor.clientId,
              message.messageId
            );
            
            // Store the original client for the response
            this._storeRequestOrigin(message.messageId, message.clientId);
            
            // Send to interceptor
            this._sendMessageToClient(interceptor.clientId, interceptMsg.toObject());
            intercepted = true;
            break;
          }
          
          if (intercepted) {
            break;
          }
        }
      }
      
      // If not intercepted, publish to all subscribers
      if (!intercepted) {
        for (const [pattern, subs] of this.subscriptions.entries()) {
          if (this._patternMatches(pattern, topic)) {
            for (const sub of subs) {
              // Forward to subscriber's client
              const subMsg = new HubMessage(
                HubMessage.TYPE_PUBLISH,
                messageData,
                sub.clientId,
                null  // Don't need a response for publications
              );
              
              this._sendMessageToClient(sub.clientId, subMsg.toObject());
            }
          }
        }
        
        // Send acknowledgment to publisher
        const response = {
          success: true,
          intercepted: false
        };
        this._sendResponseToClient(message.clientId, message.messageId, response);
      }
    }
  }
  
  /**
   * Send a message to a client
   * @param {string} clientId - Client ID
   * @param {Object} message - Message to send
   * @returns {boolean} True if sent successfully
   * @private
   */
  _sendMessageToClient(clientId, message) {
    if (this.clients.has(clientId) && this.clients.get(clientId).active) {
      try {
        const client = this.clients.get(clientId);
        client.queue.push(message);
        
        // Trigger client message handling if using worker
        if (client.worker && typeof client.worker.postMessage === 'function') {
          client.worker.postMessage({ type: 'hub_notification' });
        }
        
        return true;
      } catch (error) {
        console.error(`Error sending message to client ${clientId}: ${error}`);
      }
    }
    return false;
  }
  
  /**
   * Send a response to a client's request
   * @param {string} clientId - Client ID
   * @param {string} messageId - Original message ID
   * @param {Object} responseData - Response data
   * @returns {boolean} True if sent successfully
   * @private
   */
  _sendResponseToClient(clientId, messageId, responseData) {
    const responseMsg = new HubMessage(
      HubMessage.TYPE_API_RESPONSE,
      responseData,
      this.hubId,
      messageId
    );
    return this._sendMessageToClient(clientId, responseMsg.toObject());
  }
  
  /**
   * Store the original client for a request
   * @param {string} messageId - Message ID
   * @param {string} clientId - Client ID
   * @private
   */
  _storeRequestOrigin(messageId, clientId) {
    this.requestOrigins.set(messageId, clientId);
  }
  
  /**
   * Get the original client for a request
   * @param {string} messageId - Message ID
   * @returns {string} Client ID
   * @private
   */
  _getRequestOrigin(messageId) {
    return this.requestOrigins.get(messageId);
  }
  
  /**
   * Clear the original client for a request
   * @param {string} messageId - Message ID
   * @private
   */
  _clearRequestOrigin(messageId) {
    this.requestOrigins.delete(messageId);
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
    
    if (pattern.endsWith('*')) {
      const prefix = pattern.substring(0, pattern.length - 1);
      return topic.startsWith(prefix);
    }
    
    return false;
  }
  
  /**
   * Add a message to the hub queue
   * @param {Object} message - Message to add
   * @private
   */
  _enqueueMessage(message) {
    this.messageQueue.push(message);
    // Process will happen on next interval
  }
  
  /**
   * Register a new client with the hub
   * @param {string} clientId - Client ID
   * @param {Worker} [worker] - Worker to notify when messages are available
   * @returns {Array} Client message queue
   */
  registerClient(clientId, worker = null) {
    if (this.clients.has(clientId)) {
      console.log(`Client ${clientId} already registered`);
      return this.clients.get(clientId).queue;
    }
    
    const clientQueue = [];
    this.clients.set(clientId, {
      queue: clientQueue,
      active: true,
      worker
    });
    
    console.log(`Client ${clientId} registered with hub ${this.hubId}`);
    return clientQueue;
  }
  
  /**
   * Unregister a client from the hub
   * @param {string} clientId - Client ID
   * @returns {boolean} True if unregistered successfully
   */
  unregisterClient(clientId) {
    if (this.clients.has(clientId)) {
      this.clients.get(clientId).active = false;
      console.log(`Client ${clientId} unregistered from hub ${this.hubId}`);
      return true;
    }
    return false;
  }
  
  /**
   * Send a message to the hub
   * @param {Object} message - Message to send
   */
  sendMessage(message) {
    if (!this.running) {
      throw new Error('Hub is not running');
    }
    
    this._enqueueMessage(message);
  }
}

// ========================
// Client Implementation
// ========================

/**
 * Client to connect to the shallow hub
 */
class ShallowHubClient {
  /**
   * Create a new shallow hub client
   * @param {ShallowHub} hub - Hub to connect to
   * @param {string} [clientId] - Client ID (generated if not provided)
   */
  constructor(hub, clientId = null) {
    this.hub = hub;
    this.clientId = clientId || generateUuid();
    this.clientQueue = null;
    this.connected = false;
    this.running = true;
    
    // Callbacks for handling requests
    this.apiHandlers = new Map();
    this.subscriptionHandlers = new Map();
    this.interceptorHandlers = new Map();
    
    // Pending requests waiting for responses
    this.pendingRequests = new Map();
    
    // Message checking interval
    this.checkInterval = null;
    
    // Worker for cross-thread communication
    this.worker = null;
    
    // Is this client running in a worker?
    this.isInWorker = typeof self !== 'undefined' && typeof window === 'undefined';
  }
  
  /**
   * Connect to the shallow hub
   * @returns {boolean} True if connected successfully
   */
  connect() {
    if (this.connected) {
      return true;
    }
    
    try {
      // Register with hub and get our message queue
      this.clientQueue = this.hub.registerClient(this.clientId, this.worker);
      this.connected = true;
      
      // Start message checking
      this.checkInterval = setInterval(() => this._checkMessages(), 50);
      
      // Set up worker message handling if in a worker
      if (this.isInWorker) {
        self.onmessage = (event) => {
          if (event.data && event.data.type === 'hub_notification') {
            this._checkMessages();
          }
        };
      }
      
      console.log(`Client ${this.clientId} connected to hub ${this.hub.hubId}`);
      return true;
    } catch (error) {
      console.error(`Error connecting to hub: ${error}`);
      return false;
    }
  }
  
  /**
   * Disconnect from the shallow hub
   */
  disconnect() {
    if (!this.connected) {
      return;
    }
    
    this.running = false;
    this.hub.unregisterClient(this.clientId);
    
    // Stop message checking
    if (this.checkInterval) {
      clearInterval(this.checkInterval);
      this.checkInterval = null;
    }
    
    // Clear worker message handling
    if (this.isInWorker) {
      self.onmessage = null;
    }
    
    this.connected = false;
    console.log(`Client ${this.clientId} disconnected from hub ${this.hub.hubId}`);
  }
  
  /**
   * Check for messages in the queue
   * @private
   */
  _checkMessages() {
    if (!this.running || !this.connected || !this.clientQueue) {
      return;
    }
    
    // Process all messages in the queue
    while (this.clientQueue.length > 0) {
      const messageObj = this.clientQueue.shift();
      this._processClientMessage(messageObj);
    }
  }
  
  /**
   * Process a message received by the client
   * @param {Object} messageObj - Message object
   * @private
   */
  _processClientMessage(messageObj) {
    const message = HubMessage.fromObject(messageObj);
    
    if (message.type === HubMessage.TYPE_SHUTDOWN) {
      console.log(`Client ${this.clientId} received shutdown notification from hub`);
      this.disconnect();
      return;
    }
    
    else if (message.type === HubMessage.TYPE_API_REQUEST) {
      // Handle API request
      const requestData = message.data;
      const apiPath = requestData.path;
      
      if (this.apiHandlers.has(apiPath)) {
        const handler = this.apiHandlers.get(apiPath);
        
        // Create API request
        const request = ApiRequest.fromObject(requestData);
        
        try {
          // Call handler
          const response = handler(request);
          
          // Send response
          this._sendApiResponse(message.messageId, response.toObject());
        } catch (error) {
          console.error(`Error handling API request ${apiPath}: ${error}`);
          
          // Send error response
          const errorResponse = new ApiResponse(
            null,
            { error: error.message },
            ResponseStatus.ERROR
          );
          this._sendApiResponse(message.messageId, errorResponse.toObject());
        }
      } else {
        // API not registered with this client
        console.error(`API ${apiPath} not registered with client ${this.clientId}`);
        
        const errorResponse = new ApiResponse(
          null,
          { error: `API ${apiPath} not registered with client ${this.clientId}` },
          ResponseStatus.NOT_FOUND
        );
        this._sendApiResponse(message.messageId, errorResponse.toObject());
      }
    }
    
    else if (message.type === HubMessage.TYPE_API_RESPONSE) {
      // Handle API response
      const responseData = message.data;
      
      if (this.pendingRequests.has(message.messageId)) {
        // Get the response handler
        const { resolve, reject } = this.pendingRequests.get(message.messageId);
        
        // Remove from pending requests
        this.pendingRequests.delete(message.messageId);
        
        // Resolve the promise
        resolve(responseData);
      } else {
        console.warn(`Received response for unknown request: ${message.messageId}`);
      }
    }
    
    else if (message.type === HubMessage.TYPE_PUBLISH) {
      // Handle published message
      const messageData = message.data;
      const topic = messageData.topic;
      
      // Find matching subscription handlers
      for (const [pattern, handlers] of this.subscriptionHandlers.entries()) {
        if (this._patternMatches(pattern, topic)) {
          // Create message object
          const msg = Message.fromObject(messageData);
          
          // Call handlers in order of priority
          for (const handler of handlers.sort((a, b) => b.priority - a.priority)) {
            try {
              handler.callback(msg);
            } catch (error) {
              console.error(`Error in subscription handler for ${pattern}: ${error}`);
            }
          }
        }
      }
    }
    
    else if (message.type === HubMessage.TYPE_INTERCEPT) {
      // Handle interception request
      const messageData = message.data;
      const topic = messageData.topic;
      
      // Find matching interceptor handlers
      for (const [pattern, handlers] of this.interceptorHandlers.entries()) {
        if (this._patternMatches(pattern, topic)) {
          // Create message object
          const msg = Message.fromObject(messageData);
          
          // Call handlers in order of priority
          for (const handler of handlers.sort((a, b) => b.priority - a.priority)) {
            try {
              const result = handler.callback(msg);
              if (result !== undefined) {
                // Send interception result
                this._sendInterceptResult(message.messageId, result);
                return;
              }
            } catch (error) {
              console.error(`Error in interceptor handler for ${pattern}: ${error}`);
            }
          }
        }
      }
      
      // No interception
      this._sendInterceptResult(message.messageId, undefined);
    }
  }
  
  /**
   * Send a message to the hub
   * @param {number} msgType - Message type
   * @param {Object} data - Message data
   * @param {string} [messageId] - Message ID (generated if not provided)
   * @returns {string} Message ID
   * @private
   */
  _sendMessage(msgType, data, messageId = null) {
    if (!this.connected) {
      throw new Error('Not connected to the hub');
    }
    
    const message = new HubMessage(
      msgType,
      data,
      this.clientId,
      messageId || generateUuid()
    );
    
    this.hub.sendMessage(message.toObject());
    return message.messageId;
  }
  
  /**
   * Send an API response to the hub
   * @param {string} messageId - Original message ID
   * @param {Object} responseData - Response data
   * @returns {string} Message ID
   * @private
   */
  _sendApiResponse(messageId, responseData) {
    return this._sendMessage(HubMessage.TYPE_API_RESPONSE, responseData, messageId);
  }
  
  /**
   * Send an interception result to the hub
   * @param {string} messageId - Original message ID
   * @param {any} result - Interception result
   * @returns {string} Message ID
   * @private
   */
  _sendInterceptResult(messageId, result) {
    return this._sendMessage(
      HubMessage.TYPE_API_RESPONSE,
      { intercepted: result !== undefined, result },
      messageId
    );
  }
  
  /**
   * Send a request to the hub and wait for the response
   * @param {number} msgType - Message type
   * @param {Object} data - Message data
   * @param {number} [timeout=30000] - Timeout in milliseconds
   * @returns {Promise<Object>} Response data
   * @private
   */
  _sendRequestAndWait(msgType, data, timeout = 30000) {
    return new Promise((resolve, reject) => {
      try {
        const messageId = this._sendMessage(msgType, data);
        
        // Store in pending requests
        this.pendingRequests.set(messageId, { resolve, reject });
        
        // Set timeout
        const timeoutId = setTimeout(() => {
          if (this.pendingRequests.has(messageId)) {
            this.pendingRequests.delete(messageId);
            reject(new Error(`Request timed out after ${timeout}ms`));
          }
        }, timeout);
        
        // Modify the resolver to clear the timeout
        const originalResolve = resolve;
        resolve = (value) => {
          clearTimeout(timeoutId);
          originalResolve(value);
        };
        
      } catch (error) {
        reject(error);
      }
    });
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
    
    if (pattern.endsWith('*')) {
      const prefix = pattern.substring(0, pattern.length - 1);
      return topic.startsWith(prefix);
    }
    
    return false;
  }
  
  /**
   * Register an API endpoint with the hub
   * @param {string} path - API path
   * @param {Function} handler - Handler function
   * @param {Object} [metadata={}] - API metadata
   * @returns {Promise<boolean>} True if registration succeeded
   */
  async registerApi(path, handler, metadata = {}) {
    // Store handler locally
    this.apiHandlers.set(path, handler);
    
    // Register with hub
    const apiData = {
      path,
      metadata,
      clientId: this.clientId
    };
    
    try {
      const response = await this._sendRequestAndWait(HubMessage.TYPE_REGISTER_API, apiData);
      return response.success === true;
    } catch (error) {
      console.error(`Error registering API ${path}: ${error}`);
      return false;
    }
  }
  
  /**
   * Call an API endpoint on the hub
   * @param {string} path - API path
   * @param {any} [data=null] - Request data
   * @param {Object} [metadata={}] - Request metadata
   * @returns {Promise<ApiResponse>} API response
   */
  async callApi(path, data = null, metadata = {}) {
    // Create request
    const requestData = {
      path,
      data,
      metadata,
      senderId: this.clientId
    };
    
    try {
      const responseData = await this._sendRequestAndWait(HubMessage.TYPE_API_REQUEST, requestData);
      
      // Convert status string to enum
      let status = responseData.status;
      if (typeof status === 'string') {
        status = ResponseStatus[status.toUpperCase()] || ResponseStatus.ERROR;
      } else if (typeof status === 'number') {
        // Convert numeric status
        const statusMap = {
          0: ResponseStatus.SUCCESS,
          1: ResponseStatus.NOT_FOUND,
          2: ResponseStatus.ERROR,
          3: ResponseStatus.INTERCEPTED,
          4: ResponseStatus.APPROXIMATED
        };
        status = statusMap[status] || ResponseStatus.ERROR;
      }
      
      return new ApiResponse(
        responseData.data,
        responseData.metadata || {},
        status
      );
    } catch (error) {
      console.error(`Error calling API ${path}: ${error}`);
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
   * @param {number} [priority=0] - Subscription priority
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
    
    // Register with hub
    const subscriptionData = {
      pattern,
      subscriptionId,
      priority
    };
    
    try {
      const response = await this._sendRequestAndWait(HubMessage.TYPE_SUBSCRIBE, subscriptionData);
      if (!response.success) {
        console.warn(`Subscription registration failed: ${response.error || 'Unknown error'}`);
      }
    } catch (error) {
      console.error(`Error registering subscription: ${error}`);
    }
    
    return subscriptionId;
  }
  
  /**
   * Register a message interceptor for a pattern
   * @param {string} pattern - Message pattern
   * @param {Function} handler - Handler function
   * @param {number} [priority=0] - Interceptor priority
   * @returns {Promise<string>} Interceptor ID
   */
  async registerInterceptor(pattern, handler, priority = 0) {
    const interceptorId = generateUuid();
    
    // Store handler locally
    if (!this.interceptorHandlers.has(pattern)) {
      this.interceptorHandlers.set(pattern, []);
    }
    
    this.interceptorHandlers.get(pattern).push({
      id: interceptorId,
      callback: handler,
      priority
    });
    
    // Register with hub
    const interceptorData = {
      pattern,
      interceptorId,
      priority
    };
    
    try {
      const response = await this._sendRequestAndWait(HubMessage.TYPE_REGISTER_INTERCEPTOR, interceptorData);
      if (!response.success) {
        console.warn(`Interceptor registration failed: ${response.error || 'Unknown error'}`);
      }
    } catch (error) {
      console.error(`Error registering interceptor: ${error}`);
    }
    
    return interceptorId;
  }
  
  /**
   * Publish a message to the hub
   * @param {string} topic - Message topic
   * @param {any} data - Message data
   * @param {Object} [metadata={}] - Message metadata
   * @returns {Promise<any>} Result from any interceptors
   */
  async publish(topic, data, metadata = {}) {
    // Create message
    const messageData = {
      topic,
      data,
      metadata,
      senderId: this.clientId,
      timestamp: Date.now()
    };
    
    try {
      const response = await this._sendRequestAndWait(HubMessage.TYPE_PUBLISH, messageData);
      return response.intercepted ? response.result : undefined;
    } catch (error) {
      console.error(`Error publishing message to ${topic}: ${error}`);
      return undefined;
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
   * @param {ShallowHub} hub - Hub to connect to
   * @param {string} [nodeId] - Node ID (generated if not provided)
   */
  constructor(hub, nodeId = null) {
    this.nodeId = nodeId || generateUuid();
    this.hubClient = new ShallowHubClient(hub, this.nodeId);
  }
  
  /**
   * Connect to the hub
   * @returns {boolean} True if connected successfully
   */
  connect() {
    return this.hubClient.connect();
  }
  
  /**
   * Disconnect from the hub
   */
  disconnect() {
    this.hubClient.disconnect();
  }
  
  /**
   * Register an API endpoint with the node
   * @param {string} path - API path
   * @param {Function} handler - Handler function
   * @param {Object} [metadata={}] - API metadata
   * @returns {Promise<boolean>} True if registration succeeded
   */
  async registerApi(path, handler, metadata = {}) {
    return this.hubClient.registerApi(path, handler, metadata);
  }
  
  /**
   * Call an API endpoint on the network
   * @param {string} path - API path
   * @param {any} [data=null] - Request data
   * @param {Object} [metadata={}] - Request metadata
   * @returns {Promise<ApiResponse>} API response
   */
  async callApi(path, data = null, metadata = {}) {
    return this.hubClient.callApi(path, data, metadata);
  }
  
  /**
   * Subscribe to messages matching a pattern
   * @param {string} pattern - Message pattern
   * @param {Function} callback - Callback function
   * @param {number} [priority=0] - Subscription priority
   * @returns {Promise<string>} Subscription ID
   */
  async subscribe(pattern, callback, priority = 0) {
    return this.hubClient.subscribe(pattern, callback, priority);
  }
  
  /**
   * Register a message interceptor for a pattern
   * @param {string} pattern - Message pattern
   * @param {Function} handler - Handler function
   * @param {number} [priority=0] - Interceptor priority
   * @returns {Promise<string>} Interceptor ID
   */
  async registerInterceptor(pattern, handler, priority = 0) {
    return this.hubClient.registerInterceptor(pattern, handler, priority);
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
}

// ========================
// Helper Functions
// ========================

/**
 * Generate a random UUID
 * @returns {string} Random UUID
 */
function generateUuid() {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) {
    const r = Math.random() * 16 | 0;
    const v = c === 'x' ? r : (r & 0x3 | 0x8);
    return v.toString(16);
  });
}

/**
 * Create a new shallow hub
 * @param {string} [scope=HubScope.PROCESS] - Hub scope
 * @returns {ShallowHub} New shallow hub
 */
function createShallowHub(scope = HubScope.PROCESS) {
  return new ShallowHub(scope);
}

/**
 * Create a new node connected to a shallow hub
 * @param {ShallowHub} hub - Hub to connect to
 * @param {string} [nodeId] - Node ID (generated if not provided)
 * @returns {Node} New node
 */
function createNode(hub, nodeId = null) {
  const node = new Node(hub, nodeId);
  node.connect();
  return node;
}

// ========================
// Exports
// ========================

if (typeof module !== 'undefined') {
  // Node.js or CommonJS environment
  module.exports = {
    // Core classes
    ShallowHub,
    ShallowHubClient,
    Node,
    Message,
    ApiRequest,
    ApiResponse,
    
    // Enums
    HubScope,
    ResponseStatus,
    
    // Factory functions
    createShallowHub,
    createNode,
    generateUuid
  };
} else if (typeof window !== 'undefined') {
  // Browser environment
  window.ShallowHub = {
    ShallowHub,
    ShallowHubClient,
    Node,
    Message,
    ApiRequest,
    ApiResponse,
    HubScope,
    ResponseStatus,
    createShallowHub,
    createNode,
    generateUuid
  };
}

// ========================
// Usage Example
// ========================

/**
 * Example usage of the shallow hub
 */
async function example() {
  // Create a shallow hub
  const hub = createShallowHub();
  
  try {
    // Create two nodes
    const node1 = createNode(hub, 'node1');
    const node2 = createNode(hub, 'node2');
    
    // Register APIs on each node
    await node1.registerApi('/node1/greeting', (request) => {
      return new ApiResponse(
        'Hello from Node 1!',
        {},
        ResponseStatus.SUCCESS
      );
    });
    
    await node2.registerApi('/node2/greeting', (request) => {
      return new ApiResponse(
        'Hello from Node 2!',
        {},
        ResponseStatus.SUCCESS
      );
    });
    
    // Node 1 calls Node 2's API
    const response1 = await node1.callApi('/node2/greeting');
    console.log('Node 1 received:', response1.data);
    
    // Node 2 calls Node 1's API
    const response2 = await node2.callApi('/node1/greeting');
    console.log('Node 2 received:', response2.data);
    
    // Set up message subscription
    const messagesReceived = [];
    
    await node2.subscribe('test/*', (message) => {
      console.log(`Received message: ${message.topic} ->`, message.data);
      messagesReceived.push(message.data);
    });
    
    // Publish a message
    await node1.publish('test/hello', 'Hello from Node 1!');
    
    // Wait a bit for the message to be processed
    await new Promise(resolve => setTimeout(resolve, 100));
    
    console.log('Messages received:', messagesReceived);
    
    // Test with a more complex example
    // Register an echo API
    await node1.registerApi('/echo', (request) => {
      return new ApiResponse(
        `Echo: ${request.data}`,
        request.metadata,
        ResponseStatus.SUCCESS
      );
    });
    
    // Call the echo API with data
    const response = await node2.callApi('/echo', 'Testing echo');
    console.log('Echo response:', response.data);
    
    // Test interception
    await node2.registerInterceptor('test/*', (message) => {
      if (message.data && message.data.includes('intercept')) {
        return `Intercepted: ${message.data}`;
      }
      return undefined;
    });
    
    // Publish a message that should be intercepted
    const result1 = await node1.publish('test/intercept', 'Please intercept this message');
    console.log('Interception result:', result1);
    
    // Publish a message that should not be intercepted
    const result2 = await node1.publish('test/normal', 'Normal message');
    console.log('Normal publish result:', result2);
    
    // Disconnect nodes
    node1.disconnect();
    node2.disconnect();
  } finally {
    // Shut down hub
    hub.shutdown();
  }
}

// If running directly (not imported)
if (typeof module !== 'undefined' && require.main === module) {
  // Run the example
  example().catch(console.error);
}