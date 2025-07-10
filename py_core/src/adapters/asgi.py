# Esboço para: py_core/src/adapters/asgi.py

class ASGIAdapter:
    def __init__(self, asgi_app):
        self.asgi_app = asgi_app

    async def handle(self, scope: dict, receive: callable, send: callable):
        # A chamada é quase direta, pois as interfaces são compatíveis
        await self.asgi_app(scope, receive, send)