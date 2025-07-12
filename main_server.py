# Conteúdo para: main_server.py

import asyncio
import os
import struct

# Importa os módulos da nossa nova estrutura
from core.http_types import Request, Response
from interface.protocol import deserialize_low_level_message, serialize_high_level_message
from handlers.http_handler import parse_http_request
from core.http_response import build_response
from routes import router

IPC_SOCKET_PATH = "/tmp/space_server.sock"

async def handle_ipc_client(reader: asyncio.StreamReader, writer: asyncio.StreamWriter):
    """
    Orquestra o processamento de uma requisição usando a nova interface de aplicação.
    """
    try:
        # 1. Lê a mensagem IPC do Rust
        len_bytes = await reader.readexactly(8)
        msg_len = struct.unpack('<Q', len_bytes)[0]
        data = await reader.readexactly(msg_len)
        message = deserialize_low_level_message(data)
        
        # 2. Faz o parse da requisição HTTP para um objeto de dados
        parsed_request = parse_http_request(message['data'])

        response_obj: Response # Anotação de tipo para clareza

        if not parsed_request:
            response_obj = Response(status_code=400, body=b"Bad Request")
        else:
            method, path, query, version, headers, body = parsed_request
            
            # 3. Cria um objeto Request padronizado
            request_obj = Request(
                method=method,
                path=path,
                path_only=path.split('?')[0],
                query_params=query,
                version=version,
                headers=headers,
                body=body,
                remote_addr=message['remote_addr']
            )

            # O WAF seria chamado aqui no futuro, recebendo o request_obj
            # if not waf.inspect(request_obj):
            #     response_obj = Response(status_code=403, body=b"Forbidden")
            # else:

            # 4. O roteador retorna uma instância da aplicação a ser executada
            application_handler = router.resolve_route(request_obj.path_only)
            
            # 5. Executa o método .handle() da aplicação, que retorna um objeto Response
            response_obj = await application_handler.handle(request_obj)

        # 6. Usa o objeto Response para construir a resposta final em bytes
        response_bytes = build_response(
            status_code=response_obj.status_code,
            headers=response_obj.headers,
            set_cookies=response_obj.set_cookies,
            body=response_obj.body
        )
        
        # 7. Serializa e envia a resposta de volta para o Rust
        conn_id = message['conn_id']
        serialized_response_to_rust = serialize_high_level_message(conn_id, response_bytes)
        
        len_bytes = struct.pack('<Q', len(serialized_response_to_rust))
        writer.write(len_bytes)
        writer.write(serialized_response_to_rust)
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