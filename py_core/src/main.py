# Conteúdo final e funcional para: py_core/src/main.py

import asyncio
import os
import struct
from ipc.protocol import deserialize_low_level_message, serialize_high_level_message
from handlers.http_handler import parse_http_request
from utils.http_response import build_response
from security import waf
from routing import router

IPC_SOCKET_PATH = "/tmp/space_server.sock"

async def handle_ipc_client(reader: asyncio.StreamReader, writer: asyncio.StreamWriter):
    try:
        len_bytes = await reader.readexactly(8)
        msg_len = struct.unpack('<Q', len_bytes)[0]
        data = await reader.readexactly(msg_len)
        message = deserialize_low_level_message(data)
        parsed_request = parse_http_request(message['data'])

        if not parsed_request:
            response_bytes = build_response(400, body=b"Bad Request")
        else:
            method, path, query_params, version, headers, body = parsed_request
            
            if not waf.inspect_request_data(query_params):
                response_bytes = build_response(403, body=b"Forbidden: Potential threat detected.")
            else:
                handler = router.resolve_route(path)
                response_bytes = handler(method, path, query_params, version, headers, body)
        
        conn_id = message['conn_id']
        serialized_response = serialize_high_level_message(conn_id, response_bytes)
        writer.write(struct.pack('<Q', len(serialized_response)))
        writer.write(serialized_response)
        await writer.drain()
    except Exception as e:
        print(f"[PYTHON] Erro no handle_ipc_client: {e}")
    finally:
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