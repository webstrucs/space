# space/core/http_types.py

from dataclasses import dataclass, field
from typing import Dict, List, Any

@dataclass
class Request:
    method: str
    path: str
    query_params: Dict[str, List[str]]
    version: str
    headers: Dict[str, str]
    body: bytes
    remote_addr: str = ""

@dataclass
class Response:
    status_code: int
    headers: Dict[str, str] = field(default_factory=dict)
    set_cookies: List[str] = field(default_factory=list)
    body: bytes = b''