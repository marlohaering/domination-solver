#!/usr/bin/env python3
"""Serves web/ locally with the headers needed for multi-threaded solving."""
import functools
import http.server
import os

PORT = 8000


class Handler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        self.send_header("Cache-Control", "no-cache")
        super().end_headers()


web = os.path.join(os.path.dirname(os.path.abspath(__file__)), "web")
handler = functools.partial(Handler, directory=web)
print(f"Serving on http://localhost:{PORT}")
http.server.ThreadingHTTPServer(("localhost", PORT), handler).serve_forever()
