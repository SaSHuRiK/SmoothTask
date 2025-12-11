"""Чтение снапшотов из SQLite и формирование датасета для обучения."""

from __future__ import annotations

import json
import sqlite3
from pathlib import Path
from typing import Iterable

import pandas as pd

_PROCESS_BOOL_COLS = {
    "has_tty",
    "has_gui_window",
    "is_focused_window",
    "env_has_display",
    "env_has_wayland",
    "env_ssh",
    "is_audio_client",
    "has_active_stream",
}
_SNAPSHOT_BOOL_COLS = {"user_active", "bad_responsiveness"}
_APP_GROUP_BOOL_COLS = {"has_gui_window", "is_focused_group"}


def _json_list(value: str | None) -> list:
    """
    Парсит JSON-строку в список.

    Args:
        value: JSON-строка или None

    Returns:
        Список, если value был валидным JSON-массивом, иначе пустой список

    Raises:
        ValueError: если value не является JSON-массивом
    """
    if value is None:
        return []
    if isinstance(value, str) and value.strip() == "":
        return []

    try:
        parsed = json.loads(value)
    except json.JSONDecodeError as exc:  # pragma: no cover - конкретный текст проверяется в тесте
        raise ValueError(f"Некорректный JSON: {exc.msg}") from exc
    except TypeError as exc:  # pragma: no cover - защитная проверка типов
        raise ValueError(f"Ожидалась JSON-строка, получено: {type(value)}") from exc

    if isinstance(parsed, list):
        return parsed
    raise ValueError(f"Ожидался JSON-массив, получено: {type(parsed)}")


def _to_bool(df: pd.DataFrame, columns: Iterable[str]) -> None:
    """
    Преобразует указанные столбцы DataFrame в булевый тип.

    Преобразование выполняется через промежуточный тип Int64 для корректной
    обработки NaN значений.

    Args:
        df: DataFrame для преобразования (изменяется in-place)
        columns: Итератор с именами столбцов для преобразования
    """
    for col in columns:
        if col in df.columns:
            # Явно приводим к nullable boolean, чтобы избежать future warning'ов при двойных astype.
            df[col] = pd.Series(df[col], copy=False).astype("boolean")


def _load_table(
    conn: sqlite3.Connection, table: str, parse_dates: list[str] | None = None
) -> pd.DataFrame:
    """
    Загружает таблицу из SQLite в pandas DataFrame.

    Args:
        conn: Соединение с SQLite базой данных
        table: Имя таблицы для загрузки
        parse_dates: Список столбцов для парсинга как даты/время

    Returns:
        DataFrame с данными из таблицы
    """
    try:
        return pd.read_sql(f"SELECT * FROM {table}", conn, parse_dates=parse_dates or [])
    except (sqlite3.Error, pd.errors.DatabaseError) as exc:  # pragma: no cover - rethrown with context
        raise ValueError(f"Не удалось прочитать таблицу '{table}': {exc}") from exc


def _ensure_required_columns(table: str, df: pd.DataFrame, required: set[str]) -> None:
    """
    Проверяет наличие обязательных колонок и выбрасывает понятный ValueError.

    Args:
        table: имя таблицы для сообщения об ошибке
        df: DataFrame с данными таблицы
        required: множество обязательных колонок
    """
    missing = required.difference(df.columns)
    if missing:
        missing_sorted = ", ".join(sorted(missing))
        raise ValueError(
            f"В таблице '{table}' отсутствуют обязательные столбцы: {missing_sorted}"
        )


def load_snapshots_as_frame(db_path: Path | str) -> pd.DataFrame:
    """
    Загружает снапшоты из SQLite в pandas DataFrame.

    Функция загружает данные из трёх таблиц (snapshots, processes, app_groups)
    и объединяет их в единый DataFrame на уровне процессов. Булевые столбцы
    преобразуются в dtype boolean, JSON-поля (tags, process_ids) парсятся
    в списки Python.

    Args:
        db_path: Путь к SQLite базе данных со снапшотами

    Returns:
        DataFrame на уровне процессов с джойном глобальных и групповых метрик.
        Столбцы с булевыми значениями приведены к dtype ``boolean``,
        JSON-поля tags/process_ids распарсены в списки.

    Raises:
        FileNotFoundError: если файл базы данных не существует
    """
    path = Path(db_path)
    if not path.exists():
        raise FileNotFoundError(path)

    with sqlite3.connect(path) as conn:
        snapshots = _load_table(conn, "snapshots", parse_dates=["timestamp"])
        processes = _load_table(conn, "processes")
        app_groups = _load_table(conn, "app_groups")

    _ensure_required_columns("snapshots", snapshots, {"snapshot_id"})
    _ensure_required_columns("processes", processes, {"snapshot_id", "pid"})
    _ensure_required_columns("app_groups", app_groups, {"snapshot_id", "app_group_id"})

    if processes.empty:
        return pd.DataFrame()

    # Нормализуем булевые флаги и JSON-списки.
    _to_bool(snapshots, _SNAPSHOT_BOOL_COLS)
    _to_bool(processes, _PROCESS_BOOL_COLS)
    _to_bool(app_groups, _APP_GROUP_BOOL_COLS)

    if "tags" in processes.columns:
        processes["tags"] = processes["tags"].apply(_json_list)
    if "tags" in app_groups.columns:
        app_groups["tags"] = app_groups["tags"].apply(_json_list)
    if "process_ids" in app_groups.columns:
        app_groups["process_ids"] = app_groups["process_ids"].apply(_json_list)

    df = processes.merge(
        snapshots,
        on="snapshot_id",
        how="left",
        suffixes=("_proc", "_snap"),
    )

    if not app_groups.empty:
        df = df.merge(
            app_groups,
            on=["snapshot_id", "app_group_id"],
            how="left",
            suffixes=("", "_group"),
        )

    df = df.sort_values(["snapshot_id", "pid"]).reset_index(drop=True)

    return df
