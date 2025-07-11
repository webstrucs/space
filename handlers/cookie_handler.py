# Conteúdo final para: py_core/src/handlers/cookie_handler.py

from http.cookies import SimpleCookie
from typing import Dict, Optional

def parse_cookies(headers: Dict[str, str]) -> Dict[str, str]:
    cookie_header = headers.get('cookie')
    if not cookie_header:
        return {}
    cookie = SimpleCookie()
    cookie.load(cookie_header)
    return {k: v.value for k, v in cookie.items()}

def create_session_cookie(session_id: str) -> str:
    cookie = SimpleCookie()
    cookie['session_id'] = session_id
    cookie['session_id']['path'] = '/'
    cookie['session_id']['httponly'] = True
    cookie['session_id']['samesite'] = 'Lax' # Proteção contra CSRF
    # Em produção, adicione: cookie['session_id']['secure'] = True
    return cookie.output(header='').strip()