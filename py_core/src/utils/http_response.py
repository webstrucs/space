# Conteúdo final para: py_core/src/utils/http_response.py

from http import HTTPStatus
from typing import Dict, Optional, List

def build_response(
    status_code: int,
    headers: Optional[Dict[str, str]] = None,
    set_cookies: Optional[List[str]] = None,
    body: bytes = b''
) -> bytes:
    """
    Constrói uma resposta HTTP/1.1 completa, incluindo importantes
    cabeçalhos de segurança por padrão.
    """
    try:
        status = HTTPStatus(status_code)
        status_line = f"HTTP/1.1 {status.value} {status.phrase}\r\n"
    except ValueError:
        status_line = f"HTTP/1.1 {status_code}\r\n"

    response_headers = headers or {}
    
    # --- ADIÇÃO DOS CABEÇALHOS DE SEGURANÇA ---
    # Previne Clickjacking
    response_headers['X-Frame-Options'] = 'DENY'
    # Previne ataques de MIME sniffing
    response_headers['X-Content-Type-Options'] = 'nosniff'
    # Define uma Política de Segurança de Conteúdo (CSP) restritiva
    response_headers['Content-Security-Policy'] = "default-src 'self'; frame-ancestors 'none';"
    # -----------------------------------------------

    response_headers['Content-Length'] = str(len(body))
    if 'Content-Type' not in response_headers:
        response_headers['Content-Type'] = 'text/plain; charset=utf-8'
    
    headers_str = "".join([f"{k}: {v}\r\n" for k, v in response_headers.items()])
    
    if set_cookies:
        for cookie_str in set_cookies:
            headers_str += f"Set-Cookie: {cookie_str}\r\n"

    return status_line.encode('iso-8859-1') + \
           headers_str.encode('iso-8859-1') + \
           b"\r\n" + \
           body