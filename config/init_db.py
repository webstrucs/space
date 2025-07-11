# Conteúdo final e corrigido para: py_core/src/database/init_db.py

import sqlite3
import hashlib
from pathlib import Path

# Constrói o caminho para a raiz do projeto e depois para o arquivo do banco de dados
DB_PATH = Path(__file__).parent.parent.parent.joinpath("space_database.db")

def initialize():
    # ... o resto da função continua igual ...
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
    # Hash SHA256 para a senha 'password'
    admin_pass_hash = '5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8'
    cur.execute("""
        INSERT OR IGNORE INTO users (username, password_hash, role)
        VALUES ('admin', ?, 'admin')
    """, (admin_pass_hash,))
    con.commit()
    con.close()
    print(f"Banco de dados '{DB_PATH}' inicializado com sucesso.")

if __name__ == "__main__":
    initialize()