# Conteúdo final, completo e unificado para: py_core/src/handlers/route_handlers.py

import json
import mimetypes
from pathlib import Path
from email.utils import formatdate

# Importa nossos módulos
from utils.http_response import build_response
from security import jwt_handler
from database import user_db
from services.session_manager import SessionManager # Importa o SessionManager
import secrets # Importa o secrets para o handle_login_request (caso de exemplo)

# --- Handlers de Rotas Básicas ---

def handle_root_request(method, path, query, version, headers, body):
    return build_response(200, body=b"Use POST /login para autenticar ou GET /profile com um token.")

def handle_api_request(method, path, query, version, headers, body):
    return build_response(200, body=f"API endpoint: {path}".encode())

def handle_not_found(method, path, query, version, headers, body):
    return build_response(404, body=f"404 Not Found: {path}".encode())

# --- Handler de Arquivo Estático (da Issue #024) ---

def handle_static_request(method: str, path: str, query, version, headers: dict, body: bytes) -> bytes:
    try:
        static_root = Path(__file__).parent.parent.joinpath("wse").resolve()
        relative_path = path.removeprefix("/static/").lstrip("/")
        requested_path = static_root.joinpath(relative_path).resolve()
        
        if not requested_path.is_relative_to(static_root):
            return build_response(403, body=b"Forbidden")
        
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
        print(f"[STATIC HANDLER] Erro: {e}")
        return build_response(500, body=b"Internal Server Error")

# --- Handlers de Conteúdo Dinâmico (da Issue #025) ---

def handle_login_request(method, path, query, version, headers, body) -> bytes:
    """Processa uma tentativa de login via POST com corpo JSON."""
    if method.upper() != 'POST':
        return build_response(405, body=b"Method Not Allowed")
    
    try:
        credentials = json.loads(body)
        username = credentials.get('username')
        password = credentials.get('password')
        
        if not username or not password:
            return build_response(400, body=b"Bad Request: Username and password are required.")

        # Verifica as credenciais no banco de dados
        user_data = user_db.get_user_for_login(username, password)
        
        if user_data:
            # Se as credenciais estiverem corretas, cria o token JWT
            payload = {"sub": user_data['username'], "role": user_data['role']}
            token = jwt_handler.create_token(payload)
            response_body = json.dumps({"token": token}).encode('utf-8')
            return build_response(200, headers={"Content-Type": "application/json"}, body=response_body)
        else:
            return build_response(401, body=b"Unauthorized: Invalid credentials")

    except json.JSONDecodeError:
        return build_response(400, body=b"Bad Request: Invalid JSON.")
    except Exception as e:
        return build_response(500, body=f"Internal Server Error: {e}".encode())

def handle_profile_page(method, path, query, version, headers, body) -> bytes:
    """Serve a página de perfil, protegida por JWT."""
    
    # CORREÇÃO: Procura pelo cabeçalho 'authorization' em minúsculas.
    # Nosso parser em http_handler.py já padroniza todas as chaves de cabeçalho para minúsculas.
    auth_header = headers.get('authorization')
    
    if not auth_header or not auth_header.lower().startswith('bearer '):
        return build_response(401, body=b"Unauthorized: Missing or malformed token.")
        
    # Pega o token, removendo 'Bearer ' (7 caracteres) do início da string.
    token = auth_header[7:]
    
    # Verifica o token
    payload = jwt_handler.verify_and_decode_token(token)
    
    if not payload:
        return build_response(401, body=b"Unauthorized: Invalid or expired token.")
        
    # Se o token for válido, renderiza o template com os dados do payload
    print(f"[AUTH] Acesso permitido para o usuario: {payload.get('sub')}")
    context = {
        "username": payload.get('sub', 'N/A'),
        "access_level": payload.get('role', 'N/A')
    }
    from utils.templating import render # Importação movida para dentro para evitar dependência circular
    html_body = render("profile.html", context)
    
    return build_response(200, headers={"Content-Type": "text/html; charset=utf-8"}, body=html_body)
