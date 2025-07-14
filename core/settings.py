# space/core/settings.py

from pathlib import Path

PROJECT_ROOT = Path(__file__).parent.parent.resolve()
DB_PATH = PROJECT_ROOT / "space_database.db"
STATIC_ROOT_PATH = PROJECT_ROOT / "works" / "wse"
WSD_ROOT_PATH = PROJECT_ROOT / "works" / "wsd"