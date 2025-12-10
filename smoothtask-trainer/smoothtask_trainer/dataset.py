"""Чтение снапшотов из SQLite и формирование датасета для обучения."""

from pathlib import Path

import pandas as pd


def load_snapshots_as_frame(db_path: Path) -> pd.DataFrame:
    """
    Загружает снапшоты из SQLite в pandas DataFrame.
    
    TODO: реализовать чтение из SQLite с правильной структурой таблиц.
    """
    # TODO: подключение к БД, чтение снапшотов
    # return pd.read_sql(...)
    raise NotImplementedError("TODO: реализовать загрузку снапшотов")


