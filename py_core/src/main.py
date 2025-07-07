# Conteúdo final e corrigido para: py_core/src/main.py

import asyncio
import os
import struct # Módulo nativo para empacotar e desempacotar bytes
from ipc.protocol import deserialize_low_level_message, serialize_high_level_message

IPC_SOCKET_PATH = "/tmp/space_server.sock"

async def handle_ipc_client(reader: asyncio.StreamReader, writer: asyncio.StreamWriter):
    peer = writer.get_extra_info('peername')
    print(f"[PYTHON] Conexão IPC recebida de: {peer}")

    try:
        # --- CORREÇÃO: LÓGICA DE LEITURA COM PREFIXO DE TAMANHO ---
        # 1. Lê os primeiros 8 bytes, que contêm o tamanho da mensagem (u64).
        len_bytes = await reader.readexactly(8)
        if not len_bytes:
            print("[PYTHON] Conexão IPC fechada antes de enviar o tamanho.")
            return
        
        # 2. Desempacota esses 8 bytes para obter o número do tamanho.
        msg_len = struct.unpack('<Q', len_bytes)[0]
        
        # 3. Lê exatamente `msg_len` bytes para obter a mensagem completa.
        data = await reader.readexactly(msg_len)
        print(f"[PYTHON] Mensagem de {msg_len} bytes recebida.")
        
        # 4. Desserializa a mensagem
        message = deserialize_low_level_message(data)
        print(f"[PYTHON] Mensagem desserializada: {message['type']} para conn_id={message['conn_id']}")
        
        conn_id = message['conn_id']
        
        # --- CORREÇÃO: LÓGICA DE ESCRITA COM PREFIXO DE TAMANHO ---
        # 5. Serializa uma resposta de volta para o Rust
        response_body = b"Processed by Python!"
        serialized_response = serialize_high_level_message(conn_id, response_body)
        
        # 6. Pega o tamanho da resposta e empacota em 8 bytes.
        response_len = len(serialized_response)
        len_bytes = struct.pack('<Q', response_len)

        # 7. Envia o tamanho primeiro, depois a mensagem.
        writer.write(len_bytes)
        writer.write(serialized_response)
        await writer.drain()
        print(f"[PYTHON] Resposta de {response_len} bytes enviada para conn_id={conn_id}")

    except (ValueError, IndexError, asyncio.IncompleteReadError) as e:
        print(f"[PYTHON] Erro ao processar mensagem IPC: {e}")
    finally:
        print("[PYTHON] Fechando a conexão IPC.")
        writer.close()
        await writer.wait_closed()


async def main():
    if os.path.exists(IPC_SOCKET_PATH):
        os.remove(IPC_SOCKET_PATH)
    server = await asyncio.start_unix_server(handle_ipc_client, path=IPC_SOCKET_PATH)
    addr = server.sockets[0].getsockname()
    print(f'[PYTHON] Servidor IPC ouvindo em: {addr}')
    async with server:
        await server.serve_forever()

if __name__ == "__main__":
    asyncio.run(main())