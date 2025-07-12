# Conteúdo para: core/http_types.py

from dataclasses import dataclass, field
from typing import Dict, Any, List

@dataclass
class Request:
    """Encapsula todos os dados de uma requisição HTTP recebida."""
    method: str
    path: str
    path_only: str
    query_params: Dict[str, List[str]]
    version: str
    headers: Dict[str, str]
    body: bytes
    remote_addr: str = ""

@dataclass
class Response:
    """Encapsula todos os dados de uma resposta HTTP a ser enviada."""
    status_code: int
    headers: Dict[str, str] = field(default_factory=dict)
    set_cookies: List[str] = field(default_factory=list)
    body: bytes = b''