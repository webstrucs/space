# Conteúdo para: py_core/src/ipc/protocol.py

import struct
from typing import Dict, Any, Tuple

# --- Desserialização (Rust -> Python) ---

def deserialize_low_level_message(data: bytes) -> Dict[str, Any]:
    """
    Desserializa uma mensagem no formato bincode enviada pelo Rust.
    Formato esperado para `LowLevelMessage::Data`:
    - 4 bytes: Índice da variante do enum (neste caso, 1 para 'Data')
    - 8 bytes: conn_id (u64)
    - 8 bytes: tamanho do vetor de dados (u64)
    - N bytes: os dados em si
    """
    # Desempacota os primeiros 20 bytes (4 + 8 + 8)
    # '<' indica little-endian
    # 'I' é um unsigned int de 4 bytes (para o índice do enum)
    # 'Q' é um unsigned long long de 8 bytes (para o u64)
    variant_index, conn_id, data_len = struct.unpack('<IQQ', data[:20])

    # Pega o resto dos bytes, que são os dados da requisição
    request_data = data[20:]

    # Validação simples
    if variant_index != 1: # Esperamos a variante 'Data', que tem índice 1
        raise ValueError(f"Variante de mensagem inesperada: {variant_index}")
    if len(request_data) != data_len:
        raise ValueError("Tamanho dos dados inconsistente")

    return {
        "type": "Data",
        "conn_id": conn_id,
        "data": request_data
    }

# --- Serialização (Python -> Rust) ---

def serialize_high_level_message(conn_id: int, response_data: bytes) -> bytes:
    """
    Serializa uma mensagem de resposta no formato bincode para o Rust.
    Formato para `HighLevelMessage::ResponseData`:
    - 4 bytes: Índice da variante (0 para 'ResponseData')
    - 8 bytes: conn_id (u64)
    - 8 bytes: tamanho do vetor de dados (u64)
    - N bytes: os dados da resposta
    """
    variant_index = 0
    data_len = len(response_data)

    header = struct.pack('<IQQ', variant_index, conn_id, data_len)
    return header + response_data