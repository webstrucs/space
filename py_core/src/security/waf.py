# Conteúdo final e funcional para: py_core/src/security/waf.py

import re
from urllib.parse import unquote_plus

# Regex mais robusta que procura por palavras-chave e caracteres perigosos de SQL,
# ignorando se são maiúsculas ou minúsculas.
SQLI_REGEX = re.compile(
    r"\s*(union|select|drop|insert|update|delete|'|--|#|;)", 
    re.IGNORECASE
)

# A proteção contra Path Traversal será feita de forma correta na Issue #024.
# Por enquanto, focamos no que podemos detectar de forma confiável aqui.

def inspect_path_for_sqli(path: str) -> bool:
    """
    Inspeciona o caminho da URL (incluindo query string) em busca de padrões de SQLi.
    Retorna True se for seguro, False se uma ameaça for detectada.
    """
    try:
        # Decodifica caracteres como %27 para ' e %20 para espaço.
        decoded_path = unquote_plus(path)
        
        # Procura por qualquer um dos padrões perigosos na URL decodificada.
        if SQLI_REGEX.search(decoded_path):
            print(f"[WAF] Ameaça potencial (padrão SQL) detectada em: {decoded_path}")
            return False
            
        print(f"[WAF] Nenhuma ameaça de SQLi detectada em: {decoded_path}")
        return True

    except Exception as e:
        print(f"[WAF] Erro ao decodificar o path: {e}")
        # Considera qualquer erro de decodificação como uma tentativa maliciosa.
        return False