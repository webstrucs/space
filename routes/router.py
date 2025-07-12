# space/routes/router.py

import re
from handlers import route_handlers

# Agora todas as rotas apontam para instâncias de classes, de forma consistente.
ROUTE_RULES = [
    (re.compile(r"^/login$"), route_handlers.LoginApplication()),
    (re.compile(r"^/profile$"), route_handlers.ProfileApplication()),
    (re.compile(r"^/api/.*$"), route_handlers.ApiApplication()),
    (re.compile(r"^/static/.*$"), route_handlers.StaticFileApplication()),
    (re.compile(r"^/$"), route_handlers.RootApplication()),
]
NOT_FOUND_HANDLER = route_handlers.NotFoundApplication()

def resolve_route(path: str):
    for pattern, handler_instance in ROUTE_RULES:
        if pattern.match(path):
            return handler_instance
    return NOT_FOUND_HANDLER