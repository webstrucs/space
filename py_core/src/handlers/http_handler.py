import io
from http.client import parse_headers
from urllib.parse import urlsplit, parse_qs
from typing import Dict, Tuple, Optional, Any

def parse_http_request(request_data: bytes) -> Optional[Tuple[str, str, Dict, str, Dict, bytes]]:
    try:
        request_file = io.BytesIO(request_data)
        request_line = request_file.readline().decode('iso-8859-1').strip()
        method, full_path, version = request_line.split(" ", 2)
        parsed_url = urlsplit(full_path)
        path = parsed_url.path
        query_params = parse_qs(parsed_url.query)
        headers = dict(parse_headers(request_file))
        content_length_str = headers.get('Content-Length', headers.get('content-length', '0'))
        body = b''
        if content_length_str.isdigit():
            content_length = int(content_length_str)
            if content_length > 0:
                body = request_file.read(content_length)
        return method, path, query_params, version, headers, body
    except Exception as e:
        print(f"[PYTHON PARSER] Erro no parse: {e}")
        return None