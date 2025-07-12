# Conteúdo para: core/interfaces.py

from abc import ABC, abstractmethod
from core.http_types import Request, Response

class Application(ABC):
    """
    Define o contrato para qualquer aplicação ou handler que o Servidor Space possa rodar.
    """
    @abstractmethod
    async def handle(self, request: Request) -> Response:
        """
        Processa um objeto Request e retorna um objeto Response.
        Deve ser uma função assíncrona.
        """
        pass