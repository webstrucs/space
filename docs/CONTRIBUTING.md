## Política de Testes (Camada Python)

Para garantir a qualidade e a estabilidade da camada de aplicação em Python, todos os novos códigos devem ser acompanhados de testes.

### 1. Ferramentas
-   **Framework de Teste:** Utilizamos o módulo nativo `unittest` do Python.
-   **Execução:** Os testes são executados a partir da raiz do projeto (`space/`) com o comando: `python3 -m unittest discover tests/python`

### 2. Estrutura de Diretórios
-   Todos os testes da camada Python devem residir no diretório `tests/python/`.
-   A estrutura de diretórios dentro de `tests/python/` deve espelhar a estrutura de `core/`, `handlers/`, etc.
    ```
    tests/
    └── python/
        ├── test_handlers/
        │   └── test_route_handlers.py
        └── test_core/
            └── test_waf.py
    ```

### 3. Convenções
-   **Nomes de Arquivos:** Devem começar com `test_` (ex: `test_waf.py`).
-   **Nomes de Métodos:** Devem começar com `test_` (ex: `def test_rejeita_sqli(self):`).
-   **Tipos de Testes:**
    -   **Testes Unitários:** Devem testar uma única função ou método de forma isolada, sem depender do sistema de arquivos ou da rede. Use "mocks" para simular dependências.
    -   **Testes de Integração:** Devem testar a interação entre múltiplos módulos (ex: Roteador -> Handler).

### 4. Exemplo de Teste Unitário (`tests/python/test_core/test_waf.py`)
```python
import unittest
# Adiciona a raiz do projeto ao path para encontrar os módulos
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent.parent.parent.parent))

from core.waf import inspect_request_data

class TestWAF(unittest.TestCase):

    def test_bloqueia_sqli_simples(self):
        """Verifica se o WAF bloqueia uma tentativa clara de SQL Injection."""
        query_params = {'id': ["1' OR '1'='1"]}
        body = b''
        self.assertFalse(inspect_request_data(query_params, body), "WAF falhou em bloquear SQLi")

    def test_permite_query_segura(self):
        """Verifica se o WAF permite uma query string normal."""
        query_params = {'id': ["123"]}
        body = b''
        self.assertTrue(inspect_request_data(query_params, body), "WAF bloqueou uma requisicao legitima")

if __name__ == '__main__':
    unittest.main()