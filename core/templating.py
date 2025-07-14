# space/core/templating.py

from core.settings import WSD_ROOT_PATH

def render(template_name: str, context: dict) -> bytes:
    """Renderiza um template HTML simples substituindo placeholders."""
    try:
        template_path = WSD_ROOT_PATH.joinpath(template_name)
        with open(template_path, 'r', encoding='utf-8') as f:
            content = f.read()
        for key, value in context.items():
            content = content.replace(f"{{{{ {key} }}}}", str(value))
        return content.encode('utf-8')
    except FileNotFoundError:
        return b"Erro 500: Template nao encontrado."
    except Exception as e:
        return f"Erro 500: {e}".encode('utf-8')