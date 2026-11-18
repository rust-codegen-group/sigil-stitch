from http.server import HTTPServer
from json import dumps


def start_server():
    srv = HTTPServer()
    data = dumps(srv)
