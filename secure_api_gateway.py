# secure_api_gateway.py - Python implementation of a secure API gateway using the node library

import os
import ssl
import argparse
import threading
import json
import logging
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.request import Request, urlopen
from urllib.error import URLError, HTTPError

# Import our node library
from node_library import create_node, ApiRequest, ApiResponse, ResponseStatus

# Configure logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(name)s - %(levelname)s - %(message)s')
logger = logging.getLogger('api_gateway')

class SecureApiGateway:
    """
    Secure API Gateway built on the Information Network infrastructure.
    Provides authentication, routing, rate limiting, and encryption.
    """
    
    def __init__(self, cert_path, key_path, bind_address='localhost', port=8443):
        """Initialize the API gateway."""
        self.bind_address = bind_address
        self.port = port
        self.cert_path = cert_path
        self.key_path = key_path
        
        # Create a node
        self.node = create_node('api_gateway')
        
        # Route configuration
        self.routes = {}
        
        # Authentication and rate limiting
        self.api_keys = {}
        self.rate_limits = {}
        self.request_counts = {}
        
        # Register core APIs
        self._register_core_apis()
        
    def _register_core_apis(self):
        """Register core APIs for the gateway."""
        
        # Register API for adding routes
        self.node.register_api('/gateway/routes', self._handle_routes_api, {
            'secure': 'true',
            'admin_only': 'true'
        })
        
        # Register API for managing API keys
        self.node.register_api('/gateway/keys', self._handle_keys_api, {
            'secure': 'true',
            'admin_only': 'true'
        })
        
        # Register the catch-all proxy API
        self.node.register_api('/proxy/*', self._handle_proxy_request, {
            'secure': 'true'
        })
        
        # Register a health check API
        self.node.register_api('/health', self._handle_health_check, {
            'public': 'true'
        })
        
        logger.info("Core APIs registered")
        
    def _handle_routes_api(self, request):
        """Handle requests to add/remove/list routes."""
        try:
            data = request.data
            action = data.get('action', '')
            
            if action == 'add':
                path = data.get('path')
                target = data.get('target')
                auth_required = data.get('auth_required', True)
                rate_limit = data.get('rate_limit', 0)
                
                if path and target:
                    self.add_route(path, target, auth_required, rate_limit)
                    return ApiResponse(
                        data={'success': True, 'message': f'Route added: {path} -> {target}'},
                        metadata={},
                        status=ResponseStatus.SUCCESS
                    )
            
            elif action == 'remove':
                path = data.get('path')
                if path and path in self.routes:
                    del self.routes[path]
                    return ApiResponse(
                        data={'success': True, 'message': f'Route removed: {path}'},
                        metadata={},
                        status=ResponseStatus.SUCCESS
                    )
            
            elif action == 'list':
                return ApiResponse(
                    data={'routes': self.routes},
                    metadata={},
                    status=ResponseStatus.SUCCESS
                )
                
            return ApiResponse(
                data={'error': 'Invalid action or missing parameters'},
                metadata={},
                status=ResponseStatus.ERROR
            )
            
        except Exception as e:
            logger.error(f"Error handling routes API: {str(e)}")
            return ApiResponse(
                data={'error': str(e)},
                metadata={},
                status=ResponseStatus.ERROR
            )
            
    def _handle_keys_api(self, request):
        """Handle requests to add/remove/list API keys."""
        try:
            data = request.data
            action = data.get('action', '')
            
            if action == 'add':
                key = data.get('key')
                owner = data.get('owner')
                permissions = data.get('permissions', [])
                
                if key and owner:
                    self.api_keys[key] = {
                        'owner': owner,
                        'permissions': permissions
                    }
                    return ApiResponse(
                        data={'success': True, 'message': f'API key added for {owner}'},
                        metadata={},
                        status=ResponseStatus.SUCCESS
                    )
            
            elif action == 'remove':
                key = data.get('key')
                if key and key in self.api_keys:
                    del self.api_keys[key]
                    return ApiResponse(
                        data={'success': True, 'message': 'API key removed'},
                        metadata={},
                        status=ResponseStatus.SUCCESS
                    )
            
            elif action == 'list':
                # Don't return the actual keys, just metadata
                keys_info = {k: {'owner': v['owner']} for k, v in self.api_keys.items()}
                return ApiResponse(
                    data={'keys': keys_info},
                    metadata={},
                    status=ResponseStatus.SUCCESS
                )
                
            return ApiResponse(
                data={'error': 'Invalid action or missing parameters'},
                metadata={},
                status=ResponseStatus.ERROR
            )
            
        except Exception as e:
            logger.error(f"Error handling keys API: {str(e)}")
            return ApiResponse(
                data={'error': str(e)},
                metadata={},
                status=ResponseStatus.ERROR
            )
    
    def _handle_proxy_request(self, request):
        """Handle a proxied API request."""
        try:
            # Extract the path without the /proxy/ prefix
            path = request.path[7:]  # Skip "/proxy/"
            
            # Check if this path is registered
            if path not in self.routes:
                return ApiResponse(
                    data={'error': 'Route not found'},
                    metadata={},
                    status=ResponseStatus.NOT_FOUND
                )
                
            route = self.routes[path]
            
            # Check authentication if required
            if route.get('auth_required', True):
                api_key = request.metadata.get('api_key')
                if not api_key or api_key not in self.api_keys:
                    return ApiResponse(
                        data={'error': 'Unauthorized - Invalid API key'},
                        metadata={'www-authenticate': 'APIKey'},
                        status=ResponseStatus.ERROR
                    )
                
                # Check rate limits
                if api_key in self.rate_limits:
                    limit = self.rate_limits[api_key]
                    if api_key not in self.request_counts:
                        self.request_counts[api_key] = 0
                        
                    if self.request_counts[api_key] >= limit:
                        return ApiResponse(
                            data={'error': 'Rate limit exceeded'},
                            metadata={'retry-after': '60'},
                            status=ResponseStatus.ERROR
                        )
                        
                    self.request_counts[api_key] += 1
            
            # Forward the request to the target
            target = route['target']
            method = request.metadata.get('method', 'GET')
            headers = {k: v for k, v in request.metadata.items() 
                      if k not in ['method', 'api_key']}
                      
            # Prepare the request
            req = Request(
                url=f"{target}/{path}",
                data=json.dumps(request.data).encode('utf-8') if request.data else None,
                headers=headers,
                method=method
            )
            
            # Make the request with SSL context
            ctx = ssl.create_default_context()
            response = urlopen(req, context=ctx)
            
            # Process the response
            status_code = response.status
            response_data = response.read().decode('utf-8')
            response_headers = dict(response.getheaders())
            
            try:
                response_json = json.loads(response_data)
            except:
                response_json = response_data
                
            return ApiResponse(
                data=response_json,
                metadata=response_headers,
                status=ResponseStatus.SUCCESS
            )
            
        except HTTPError as e:
            logger.error(f"HTTP error during proxy: {str(e)}")
            return ApiResponse(
                data={'error': f'Backend server error: {str(e)}'},
                metadata={'status_code': str(e.code)},
                status=ResponseStatus.ERROR
            )
            
        except URLError as e:
            logger.error(f"URL error during proxy: {str(e)}")
            return ApiResponse(
                data={'error': f'Connection error: {str(e)}'},
                metadata={},
                status=ResponseStatus.ERROR
            )
            
        except Exception as e:
            logger.error(f"Error during proxy: {str(e)}")
            return ApiResponse(
                data={'error': str(e)},
                metadata={},
                status=ResponseStatus.ERROR
            )
    
    def _handle_health_check(self, request):
        """Handle health check requests."""
        return ApiResponse(
            data={'status': 'ok', 'version': '1.0.0'},
            metadata={},
            status=ResponseStatus.SUCCESS
        )
        
    def add_route(self, path, target, auth_required=True, rate_limit=0):
        """Add a route to the gateway."""
        self.routes[path] = {
            'target': target,
            'auth_required': auth_required,
            'rate_limit': rate_limit
        }
        logger.info(f"Added route: {path} -> {target} (auth: {auth_required}, rate limit: {rate_limit})")
        
    def set_api_key(self, key, owner, permissions=None):
        """Set an API key."""
        if permissions is None:
            permissions = []
            
        self.api_keys[key] = {
            'owner': owner,
            'permissions': permissions
        }
        logger.info(f"Added API key for {owner}")
        
    def set_rate_limit(self, key, limit):
        """Set a rate limit for an API key."""
        self.rate_limits[key] = limit
        logger.info(f"Set rate limit for {key} to {limit} requests per minute")
        
    def start(self):
        """Start the API gateway HTTP server."""
        class GatewayRequestHandler(BaseHTTPRequestHandler):
            gateway = self
            
            def do_GET(self):
                self._handle_request('GET')
                
            def do_POST(self):
                self._handle_request('POST')
                
            def do_PUT(self):
                self._handle_request('PUT')
                
            def do_DELETE(self):
                self._handle_request('DELETE')
                
            def _handle_request(self, method):
                try:
                    # Read request body if present
                    content_length = int(self.headers.get('Content-Length', 0))
                    body = self.rfile.read(content_length) if content_length else b''
                    
                    # Parse JSON body if possible
                    if body and self.headers.get('Content-Type') == 'application/json':
                        try:
                            body_data = json.loads(body)
                        except:
                            body_data = body.decode('utf-8')
                    else:
                        body_data = body.decode('utf-8') if body else None
                    
                    # Convert headers to metadata
                    metadata = {k: v for k, v in self.headers.items()}
                    metadata['method'] = method
                    
                    # Create API request
                    if self.path.startswith('/proxy/'):
                        api_path = self.path
                    else:
                        api_path = f"/proxy{self.path}"
                        
                    request = ApiRequest(
                        path=api_path,
                        data=body_data,
                        metadata=metadata,
                        sender_id='http-client'
                    )
                    
                    # Process through the node
                    response = self.gateway.node.call_api(api_path, body_data, metadata)
                    
                    # Convert response to HTTP
                    status_code = 200
                    if response.status == ResponseStatus.NOT_FOUND:
                        status_code = 404
                    elif response.status == ResponseStatus.ERROR:
                        status_code = 500
                        
                    # Set status code
                    self.send_response(status_code)
                    
                    # Set headers from metadata
                    for k, v in response.metadata.items():
                        if k.lower() not in ['content-length', 'connection']:
                            self.send_header(k, v)
                            
                    # Set content type if not already set
                    if 'Content-Type' not in response.metadata:
                        self.send_header('Content-Type', 'application/json')
                        
                    # Serialize response data
                    if isinstance(response.data, dict) or isinstance(response.data, list):
                        response_body = json.dumps(response.data).encode('utf-8')
                    elif isinstance(response.data, str):
                        response_body = response.data.encode('utf-8')
                    else:
                        response_body = str(response.data).encode('utf-8')
                        
                    self.send_header('Content-Length', str(len(response_body)))
                    self.end_headers()
                    
                    # Send response body
                    self.wfile.write(response_body)
                    
                except Exception as e:
                    logger.error(f"Error handling HTTP request: {str(e)}")
                    self.send_error(500, str(e))
        
        # Create HTTPS server with SSL context
        server_address = (self.bind_address, self.port)
        httpd = HTTPServer(server_address, GatewayRequestHandler)
        
        # Set up SSL
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(self.cert_path, self.key_path)
        httpd.socket = context.wrap_socket(httpd.socket, server_side=True)
        
        logger.info(f"Starting secure API gateway on {self.bind_address}:{self.port}")
        httpd.serve_forever()
        
def main():
    """Main entry point for the API gateway."""
    parser = argparse.ArgumentParser(description='Secure API Gateway')
    parser.add_argument('--bind', default='localhost', help='Address to bind to')
    parser.add_argument('--port', type=int, default=8443, help='Port to listen on')
    parser.add_argument('--cert', required=True, help='Path to TLS certificate')
    parser.add_argument('--key', required=True, help='Path to TLS key')
    args = parser.parse_args()
    
    # Create and configure the gateway
    gateway = SecureApiGateway(
        cert_path=args.cert,
        key_path=args.key,
        bind_address=args.bind,
        port=args.port
    )
    
    # Add some example routes
    gateway.add_route('api/users', 'https://api.example.com/users', auth_required=True)
    gateway.add_route('api/products', 'https://api.example.com/products', auth_required=False)
    
    # Add an example API key
    gateway.set_api_key('test-api-key-1234', 'Test User', ['api/users'])
    gateway.set_rate_limit('test-api-key-1234', 100)  # 100 requests per minute
    
    # Start the gateway
    gateway.start()
    
if __name__ == '__main__':
    main()