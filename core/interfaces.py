# space/core/interfaces.py

from abc import ABC, abstractmethod
from core.http_types import Request, Response

class Application(ABC):
    @abstractmethod
    async def handle(self, request: Request) -> Response:
        pass