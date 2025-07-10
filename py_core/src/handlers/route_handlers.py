# Conteúdo final, definitivo e funcional para: py_core/src/handlers/route_handlers.py

import os
import mimetypes
from pathlib import Path
from email.utils import formatdate
from utils.http_response import build_response

def handle_root_request(method, path, query, version, headers, body) -> bytes:
    return build_response(200, body=b"Handler da Rota Raiz (/) foi chamado.")

def handle_api_request(method, path, query, version, headers, body) -> bytes:
    return build_response(200, body=f"Handler de API para o caminho: {path}".encode())

def handle_not_found(method, path, query, version, headers, body) -> bytes:
    return build_response(404, body=f"404 Not Found: {path}".encode())

def handle_static_request(method: str, path: str, query, version, headers, body) -> bytes:
    """
    Implementação final e segura para servir arquivos estáticos.
    """
    try:
        # CORREÇÃO DEFINITIVA: Construção do caminho absoluto e robusto.
        # __file__ é o caminho para este arquivo (.../py_core/src/handlers/route_handlers.py)
        # .parent sobe um nível para a pasta 'handlers'
        # .parent.parent sobe para a pasta 'src'
        # .parent.parent.parent sobe para a pasta 'py_core', onde 'wse' se encontra.
        static_root = Path(__file__).parent.parent.parent.joinpath("wse").resolve()
        
        relative_path = path.removeprefix("/static/").lstrip("/")
        
        # Constrói o caminho completo e resolve para sua forma canônica.
        requested_path = static_root.joinpath(relative_path).resolve()

        # A verificação de segurança: o caminho final ainda está dentro da nossa pasta 'wse'?
        if not requested_path.is_relative_to(static_root):
            print(f"[SECURITY] Path Traversal bloqueado para: {path}")
            return build_response(403, body=b"Forbidden")
        
        # Se for seguro, verifica se o arquivo existe e o serve.
        if requested_path.is_file():
            with open(requested_path, "rb") as f:
                file_body = f.read()
            mime_type, _ = mimetypes.guess_type(requested_path)
            response_headers = {"Content-Type": mime_type or "application/octet-stream"}
            response_headers["Last-Modified"] = formatdate(requested_path.stat().st_mtime, usegmt=True)
            return build_response(200, headers=response_headers, body=file_body)
        else:
            return handle_not_found(method, path, query, version, headers, body)

    except Exception as e:
        print(f"[STATIC HANDLER] Erro ao servir arquivo: {e}")
        return build_response(500, body=b"Internal Server Error")