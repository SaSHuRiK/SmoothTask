"""Чтение снапшотов из SQLite и формирование датасета для обучения."""

from __future__ import annotations

import json
import sqlite3
from pathlib import Path
from typing import Iterable

import numpy as np
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

_SNAPSHOT_NUMERIC_COLS = {
    "cpu_user",
    "cpu_system",
    "cpu_idle",
    "cpu_iowait",
    "mem_total_kb",
    "mem_used_kb",
    "mem_available_kb",
    "swap_total_kb",
    "swap_used_kb",
    "load_avg_one",
    "load_avg_five",
    "load_avg_fifteen",
    "psi_cpu_some_avg10",
    "psi_cpu_some_avg60",
    "psi_io_some_avg10",
    "psi_mem_some_avg10",
    "psi_mem_full_avg10",
    "time_since_last_input_ms",
    "sched_latency_p95_ms",
    "sched_latency_p99_ms",
    "audio_xruns_delta",
    "ui_loop_p95_ms",
    "frame_jank_ratio",
    "responsiveness_score",
}

_PROCESS_NUMERIC_COLS = {
    "start_time",
    "uptime_sec",
    "tty_nr",
    "cpu_share_1s",
    "cpu_share_10s",
    "io_read_bytes",
    "io_write_bytes",
    "rss_mb",
    "swap_mb",
    "voluntary_ctx",
    "involuntary_ctx",
    "nice",
    "ionice_class",
    "ionice_prio",
    "teacher_score",
}

_APP_GROUP_NUMERIC_COLS = {
    "root_pid",
    "total_cpu_share",
    "total_io_read_bytes",
    "total_io_write_bytes",
    "total_rss_mb",
}


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


def _parse_process_ids(value: str | None) -> list[int]:
    """
    Приводит process_ids к списку целых чисел с валидацией.

    Допускаются числа или строковые представления чисел. Пустые строки и NaN
    игнорируются. При других значениях выбрасывается ValueError с примерами.
    """
    parsed = _json_list(value)
    if not parsed:
        return []

    process_ids: list[int] = []
    invalid_values: list[object] = []

    for item in parsed:
        if item is None or (isinstance(item, float) and np.isnan(item)):
            continue
        if isinstance(item, (list, dict, set, tuple, bool)):
            invalid_values.append(item)
            continue
        if isinstance(item, (int, np.integer)):
            process_ids.append(int(item))
            continue
        if isinstance(item, (float, np.floating)):
            if np.isfinite(item) and float(item).is_integer():
                process_ids.append(int(item))
            else:
                invalid_values.append(item)
            continue
        if isinstance(item, str):
            stripped = item.strip()
            if stripped == "":
                continue
            try:
                number = float(stripped)
            except ValueError:
                invalid_values.append(item)
                continue
            if not number.is_integer():
                invalid_values.append(item)
                continue
            process_ids.append(int(number))
            continue

        invalid_values.append(item)

    if invalid_values:
        sample_values = ", ".join(repr(v) for v in invalid_values[:5])
        raise ValueError(
            f"Колонка 'process_ids' содержит нецелые значения: {sample_values}"
        )

    return process_ids


def _normalize_tags_list(value: str | None, column: str) -> list[str]:
    """
    Приводит tags к списку строк, убирая пустые элементы и валидируя типы.

    Допускаются скалярные значения (строки, числа, bool). Пустые строки и NaN
    отбрасываются. Сложные структуры (list/dict/set/tuple) приводят к
    ValueError.
    """
    parsed = _json_list(value)
    if not parsed:
        return []

    normalized: list[str] = []
    invalid_values: list[object] = []

    for item in parsed:
        if item is None or (isinstance(item, float) and np.isnan(item)):
            continue
        if isinstance(item, (list, dict, set, tuple)):
            invalid_values.append(item)
            continue
        text = str(item).strip()
        if text == "":
            continue
        normalized.append(text)

    if invalid_values:
        sample_values = ", ".join(repr(v) for v in invalid_values[:5])
        raise ValueError(
            f"Колонка '{column}' содержит некорректные элементы tags: {sample_values}"
        )

    return normalized


def _coerce_bool_column(
    series: pd.Series, column: str, table: str
) -> pd.Series:
    """
    Приводит столбец к nullable boolean с валидацией допустимых значений.

    Допускаются True/False, 0/1, строковые "0"/"1" и NaN. При других значениях
    выбрасывается ValueError с указанием таблицы и примеров значений.
    """
    coerced: list[object] = []
    invalid_values: list[object] = []

    for value in series:
        if pd.isna(value):
            coerced.append(pd.NA)
            continue
        if isinstance(value, (bool, np.bool_)):
            coerced.append(bool(value))
            continue
        if isinstance(value, (int, np.integer)):
            if value in (0, 1):
                coerced.append(bool(value))
                continue
        if isinstance(value, (float, np.floating)):
            if value in (0.0, 1.0):
                coerced.append(bool(int(value)))
                continue
        if isinstance(value, str):
            stripped = value.strip().lower()
            if stripped in {"0", "1"}:
                coerced.append(stripped == "1")
                continue
        invalid_values.append(value)
        coerced.append(pd.NA)

    if invalid_values:
        sample_values = ", ".join(repr(v) for v in invalid_values[:5])
        raise ValueError(
            f"Колонка '{column}' в таблице '{table}' содержит невалидные булевые значения: {sample_values}"
        )

    return pd.Series(coerced, index=series.index, dtype="boolean")


def _to_bool(df: pd.DataFrame, columns: Iterable[str], table: str) -> None:
    """
    Преобразует указанные столбцы DataFrame в булевый тип.

    Преобразование выполняется через промежуточный тип Int64 для корректной
    обработки NaN значений.

    Args:
        df: DataFrame для преобразования (изменяется in-place)
        columns: Итератор с именами столбцов для преобразования
        table: Имя таблицы для сообщений об ошибке
    """
    for col in columns:
        if col in df.columns:
            df[col] = _coerce_bool_column(
                pd.Series(df[col], copy=False), column=col, table=table
            )


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


def _ensure_unique_keys(
    df: pd.DataFrame, table: str, keys: list[str], sample_size: int = 5
) -> None:
    """
    Проверяет отсутствие дубликатов по указанным ключевым столбцам.

    Args:
        df: DataFrame с данными
        table: Имя таблицы для сообщения об ошибке
        keys: Список столбцов, образующих ключ
        sample_size: Сколько ключей показать в сообщении об ошибке
    """
    duplicates = df[df.duplicated(subset=keys, keep=False)]
    if duplicates.empty:
        return

    key_samples = duplicates[keys].drop_duplicates().head(sample_size)
    formatted = "; ".join(
        "(" + ", ".join(str(row[key]) for key in keys) + ")" for _, row in key_samples.iterrows()
    )
    raise ValueError(
        f"В таблице '{table}' обнаружены дубликаты по ключу {keys}: {formatted}"
    )


def _ensure_no_nan(
    df: pd.DataFrame, table: str, columns: Iterable[str], sample_size: int = 5
) -> None:
    """
    Проверяет отсутствие NaN/NA в обязательных колонках и выбрасывает понятный ValueError.
    """
    for col in columns:
        nan_mask = df[col].isna()
        if nan_mask.any():
            sample_indices = ", ".join(str(idx) for idx in nan_mask[nan_mask].index[:sample_size])
            raise ValueError(
                f"В таблице '{table}' колонка '{col}' содержит пустые значения (строки: {sample_indices})"
            )


def _ensure_no_infinite(
    df: pd.DataFrame, table: str, columns: Iterable[str], sample_size: int = 5
) -> None:
    """
    Проверяет отсутствие бесконечных значений в указанных колонках.
    """
    for col in columns:
        if col not in df.columns:
            continue
        numeric = pd.to_numeric(df[col], errors="coerce")
        inf_mask = np.isinf(numeric)
        if inf_mask.any():
            sample_indices = ", ".join(str(idx) for idx in numeric[inf_mask].index[:sample_size])
            raise ValueError(
                f"В таблице '{table}' колонка '{col}' содержит бесконечные значения (строки: {sample_indices})"
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
    _ensure_no_nan(snapshots, table="snapshots", columns={"snapshot_id"})
    _ensure_no_nan(processes, table="processes", columns={"snapshot_id", "pid"})
    _ensure_no_nan(app_groups, table="app_groups", columns={"snapshot_id", "app_group_id"})
    _ensure_no_infinite(
        snapshots,
        table="snapshots",
        columns=_SNAPSHOT_NUMERIC_COLS,
    )
    _ensure_no_infinite(
        processes,
        table="processes",
        columns=_PROCESS_NUMERIC_COLS,
    )
    _ensure_no_infinite(
        app_groups,
        table="app_groups",
        columns=_APP_GROUP_NUMERIC_COLS,
    )

    # Проверяем ссылочную целостность snapshot_id в processes.
    snapshot_ids = set(snapshots["snapshot_id"].dropna().unique())
    process_snapshot_ids = set(processes["snapshot_id"].dropna().unique())
    missing_snapshots = sorted(process_snapshot_ids.difference(snapshot_ids))
    if missing_snapshots:
        missing_preview = ", ".join(str(sid) for sid in missing_snapshots[:5])
        raise ValueError(
            f"В таблице 'processes' найдены snapshot_id без записей в 'snapshots': {missing_preview}"
        )

    _ensure_unique_keys(processes, table="processes", keys=["snapshot_id", "pid"])
    _ensure_unique_keys(app_groups, table="app_groups", keys=["snapshot_id", "app_group_id"])

    app_group_snapshot_ids = set(app_groups["snapshot_id"].dropna().unique())
    missing_group_snapshots = sorted(app_group_snapshot_ids.difference(snapshot_ids))
    if missing_group_snapshots:
        missing_preview = ", ".join(str(sid) for sid in missing_group_snapshots[:5])
        raise ValueError(
            f"В таблице 'app_groups' найдены snapshot_id без записей в 'snapshots': {missing_preview}"
        )

    if "app_group_id" in processes.columns:
        process_groups = processes[["snapshot_id", "app_group_id"]].dropna(
            subset=["app_group_id"]
        )
        if not process_groups.empty:
            app_group_keys = {
                (row.snapshot_id, row.app_group_id)
                for row in app_groups.dropna(subset=["snapshot_id", "app_group_id"]).itertuples(index=False)
            }
            process_group_keys = {
                (row.snapshot_id, row.app_group_id) for row in process_groups.itertuples(index=False)
            }
            missing_pairs = sorted(process_group_keys.difference(app_group_keys))
            if missing_pairs:
                formatted = "; ".join(
                    f"(snapshot_id={sid}, app_group_id={gid})" for sid, gid in missing_pairs[:5]
                )
                raise ValueError(
                    "В таблице 'processes' есть app_group_id без записей в 'app_groups': "
                    f"{formatted}"
                )

    if processes.empty:
        return pd.DataFrame()

    # Нормализуем булевые флаги и JSON-списки.
    _to_bool(snapshots, _SNAPSHOT_BOOL_COLS, table="snapshots")
    _to_bool(processes, _PROCESS_BOOL_COLS, table="processes")
    _to_bool(app_groups, _APP_GROUP_BOOL_COLS, table="app_groups")

    if "tags" in processes.columns:
        processes["tags"] = processes["tags"].apply(
            lambda value: _normalize_tags_list(value, column="tags")
        )
    if "tags" in app_groups.columns:
        app_groups["tags"] = app_groups["tags"].apply(
            lambda value: _normalize_tags_list(value, column="tags")
        )
    if "process_ids" in app_groups.columns:
        app_groups["process_ids"] = app_groups["process_ids"].apply(_parse_process_ids)

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
