# Conteúdo para: py_core/src/security/waf.py

import re
from urllib.parse import unquote

# Regex específica para o padrão de ataque que estamos testando
SQLI_REGEX = re.compile(r"'.*OR.*'.*='", re.IGNORECASE)

def inspect_request_data(query_params: dict) -> bool:
    """Inspeciona os parâmetros de query em busca de ameaças."""
    for key in query_params:
        for value in query_params[key]:
            # Decodifica cada valor antes de verificar
            decoded_value = unquote(value)
            if SQLI_REGEX.search(decoded_value):
                print(f"[WAF] Ameaça SQLi detectada no parâmetro '{key}': {decoded_value}")
                return False
    return True