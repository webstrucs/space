# Conteúdo final e corrigido para: py_core/src/main.py

import asyncio
import os
import struct
from typing import Dict, Any

from ipc.protocol import deserialize_low_level_message, serialize_high_level_message
from handlers.http_handler import parse_http_request

IPC_SOCKET_PATH = "/tmp/space_server.sock"

async def handle_ipc_client(reader: asyncio.StreamReader, writer: asyncio.StreamWriter):
    peer = writer.get_extra_info('peername')
    print(f"[PYTHON] Conexão IPC recebida de: {peer}")

    try:
        len_bytes = await reader.readexactly(8)
        msg_len = struct.unpack('<Q', len_bytes)[0]
        data = await reader.readexactly(msg_len)
        
        message = deserialize_low_level_message(data)
        print(f"[PYTHON] Mensagem IPC desserializada: {message['type']} para conn_id={message['conn_id']}")
        
        http_request_data = message['data']
        parsed_request = parse_http_request(http_request_data)

        if parsed_request:
            # CORREÇÃO: Agora desempacotamos os 5 valores, incluindo o 'body'.
            method, path, version, headers, body = parsed_request
            
            print("--- Requisição HTTP Detalhada ---")
            print(f"  Método: {method}")
            print(f"  Caminho: {path}")
            print(f"  Versão: {version}")
            print(f"  Cabeçalhos: {headers}")
            # CORREÇÃO: Adicionamos o print do corpo da requisição.
            print(f"  Corpo (bytes): {body!r}")
            print("---------------------------------")
        else:
            print("[PYTHON] Falha ao fazer o parse da requisição HTTP.")

        conn_id = message['conn_id']
        # A resposta pode continuar a mesma por enquanto
        response_body = b"Request parsed and body received by Python!"
        serialized_response = serialize_high_level_message(conn_id, response_body)
        
        response_len = len(serialized_response)
        len_bytes = struct.pack('<Q', response_len)

        writer.write(len_bytes)
        writer.write(serialized_response)
        await writer.drain()
        print(f"[PYTHON] Resposta enviada de volta para o Rust para conn_id={conn_id}")

    except (ValueError, IndexError, asyncio.IncompleteReadError) as e:
        print(f"[PYTHON] Erro ao processar mensagem IPC: {e}")
    finally:
        print("[PYTHON] Fechando a conexão IPC.")
        writer.close()
        await writer.wait_closed()


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