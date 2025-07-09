# Conteúdo final e corrigido para: py_core/src/main.py

import asyncio
import os
import struct
import secrets
from typing import Dict, Any

from ipc.protocol import deserialize_low_level_message, serialize_high_level_message
from handlers.http_handler import parse_http_request
from handlers.cookie_handler import parse_cookies, create_session_cookie
from services.session_manager import SessionManager
from utils.http_response import build_response


IPC_SOCKET_PATH = "/tmp/space_server.sock"

async def handle_ipc_client(reader: asyncio.StreamReader, writer: asyncio.StreamWriter):
    """
    Callback que agora gerencia o ciclo completo de uma requisição,
    incluindo cookies e sessões.
    """
    print("\n[PYTHON] Nova Conexão IPC recebida...")
    
    try:
        len_bytes = await reader.readexactly(8)
        msg_len = struct.unpack('<Q', len_bytes)[0]
        data = await reader.readexactly(msg_len)
        
        message = deserialize_low_level_message(data)
        http_request_data = message['data']
        parsed_request = parse_http_request(http_request_data)

        if not parsed_request:
            # Em caso de falha no parse, envia um erro 400 Bad Request
            response_bytes = build_response(status_code=400, body=b"Bad Request")
        else:
            method, path, version, headers, body = parsed_request
            cookies = parse_cookies(headers)
            session_id = cookies.get('session_id')
            
            user_session = None
            if session_id:
                user_session = SessionManager.get_session(session_id)

            set_cookies_headers = []
            if user_session:
                username = user_session.get('username', 'amigo')
                response_body = f"Bem-vindo de volta, {username}!".encode('utf-8')
                print(f"[PYTHON] Usuário reconhecido pela sessão: {session_id}")
                response_bytes = build_response(status_code=200, body=response_body)
            else:
                new_session_data = {'username': f'visitante_{secrets.randbelow(1000)}'}
                new_session_id = SessionManager.create_session(new_session_data)
                set_cookies_headers.append(create_session_cookie(new_session_id))
                response_body = b"Ola! Uma nova sessao foi criada para voce."
                print(f"[PYTHON] Novo usuário. Criando sessão: {new_session_id}")
                response_bytes = build_response(
                    status_code=200,
                    set_cookies=set_cookies_headers,
                    body=response_body
                )

        # Envia a resposta HTTP (construída acima) de volta para o Rust
        conn_id = message['conn_id']
        serialized_response_to_rust = serialize_high_level_message(conn_id, response_bytes)
        
        len_bytes = struct.pack('<Q', len(serialized_response_to_rust))
        writer.write(len_bytes)
        writer.write(serialized_response_to_rust)
        await writer.drain()
        print(f"[PYTHON] Resposta HTTP completa enviada para o Rust.")

    except Exception as e:
        print(f"[PYTHON] Erro ao processar mensagem IPC: {e}")
    finally:
        print("[PYTHON] Fechando a conexão IPC.")
        writer.close()
        await writer.wait_closed()

# O resto do arquivo (função main, etc.) continua o mesmo
async def main():
    if os.path.exists(IPC_SOCKET_PATH):
        try:
            os.remove(IPC_SOCKET_PATH)
        except OSError as e:
            print(f"Erro ao remover socket antigo: {e}")
            return
    server = await asyncio.start_unix_server(handle_ipc_client, path=IPC_SOCKET_PATH)
    addr = server.sockets[0].getsockname() if server.sockets else IPC_SOCKET_PATH
    print(f'[PYTHON] Servidor IPC ouvindo em: {addr}')
    async with server:
        await server.serve_forever()

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n[PYTHON] Servidor IPC encerrado.")