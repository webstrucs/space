# Conteúdo final e funcional para: py_core/src/main.py

import asyncio
import os
import struct

# Módulos do nosso projeto
from ipc.protocol import deserialize_low_level_message, serialize_high_level_message
from handlers.http_handler import parse_http_request
from handlers.cookie_handler import parse_cookies, create_session_cookie
from services.session_manager import SessionManager
from utils.http_response import build_response
from security import waf
from routing import router


IPC_SOCKET_PATH = "/tmp/space_server.sock"

async def handle_ipc_client(reader: asyncio.StreamReader, writer: asyncio.StreamWriter):
    """
    Callback que gerencia o ciclo completo de uma requisição,
    com a lógica de WAF e Roteamento integrada.
    """
    print("\n[PYTHON] Nova Conexão IPC recebida...")
    
    try:
        # 1. Leitura da mensagem vinda do Rust
        len_bytes = await reader.readexactly(8)
        msg_len = struct.unpack('<Q', len_bytes)[0]
        data = await reader.readexactly(msg_len)
        message = deserialize_low_level_message(data)
        
        http_request_data = message['data']
        parsed_request = parse_http_request(http_request_data)

        response_bytes = b'' # Inicializa a resposta como vazia

        if not parsed_request:
            response_bytes = build_response(400, body=b"Bad Request")
        else:
            method, path, version, headers, body = parsed_request

            # 2. WAF: A requisição é inspecionada ANTES de qualquer outra coisa.
            if not waf.inspect_path_for_sqli(path):
                # Se o WAF detectar uma ameaça, gera uma resposta 403 Forbidden.
                response_bytes = build_response(403, body=b"Forbidden: Potential threat detected.")
            else:
                # 3. Roteador: Se a requisição for segura, encontra o handler correto.
                handler = router.resolve_route(path)
                # 4. Executa o handler para gerar a resposta.
                response_bytes = handler(method, path, headers, body)
        
        # 5. Envia a resposta (seja do WAF ou do handler) de volta para o Rust.
        conn_id = message['conn_id']
        serialized_response_to_rust = serialize_high_level_message(conn_id, response_bytes)
        
        len_bytes = struct.pack('<Q', len(serialized_response_to_rust))
        writer.write(len_bytes)
        writer.write(serialized_response_to_rust)
        await writer.drain()
        print(f"[PYTHON] Resposta final enviada para o Rust.")

    except Exception as e:
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