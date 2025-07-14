# Conteúdo para: config/route_loader.py
import json
from pathlib import Path

def load_routes_from_config():
    """
    Carrega e parseia o arquivo de configuração de rotas.
    """
    try:
        config_path = Path(__file__).parent.joinpath("routes.json").resolve()
        with open(config_path, 'r', encoding='utf-8') as f:
            routes_data = json.load(f)
        
        # Validação básica
        if not isinstance(routes_data, list):
            raise TypeError("A configuração de rotas deve ser uma lista de objetos.")
            
        print("[ROUTE_LOADER] Configuração de rotas carregada com sucesso.")
        return routes_data
    except Exception as e:
        print(f"[ROUTE_LOADER] Erro fatal ao carregar 'routes.json': {e}")
        return [] # Retorna uma lista vazia em caso de erro
    