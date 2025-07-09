# Conteúdo para: py_core/src/handlers/cookie_handler.py

from http.cookies import SimpleCookie
from typing import Dict, Optional

def parse_cookies(headers: Dict[str, str]) -> Dict[str, str]:
    """
    Extrai os cookies do cabeçalho 'cookie' de uma requisição.
    """
    cookie_header = headers.get('cookie')
    if not cookie_header:
        return {}
    
    cookie = SimpleCookie()
    cookie.load(cookie_header)
    
    # SimpleCookie armazena objetos Morsel, extraímos apenas os valores.
    return {k: v.value for k, v in cookie.items()}

def create_session_cookie(session_id: str) -> str:
    """
    Cria o valor para o cabeçalho 'Set-Cookie' para uma nova sessão.
    """
    cookie = SimpleCookie()
    cookie['session_id'] = session_id
    cookie['session_id']['path'] = '/'
    cookie['session_id']['httponly'] = True
    # Adicionar 'secure' = True e 'samesite' = 'Lax' ou 'Strict' em produção
    
    return cookie.output(header='').strip()