# space/core/session_manager.py

import secrets
from typing import Dict, Any, Optional

_sessions: Dict[str, Dict[str, Any]] = {}

class SessionManager:
    @staticmethod
    def create_session(user_data: Dict[str, Any]) -> str:
        session_id = secrets.token_hex(16)
        _sessions[session_id] = user_data
        print(f"[SESSION] Nova sessão criada: {session_id} para o usuário {user_data.get('username')}")
        return session_id

    @staticmethod
    def get_session(session_id: str) -> Optional[Dict[str, Any]]:
        print(f"[SESSION] Buscando sessão: {session_id}")
        return _sessions.get(session_id)

    @staticmethod
    def delete_session(session_id: str) -> None:
        if session_id in _sessions:
            del _sessions[session_id]
            print(f"[SESSION] Sessão removida: {session_id}")