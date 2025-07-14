# space/core/waf.py

import re
from urllib.parse import unquote

SQLI_REGEX = re.compile(r"'.*OR.*'.*='", re.IGNORECASE)

def inspect_request_data(query_params: dict, body: bytes) -> bool:
    for key in query_params:
        for value in query_params[key]:
            decoded_value = unquote(value)
            if SQLI_REGEX.search(decoded_value):
                print(f"[WAF] Ameaça SQLi detectada no parâmetro '{key}'")
                return False
    # Futuramente, adicionar inspeção do corpo
    return True