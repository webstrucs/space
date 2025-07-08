# Conteúdo final para: py_core/src/handlers/http_handler.py

import io
from http.client import parse_headers
from typing import Dict, Tuple, Optional, Any

def parse_http_request(request_data: bytes) -> Optional[Tuple[str, str, str, Dict[str, Any], bytes]]:
    """
    Faz o parse de uma requisição HTTP/1.x completa (incluindo o corpo).
    
    Retorna uma tupla com:
    (método, caminho, versão, cabeçalhos, corpo_da_requisição)
    ou None se o parse falhar.
    """
    try:
        request_file = io.BytesIO(request_data)
        
        # 1. Parse da Request-Line
        request_line = request_file.readline().decode('iso-8859-1').strip()
        method, path, version = request_line.split()

        # 2. Parse dos Cabeçalhos
        headers = parse_headers(request_file)
        header_dict = {k.lower(): v for k, v in headers.items()} # Converte chaves para minúsculas

        # 3. --- NOVA LÓGICA: LEITURA DO CORPO ---
        body = b'' # Corpo vazio por padrão
        if 'content-length' in header_dict:
            try:
                content_length = int(header_dict['content-length'])
                # O resto do `request_file` é o corpo. Lemos o tamanho especificado.
                body = request_file.read(content_length)
            except (ValueError, TypeError):
                print("[PYTHON PARSER] Valor de Content-Length inválido.")
                # Decide-se por tratar como se não houvesse corpo.
                pass

        print("[PYTHON PARSER] Requisição e corpo parseados com sucesso.")
        return method, path, version, header_dict, body

    except Exception as e:
        print(f"[PYTHON PARSER] Erro ao fazer o parse da requisição: {e}")
        return None