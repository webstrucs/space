# Conteúdo para: py_core/src/routing/router.py

import re
from handlers import route_handlers

# Prioridade de rotas: API > Static > Root (o mais específico primeiro)
ROUTE_RULES = [
    (re.compile(r"^/api/.*$"), route_handlers.handle_api_request),
    (re.compile(r"^/static/.*$"), route_handlers.handle_static_request),
    (re.compile(r"^/$"), route_handlers.handle_root_request),
]

def resolve_route(path: str):
    """
    Encontra o handler correto para um dado caminho (path).
    """
    for pattern, handler in ROUTE_RULES:
        if pattern.match(path):
            return handler

    # Se nenhuma regra específica for encontrada, trata-se de um 404
    return route_handlers.handle_not_found