import sys
from pathlib import Path

# Adiciona a raiz do projeto ao path do Python para resolver os imports
project_root = Path(__file__).parent.resolve()
sys.path.insert(0, str(project_root))

import asyncio
import os
import struct

from core.http_types import Request, Response
from core.http_response import build_response
from core.waf import inspect_request_data
from handlers.http_handler import parse_http_request
from interface.protocol import deserialize_low_level_message, serialize_high_level_message
from routes.router import router_instance as router

IPC_SOCKET_PATH = "/tmp/space_server.sock"

async def handle_ipc_client(reader: asyncio.StreamReader, writer: asyncio.StreamWriter):
    try:
        len_bytes = await reader.readexactly(8)
        msg_len = struct.unpack('<Q', len_bytes)[0]
        data = await reader.readexactly(msg_len)
        
        message = deserialize_low_level_message(data)
        parsed_request = parse_http_request(message['data'])

        response_obj: Response

        if not parsed_request:
            response_obj = Response(status_code=400, body=b"Bad Request")
        else:
            method, path, query_params, version, headers, body = parsed_request
            
            request_obj = Request(
                method=method, path=path, query_params=query_params,
                version=version, headers=headers, body=body,
                remote_addr=message.get('remote_addr', '')
            )
            
            if not inspect_request_data(request_obj.query_params, request_obj.body):
                response_obj = Response(status_code=403, body=b"Forbidden: Potential threat detected.")
            else:
                application_handler = router.resolve_route(request_obj.path)
                response_obj = await application_handler.handle(request_obj)
        
        response_bytes = build_response(
            status_code=response_obj.status_code,
            headers=response_obj.headers,
            set_cookies=response_obj.set_cookies,
            body=response_obj.body
        )
        
        conn_id = message['conn_id']
        serialized_response_to_rust = serialize_high_level_message(conn_id, response_bytes)
        
        len_bytes = struct.pack('<Q', len(serialized_response_to_rust))
        writer.write(len_bytes)
        writer.write(serialized_response_to_rust)
        await writer.drain()

    except Exception as e:
        print(f"[PYTHON] Erro no handle_ipc_client: {e}")
    finally:
        if 'writer' in locals() and not writer.is_closing():
            writer.close()
            await writer.wait_closed()


async def main():
    if os.path.exists(IPC_SOCKET_PATH):
        try: os.remove(IPC_SOCKET_PATH)
        except OSError as e: print(f"Erro ao remover socket antigo: {e}"); return
            
    server = await asyncio.start_unix_server(handle_ipc_client, path=IPC_SOCKET_PATH)
    addr = server.sockets[0].getsockname() if server.sockets else IPC_SOCKET_PATH
    print(f'[PYTHON] Servidor IPC ouvindo em: {addr}')
    async with server:
        await server.serve_forever()

if __name__ == "__main__":
    try: asyncio.run(main())
    except KeyboardInterrupt: print("\n[PYTHON] Servidor IPC encerrado.")