# Conteúdo final e corrigido para: py_core/src/database/user_db.py

import sqlite3
import hashlib
from pathlib import Path
from typing import Optional, Dict, Any

# Usa exatamente a mesma lógica de caminho do init_db.py
DB_PATH = Path(__file__).parent.parent.parent.joinpath("space_database.db")

def get_user_for_login(username: str, password_raw: str) -> Optional[Dict[str, Any]]:
    """
    Verifica as credenciais do usuário no banco de dados.
    """
    try:
        password_hash = hashlib.sha256(password_raw.encode('utf-8')).hexdigest()
        
        con = sqlite3.connect(DB_PATH)
        con.row_factory = sqlite3.Row
        cur = con.cursor()
        
        res = cur.execute(
            "SELECT username, role FROM users WHERE username = ? AND password_hash = ?",
            (username, password_hash)
        ).fetchone()
        
        con.close()
        
        if res:
            return dict(res)
        return None

    except Exception as e:
        print(f"[DB] Erro ao buscar usuário: {e}")
        return None