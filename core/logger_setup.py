# space/core/logger_setup.py

import logging
import sys

def setup_logging():
    """
    Configura o logging para a aplicação, definindo o formato
    e o nível padrão.
    """
    log_format = "%(asctime)s - [%(levelname)s] - %(name)s - (%(filename)s).%(funcName)s(%(lineno)d) - %(message)s"

    logging.basicConfig(
        level=logging.INFO,
        format=log_format,
        stream=sys.stdout, # Envia os logs para a saída padrão (console)
    )

    logging.info("Sistema de Logging configurado.")