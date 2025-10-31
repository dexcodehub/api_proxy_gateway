#!/usr/bin/env python3
"""
Simple HTTP upstream server for testing the Pingora proxy.
Provides various endpoints to test different scenarios.
"""

import json
import time
import random
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse, parse_qs
import sys

class TestUpstreamHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        parsed_path = urlparse(self.path)
        path = parsed_path.path
        query_params = parse_qs(parsed_path.query)
        
        if path == '/':
            self.send_basic_response("Hello from test upstream!")
            
        elif path == '/health':
            self.send_json_response({"status": "healthy", "timestamp": time.time()})
            
        elif path == '/delay':
            # Simulate slow response
            delay = float(query_params.get('ms', ['1000'])[0]) / 1000
            time.sleep(delay)
            self.send_json_response({"delayed": f"{delay}s", "timestamp": time.time()})
            
        elif path == '/error':
            # Simulate error responses
            error_code = int(query_params.get('code', ['500'])[0])
            self.send_response(error_code)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            response = {"error": f"Simulated error {error_code}", "timestamp": time.time()}
            self.wfile.write(json.dumps(response).encode())
            
        elif path == '/random':
            # Random success/failure for circuit breaker testing
            if random.random() < 0.3:  # 30% failure rate
                self.send_response(503)
                self.send_header('Content-type', 'application/json')
                self.end_headers()
                response = {"error": "Random failure", "timestamp": time.time()}
                self.wfile.write(json.dumps(response).encode())
            else:
                self.send_json_response({"status": "success", "timestamp": time.time()})
                
        elif path == '/echo':
            # Echo request information
            self.send_json_response({
                "method": self.command,
                "path": self.path,
                "headers": dict(self.headers),
                "timestamp": time.time()
            })
            
        else:
            self.send_response(404)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            response = {"error": "Not found", "path": path, "timestamp": time.time()}
            self.wfile.write(json.dumps(response).encode())

    def do_POST(self):
        content_length = int(self.headers.get('Content-Length', 0))
        post_data = self.rfile.read(content_length)
        
        try:
            body = json.loads(post_data.decode()) if post_data else {}
        except:
            body = {"raw": post_data.decode()}
            
        self.send_json_response({
            "method": "POST",
            "path": self.path,
            "headers": dict(self.headers),
            "body": body,
            "timestamp": time.time()
        })

    def send_basic_response(self, message):
        self.send_response(200)
        self.send_header('Content-type', 'text/plain')
        self.end_headers()
        self.wfile.write(message.encode())

    def send_json_response(self, data):
        self.send_response(200)
        self.send_header('Content-type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps(data, indent=2).encode())

    def log_message(self, format, *args):
        # Custom logging format
        print(f"[{time.strftime('%Y-%m-%d %H:%M:%S')}] {format % args}")

def run_server(port):
    server_address = ('', port)
    httpd = HTTPServer(server_address, TestUpstreamHandler)
    print(f"Test upstream server running on port {port}")
    print(f"Available endpoints:")
    print(f"  GET  /           - Basic hello response")
    print(f"  GET  /health     - Health check")
    print(f"  GET  /delay?ms=X - Delayed response (default 1000ms)")
    print(f"  GET  /error?code=X - Error response (default 500)")
    print(f"  GET  /random     - Random success/failure (30% failure)")
    print(f"  GET  /echo       - Echo request info")
    print(f"  POST /echo       - Echo request with body")
    print(f"  Press Ctrl+C to stop")
    
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print(f"\nShutting down server on port {port}")
        httpd.shutdown()

if __name__ == '__main__':
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8080
    run_server(port)