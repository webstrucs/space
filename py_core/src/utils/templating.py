# Conteúdo para: py_core/src/utils/templating.py

from pathlib import Path

def render(template_name: str, context: dict) -> bytes:
    """
    Renderiza um template HTML simples substituindo placeholders.
    Exemplo de placeholder no HTML: {{ username }}
    """
    try:
        template_path = Path(__file__).parent.parent.parent.joinpath("wsd", template_name).resolve()

        with open(template_path, 'r', encoding='utf-8') as f:
            content = f.read()

        for key, value in context.items():
            content = content.replace(f"{{{{ {key} }}}}", str(value))

        return content.encode('utf-8')
    except FileNotFoundError:
        return b"Erro 500: Template nao encontrado."
    except Exception as e:
        return f"Erro 500: {e}".encode('utf-8')