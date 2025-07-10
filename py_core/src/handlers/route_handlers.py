# Conteúdo para: py_core/src/handlers/route_handlers.py
from utils.http_response import build_response

def handle_root_request(method, path, headers, body) -> bytes:
    return build_response(200, body=b"Handler da Rota Raiz (/) foi chamado.")

def handle_api_request(method, path, headers, body) -> bytes:
    return build_response(200, body=f"Handler de API para o caminho: {path}".encode())

def handle_static_request(method, path, headers, body) -> bytes:
    # A lógica real virá na Issue #024
    return build_response(200, body=f"Handler de arquivo estatico para: {path}".encode())

def handle_not_found(method, path, headers, body) -> bytes:
    return build_response(404, body=f"404 Not Found: {path}".encode())