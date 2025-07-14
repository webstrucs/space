# space/config/init_db.py

import sys
from pathlib import Path
import sqlite3
import hashlib

# Adiciona a raiz do projeto ao path para encontrar o módulo 'core'
project_root = Path(__file__).parent.parent.resolve()
sys.path.insert(0, str(project_root))

from core.settings import DB_PATH

def initialize():
    print(f"Inicializando banco de dados em: {DB_PATH}")
    con = sqlite3.connect(DB_PATH)
    cur = con.cursor()
    cur.execute("""
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY,
            username TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL
        )
    """)
    admin_pass_hash = '5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8'
    try:
        cur.execute("INSERT INTO users (username, password_hash, role) VALUES ('admin', ?, 'admin')", (admin_pass_hash,))
        print("Usuário 'admin' inserido com sucesso.")
    except sqlite3.IntegrityError:
        print("Usuário 'admin' já existe.")
    con.commit()
    con.close()
    print(f"Banco de dados '{DB_PATH}' inicializado e pronto.")

if __name__ == "__main__":
    initialize()