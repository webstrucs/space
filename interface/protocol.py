# Conteúdo completo para: py_core/src/ipc/protocol.py

import struct
from typing import Dict, Any, Tuple

def deserialize_low_level_message(data: bytes) -> Dict[str, Any]:
    """
    Desserializa uma mensagem no formato bincode enviada pelo Rust.
    Formato esperado para `LowLevelMessage::Data` agora é:
    - 4 bytes: Índice da variante do enum (1 para 'Data')
    - 8 bytes: conn_id (u64)
    - 8 bytes: tamanho da string remote_addr (u64)
    - N bytes: string remote_addr em utf-8
    - 8 bytes: tamanho do vetor de dados (u64)
    - M bytes: os dados da requisição em si
    """
    # Desempacota o cabeçalho inicial: índice, conn_id, e tamanho do endereço
    variant_index, conn_id, addr_len = struct.unpack('<IQQ', data[:20])
    
    # Calcula onde o endereço termina e o extrai
    addr_end = 20 + int(addr_len)
    remote_addr = data[20:addr_end].decode('utf-8')
    
    # Calcula onde os dados da requisição começam e os extrai
    data_len_bytes = data[addr_end:addr_end + 8]
    data_len = struct.unpack('<Q', data_len_bytes)[0]
    
    request_data = data[addr_end + 8:]

    # Validações
    if variant_index != 1:
        raise ValueError(f"Variante de mensagem inesperada: {variant_index}")
    if len(request_data) != data_len:
        raise ValueError("Tamanho dos dados da requisição inconsistente")

    return {
        "type": "Data",
        "conn_id": conn_id,
        "remote_addr": remote_addr,
        "data": request_data
    }

def serialize_high_level_message(conn_id: int, response_data: bytes) -> bytes:
    """
    Serializa uma mensagem de resposta no formato bincode para o Rust.
    """
    variant_index = 0
    data_len = len(response_data)
    
    header = struct.pack('<IQQ', variant_index, conn_id, data_len)
    return header + response_data