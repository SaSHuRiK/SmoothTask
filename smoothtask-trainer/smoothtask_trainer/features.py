"""Трансформация и нормализация фич для CatBoostRanker."""

from __future__ import annotations

from typing import Iterable, List, Tuple

import numpy as np
import pandas as pd

_NUMERIC_COLS: list[str] = [
    # Процессные метрики
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
    # Глобальные метрики
    "load_avg_one",
    "load_avg_five",
    "load_avg_fifteen",
    "mem_used_kb",
    "mem_available_kb",
    "mem_total_kb",
    "swap_total_kb",
    "swap_used_kb",
    "time_since_last_input_ms",
    "cpu_user",
    "cpu_system",
    "cpu_idle",
    "cpu_iowait",
    "psi_cpu_some_avg10",
    "psi_cpu_some_avg60",
    "psi_io_some_avg10",
    "psi_mem_some_avg10",
    "psi_mem_full_avg10",
    # Групповые метрики
    "total_cpu_share",
    "total_io_read_bytes",
    "total_io_write_bytes",
    "total_rss_mb",
]

_BOOL_COLS: list[str] = [
    "user_active",
    "bad_responsiveness",
    "has_tty",
    "has_gui_window",
    "is_focused_window",
    "env_has_display",
    "env_has_wayland",
    "env_ssh",
    "is_audio_client",
    "has_active_stream",
    "has_gui_window_group",
    "is_focused_group",
]

_CAT_COLS: list[str] = [
    "process_type",
    "app_name",
    "priority_class",
    "teacher_priority_class",
    "env_term",
    "tags_joined",
]


def _ensure_column(
    df: pd.DataFrame, column: str, default: object, dtype: str | None = None
) -> pd.Series:
    """
    Возвращает столбец из DataFrame или создаёт новый с дефолтным значением.

    Если столбец существует в DataFrame, возвращает его (с приведением типа,
    если указан dtype). Если столбца нет, создаёт новый Series с дефолтным
    значением для всех строк.

    Args:
        df: DataFrame для извлечения столбца
        column: Имя столбца
        default: Дефолтное значение для создания нового столбца
        dtype: Опциональный тип данных для приведения (например, "boolean", "float")

    Returns:
        Series с данными столбца или дефолтными значениями
    """
    if column in df.columns:
        series = df[column]
    else:
        series = pd.Series([default] * len(df), index=df.index)
    if dtype:
        return series.astype(dtype)
    return series


def _prepare_tags_column(series: Iterable[object]) -> pd.Series:
    """
    Преобразует список тегов в строку для категориальной фичи.

    Функция принимает итератор, где каждый элемент может быть списком тегов,
    и преобразует его в строку с тегами, разделёнными символом "|".
    Теги сортируются для консистентности. Если значение отсутствует (NaN),
    возвращается строка "unknown".

    Args:
        series: Итератор со значениями (списки тегов, строки или None)

    Returns:
        Series со строками вида "tag1|tag2|tag3" или "unknown"
    """

    def _join_tags(value: object) -> str:
        if isinstance(value, (list, tuple, set)):
            tags = sorted([str(v) for v in value])
            if not tags:
                return "unknown"
            return "|".join(tags)
        if pd.isna(value):
            return "unknown"
        return str(value)

    return pd.Series([_join_tags(v) for v in series])


def build_feature_matrix(
    df: pd.DataFrame,
) -> Tuple[pd.DataFrame, pd.Series, pd.Series, List[int]]:
    """
    Строит матрицу фич X, таргеты y, group_id и список категориальных фич.

    Функция преобразует DataFrame со снапшотами в формат, пригодный для
    обучения CatBoostRanker. Извлекает числовые, булевые и категориальные
    фичи, выбирает таргет (teacher_score в приоритете, иначе responsiveness_score)
    и группирует данные по snapshot_id.

    Args:
        df: DataFrame со снапшотами (должен содержать столбец snapshot_id)

    Returns:
        Tuple с четырьмя элементами:
        - X: DataFrame с числовыми/булевыми/категориальными фичами
        - y: Series с целевой меткой (teacher_score или responsiveness_score)
        - group_id: Series с идентификатором запроса (snapshot_id)
        - cat_feature_indices: Список индексов категориальных колонок в X

    Raises:
        ValueError: если DataFrame пуст или отсутствует столбец snapshot_id,
                    или если нет доступных таргетов
    """
    if df is None or df.empty:
        raise ValueError("DataFrame с снапшотами пуст")
    if "snapshot_id" not in df.columns:
        raise ValueError("Ожидается столбец snapshot_id для группировки")

    work_df = df.copy()

    # Выбор таргета: teacher_score в приоритете, иначе responsiveness_score.
    teacher = (
        work_df["teacher_score"]
        if "teacher_score" in work_df
        else pd.Series(np.nan, index=work_df.index)
    )
    resp = (
        work_df["responsiveness_score"]
        if "responsiveness_score" in work_df
        else pd.Series(np.nan, index=work_df.index)
    )
    target = teacher.combine_first(resp)

    valid_mask = target.notna()
    if not valid_mask.any():
        raise ValueError(
            "Нет доступных таргетов teacher_score или responsiveness_score"
        )

    work_df = work_df.loc[valid_mask].reset_index(drop=True)
    target = target.loc[valid_mask].reset_index(drop=True)
    group_id = work_df["snapshot_id"].reset_index(drop=True)

    features = {}
    column_order: list[str] = []

    # Числовые фичи
    for col in _NUMERIC_COLS:
        series = pd.to_numeric(_ensure_column(work_df, col, 0.0), errors="coerce")
        features[col] = series.fillna(0.0).astype(float)
        column_order.append(col)

    # Булевые фичи -> 0/1
    for col in _BOOL_COLS:
        series = _ensure_column(work_df, col, False, dtype="boolean")
        features[col] = series.fillna(False).astype(int)
        column_order.append(col)

    # Теги в отдельную категориальную колонку
    if "tags" in work_df:
        work_df["tags_joined"] = _prepare_tags_column(work_df["tags"])
    else:
        work_df["tags_joined"] = pd.Series(
            ["unknown"] * len(work_df), index=work_df.index
        )

    # Категориальные фичи
    cat_feature_indices: list[int] = []
    for col in _CAT_COLS:
        series = _ensure_column(work_df, col, "unknown").astype("string")
        features[col] = series.fillna("unknown")
        cat_feature_indices.append(len(column_order))
        column_order.append(col)

    X = pd.DataFrame(features, columns=column_order)
    return X, target, group_id, cat_feature_indices
