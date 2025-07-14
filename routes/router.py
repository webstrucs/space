# Conteúdo final e funcional para: routes/router.py

import re
from handlers import route_handlers
from config import route_loader
from core.interfaces import Application

class Router:
    """
    O Roteador carrega as regras de um arquivo de configuração e mapeia
    um caminho de URL para a classe de Aplicação correta.
    """
    def __init__(self):
        self.routes: list[tuple[re.Pattern, Application]] = []
        self.not_found_handler = route_handlers.NotFoundApplication()
        self._load_routes()
        print("[ROUTER] Roteador inicializado e rotas carregadas.")

    def _load_routes(self):
        """
        Carrega as rotas do arquivo de configuração e as prepara para uso.
        """
        handler_registry = {
            "RootApplication": route_handlers.RootApplication,
            "ApiApplication": route_handlers.ApiApplication,
            "StaticFileApplication": route_handlers.StaticFileApplication,
            "LoginApplication": route_handlers.LoginApplication,
            "ProfileApplication": route_handlers.ProfileApplication,
        }
        raw_routes = route_loader.load_routes_from_config()
        for route_info in raw_routes:
            handler_name = route_info.get("handler")
            handler_class = handler_registry.get(handler_name)
            if not handler_class:
                print(f"[ROUTER] Aviso: Handler '{handler_name}' definido em routes.json não foi encontrado no registro.")
                continue
            try:
                pattern = re.compile(route_info["path_regex"])
                self.routes.append((pattern, handler_class()))
            except re.error as e:
                print(f"[ROUTER] Erro de Regex na rota para '{handler_name}': {e}")

    def resolve_route(self, path: str) -> Application:
        """
        Encontra a instância da aplicação correta para um dado caminho (path).
        """
        for pattern, handler_instance in self.routes:
            if pattern.match(path):
                return handler_instance
        return self.not_found_handler

# --- CORREÇÃO DEFINITIVA ---
# Cria uma instância única do roteador para ser importada por toda a aplicação.
# Esta linha estava faltando.
router_instance = Router()