# space/core/user_db.py

import sqlite3
import hashlib
from typing import Optional, Dict, Any
from core.settings import DB_PATH

def get_user_for_login(username: str, password_raw: str) -> Optional[Dict[str, Any]]:
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