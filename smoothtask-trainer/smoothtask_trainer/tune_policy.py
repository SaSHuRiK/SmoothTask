"""Оффлайн-тюнинг параметров политики по логам и метрикам латентности."""

import sqlite3
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Dict, Optional

import pandas as pd
import yaml


def _validate_db_path(db_path: Path) -> None:
    """
    Проверяет существование файла базы данных.

    Args:
        db_path: Путь к SQLite базе данных

    Raises:
        FileNotFoundError: если файл не существует
    """
    if not db_path.exists():
        raise FileNotFoundError(f"База данных не найдена: {db_path}")


def _validate_db_schema(conn: sqlite3.Connection) -> None:
    """
    Проверяет наличие необходимых таблиц в базе данных.

    Args:
        conn: Соединение с SQLite базой данных

    Raises:
        ValueError: если отсутствуют необходимые таблицы
    """
    cursor = conn.cursor()
    cursor.execute(
        "SELECT name FROM sqlite_master WHERE type='table' AND name IN ('snapshots', 'processes', 'app_groups')"
    )
    tables = {row[0] for row in cursor.fetchall()}

    required_tables = {"snapshots", "processes", "app_groups"}
    missing_tables = required_tables - tables

    if missing_tables:
        raise ValueError(
            f"База данных не содержит необходимые таблицы: {', '.join(missing_tables)}"
        )


def _count_snapshots(conn: sqlite3.Connection, days_back: int = 7) -> int:
    """
    Подсчитывает количество снапшотов за указанный период.

    Args:
        conn: Соединение с SQLite базой данных
        days_back: Количество дней назад для фильтрации (по умолчанию 7)

    Returns:
        Количество снапшотов за указанный период
    """
    cursor = conn.cursor()

    if days_back > 0:
        cutoff_time = datetime.now(timezone.utc) - timedelta(days=days_back)
        cutoff_timestamp = cutoff_time.isoformat()
        cursor.execute(
            "SELECT COUNT(*) FROM snapshots WHERE timestamp >= ?",
            (cutoff_timestamp,),
        )
    else:
        cursor.execute("SELECT COUNT(*) FROM snapshots")

    result = cursor.fetchone()
    return result[0] if result else 0


def load_snapshots_for_tuning(
    db_path: Path, min_snapshots: int = 100, days_back: int = 7
) -> pd.DataFrame:
    """
    Загружает снапшоты из БД для тюнинга политики с фильтрацией по времени.

    Функция загружает снапшоты за указанный период и проверяет минимальное
    количество данных для надёжной оптимизации.

    Args:
        db_path: Путь к SQLite базе данных со снапшотами
        min_snapshots: Минимальное количество снапшотов для тюнинга (по умолчанию 100)
        days_back: Количество дней назад для фильтрации (по умолчанию 7)

    Returns:
        DataFrame со снапшотами за указанный период

    Raises:
        FileNotFoundError: если файл базы данных не существует
        ValueError: если данных недостаточно для тюнинга
        sqlite3.OperationalError: если БД имеет некорректный формат
    """
    _validate_db_path(db_path)

    with sqlite3.connect(db_path) as conn:
        _validate_db_schema(conn)

        snapshot_count = _count_snapshots(conn, days_back)

        if snapshot_count < min_snapshots:
            raise ValueError(
                f"Недостаточно данных для тюнинга: найдено {snapshot_count} снапшотов, "
                f"требуется минимум {min_snapshots} за последние {days_back} дней"
            )

        # Загружаем снапшоты за указанный период
        if days_back > 0:
            cutoff_time = datetime.now(timezone.utc) - timedelta(days=days_back)
            cutoff_timestamp = cutoff_time.isoformat()
            query = "SELECT * FROM snapshots WHERE timestamp >= ? ORDER BY timestamp"
            df = pd.read_sql(
                query, conn, params=(cutoff_timestamp,), parse_dates=["timestamp"]
            )
        else:
            query = "SELECT * FROM snapshots ORDER BY timestamp"
            df = pd.read_sql(query, conn, parse_dates=["timestamp"])

    return df


def compute_policy_correlations(snapshots_df: pd.DataFrame) -> Dict[str, float]:
    """
    Вычисляет корреляции между параметрами политики и метриками отзывчивости.

    Функция анализирует корреляции между:
    - PSI-метриками (psi_cpu_some_avg10, psi_io_some_avg10) и bad_responsiveness
    - Latency-метриками (sched_latency_p99_ms, ui_loop_p95_ms) и bad_responsiveness
    - PSI-метриками и responsiveness_score
    - Latency-метриками и responsiveness_score

    Корреляции вычисляются с использованием метода Пирсона. Значения корреляции
    находятся в диапазоне [-1.0, 1.0], где:
    - Положительные значения указывают на прямую зависимость
    - Отрицательные значения указывают на обратную зависимость
    - Значения близкие к 0 указывают на отсутствие линейной зависимости

    Args:
        snapshots_df: DataFrame со снапшотами из БД (должен содержать колонки:
                     psi_cpu_some_avg10, psi_io_some_avg10, sched_latency_p99_ms,
                     ui_loop_p95_ms, bad_responsiveness, responsiveness_score)

    Returns:
        Словарь с корреляциями, где ключи:
        - 'psi_cpu_vs_bad_responsiveness': корреляция между psi_cpu_some_avg10 и bad_responsiveness
        - 'psi_io_vs_bad_responsiveness': корреляция между psi_io_some_avg10 и bad_responsiveness
        - 'sched_latency_vs_bad_responsiveness': корреляция между sched_latency_p99_ms и bad_responsiveness
        - 'ui_latency_vs_bad_responsiveness': корреляция между ui_loop_p95_ms и bad_responsiveness
        - 'psi_cpu_vs_responsiveness_score': корреляция между psi_cpu_some_avg10 и responsiveness_score
        - 'psi_io_vs_responsiveness_score': корреляция между psi_io_some_avg10 и responsiveness_score
        - 'sched_latency_vs_responsiveness_score': корреляция между sched_latency_p99_ms и responsiveness_score
        - 'ui_latency_vs_responsiveness_score': корреляция между ui_loop_p95_ms и responsiveness_score

        Значения могут быть NaN, если для соответствующей пары метрик недостаточно данных.

    Examples:
        >>> import pandas as pd
        >>> from smoothtask_trainer.tune_policy import compute_policy_correlations
        >>>
        >>> # Создаём тестовый DataFrame
        >>> df = pd.DataFrame({
        ...     'psi_cpu_some_avg10': [0.1, 0.2, 0.3, 0.4, 0.5],
        ...     'psi_io_some_avg10': [0.05, 0.1, 0.15, 0.2, 0.25],
        ...     'sched_latency_p99_ms': [5.0, 10.0, 15.0, 20.0, 25.0],
        ...     'ui_loop_p95_ms': [10.0, 15.0, 20.0, 25.0, 30.0],
        ...     'bad_responsiveness': [0, 0, 1, 1, 1],
        ...     'responsiveness_score': [1.0, 0.9, 0.7, 0.5, 0.3]
        ... })
        >>>
        >>> correlations = compute_policy_correlations(df)
        >>> print(f"PSI CPU vs bad_responsiveness: {correlations['psi_cpu_vs_bad_responsiveness']:.3f}")
        >>> print(f"Sched latency vs responsiveness_score: {correlations['sched_latency_vs_responsiveness_score']:.3f}")
    """
    if snapshots_df.empty:
        return {
            "psi_cpu_vs_bad_responsiveness": float("nan"),
            "psi_io_vs_bad_responsiveness": float("nan"),
            "sched_latency_vs_bad_responsiveness": float("nan"),
            "ui_latency_vs_bad_responsiveness": float("nan"),
            "psi_cpu_vs_responsiveness_score": float("nan"),
            "psi_io_vs_responsiveness_score": float("nan"),
            "sched_latency_vs_responsiveness_score": float("nan"),
            "ui_latency_vs_responsiveness_score": float("nan"),
        }

    # Преобразуем bad_responsiveness в числовой тип, если это необходимо
    if "bad_responsiveness" in snapshots_df.columns:
        if (
            snapshots_df["bad_responsiveness"].dtype == "object"
            or snapshots_df["bad_responsiveness"].dtype == "bool"
        ):
            snapshots_df = snapshots_df.copy()
            snapshots_df["bad_responsiveness"] = snapshots_df[
                "bad_responsiveness"
            ].astype(int)

    correlations = {}

    # Корреляции PSI-метрик с bad_responsiveness
    if (
        "psi_cpu_some_avg10" in snapshots_df.columns
        and "bad_responsiveness" in snapshots_df.columns
    ):
        psi_cpu_bad = snapshots_df[
            ["psi_cpu_some_avg10", "bad_responsiveness"]
        ].dropna()
        if len(psi_cpu_bad) > 1:
            corr = psi_cpu_bad["psi_cpu_some_avg10"].corr(
                psi_cpu_bad["bad_responsiveness"]
            )
            correlations["psi_cpu_vs_bad_responsiveness"] = (
                corr if not pd.isna(corr) else float("nan")
            )
        else:
            correlations["psi_cpu_vs_bad_responsiveness"] = float("nan")
    else:
        correlations["psi_cpu_vs_bad_responsiveness"] = float("nan")

    if (
        "psi_io_some_avg10" in snapshots_df.columns
        and "bad_responsiveness" in snapshots_df.columns
    ):
        psi_io_bad = snapshots_df[["psi_io_some_avg10", "bad_responsiveness"]].dropna()
        if len(psi_io_bad) > 1:
            corr = psi_io_bad["psi_io_some_avg10"].corr(
                psi_io_bad["bad_responsiveness"]
            )
            correlations["psi_io_vs_bad_responsiveness"] = (
                corr if not pd.isna(corr) else float("nan")
            )
        else:
            correlations["psi_io_vs_bad_responsiveness"] = float("nan")
    else:
        correlations["psi_io_vs_bad_responsiveness"] = float("nan")

    # Корреляции latency-метрик с bad_responsiveness
    if (
        "sched_latency_p99_ms" in snapshots_df.columns
        and "bad_responsiveness" in snapshots_df.columns
    ):
        sched_bad = snapshots_df[
            ["sched_latency_p99_ms", "bad_responsiveness"]
        ].dropna()
        if len(sched_bad) > 1:
            corr = sched_bad["sched_latency_p99_ms"].corr(
                sched_bad["bad_responsiveness"]
            )
            correlations["sched_latency_vs_bad_responsiveness"] = (
                corr if not pd.isna(corr) else float("nan")
            )
        else:
            correlations["sched_latency_vs_bad_responsiveness"] = float("nan")
    else:
        correlations["sched_latency_vs_bad_responsiveness"] = float("nan")

    if (
        "ui_loop_p95_ms" in snapshots_df.columns
        and "bad_responsiveness" in snapshots_df.columns
    ):
        ui_bad = snapshots_df[["ui_loop_p95_ms", "bad_responsiveness"]].dropna()
        if len(ui_bad) > 1:
            corr = ui_bad["ui_loop_p95_ms"].corr(ui_bad["bad_responsiveness"])
            correlations["ui_latency_vs_bad_responsiveness"] = (
                corr if not pd.isna(corr) else float("nan")
            )
        else:
            correlations["ui_latency_vs_bad_responsiveness"] = float("nan")
    else:
        correlations["ui_latency_vs_bad_responsiveness"] = float("nan")

    # Корреляции PSI-метрик с responsiveness_score
    if (
        "psi_cpu_some_avg10" in snapshots_df.columns
        and "responsiveness_score" in snapshots_df.columns
    ):
        psi_cpu_score = snapshots_df[
            ["psi_cpu_some_avg10", "responsiveness_score"]
        ].dropna()
        if len(psi_cpu_score) > 1:
            corr = psi_cpu_score["psi_cpu_some_avg10"].corr(
                psi_cpu_score["responsiveness_score"]
            )
            correlations["psi_cpu_vs_responsiveness_score"] = (
                corr if not pd.isna(corr) else float("nan")
            )
        else:
            correlations["psi_cpu_vs_responsiveness_score"] = float("nan")
    else:
        correlations["psi_cpu_vs_responsiveness_score"] = float("nan")

    if (
        "psi_io_some_avg10" in snapshots_df.columns
        and "responsiveness_score" in snapshots_df.columns
    ):
        psi_io_score = snapshots_df[
            ["psi_io_some_avg10", "responsiveness_score"]
        ].dropna()
        if len(psi_io_score) > 1:
            corr = psi_io_score["psi_io_some_avg10"].corr(
                psi_io_score["responsiveness_score"]
            )
            correlations["psi_io_vs_responsiveness_score"] = (
                corr if not pd.isna(corr) else float("nan")
            )
        else:
            correlations["psi_io_vs_responsiveness_score"] = float("nan")
    else:
        correlations["psi_io_vs_responsiveness_score"] = float("nan")

    # Корреляции latency-метрик с responsiveness_score
    if (
        "sched_latency_p99_ms" in snapshots_df.columns
        and "responsiveness_score" in snapshots_df.columns
    ):
        sched_score = snapshots_df[
            ["sched_latency_p99_ms", "responsiveness_score"]
        ].dropna()
        if len(sched_score) > 1:
            corr = sched_score["sched_latency_p99_ms"].corr(
                sched_score["responsiveness_score"]
            )
            correlations["sched_latency_vs_responsiveness_score"] = (
                corr if not pd.isna(corr) else float("nan")
            )
        else:
            correlations["sched_latency_vs_responsiveness_score"] = float("nan")
    else:
        correlations["sched_latency_vs_responsiveness_score"] = float("nan")

    if (
        "ui_loop_p95_ms" in snapshots_df.columns
        and "responsiveness_score" in snapshots_df.columns
    ):
        ui_score = snapshots_df[["ui_loop_p95_ms", "responsiveness_score"]].dropna()
        if len(ui_score) > 1:
            corr = ui_score["ui_loop_p95_ms"].corr(ui_score["responsiveness_score"])
            correlations["ui_latency_vs_responsiveness_score"] = (
                corr if not pd.isna(corr) else float("nan")
            )
        else:
            correlations["ui_latency_vs_responsiveness_score"] = float("nan")
    else:
        correlations["ui_latency_vs_responsiveness_score"] = float("nan")

    return correlations


def optimize_psi_thresholds(
    snapshots_df: pd.DataFrame, percentile: float = 0.95
) -> Dict[str, float]:
    """
    Оптимизирует пороги PSI на основе перцентилей PSI значений в моменты bad_responsiveness.

    Функция анализирует PSI значения (psi_cpu_some_avg10, psi_io_some_avg10) в моменты,
    когда система была неотзывчивой (bad_responsiveness = true), и вычисляет перцентили
    этих значений. Оптимальные пороги устанавливаются на уровне указанного перцентиля,
    чтобы система могла предсказать неотзывчивость до того, как она станет критической.

    Args:
        snapshots_df: DataFrame со снапшотами из БД (должен содержать колонки:
                     psi_cpu_some_avg10, psi_io_some_avg10, bad_responsiveness)
        percentile: Перцентиль для вычисления порогов (по умолчанию 0.95, т.е. P95)

    Returns:
        Словарь с оптимальными порогами:
        - 'psi_cpu_some_high': оптимальный порог PSI CPU (float в диапазоне [0.0, 1.0])
        - 'psi_io_some_high': оптимальный порог PSI IO (float в диапазоне [0.0, 1.0])

        Если данных недостаточно или нет моментов bad_responsiveness, возвращаются
        значения по умолчанию (0.6 для CPU, 0.4 для IO).

    Examples:
        >>> import pandas as pd
        >>> from smoothtask_trainer.tune_policy import optimize_psi_thresholds
        >>>
        >>> # Создаём тестовый DataFrame с моментами bad_responsiveness
        >>> df = pd.DataFrame({
        ...     'psi_cpu_some_avg10': [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
        ...     'psi_io_some_avg10': [0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4],
        ...     'bad_responsiveness': [0, 0, 0, 0, 1, 1, 1, 1],
        ... })
        >>>
        >>> thresholds = optimize_psi_thresholds(df, percentile=0.95)
        >>> print(f"PSI CPU threshold: {thresholds['psi_cpu_some_high']:.3f}")
        >>> print(f"PSI IO threshold: {thresholds['psi_io_some_high']:.3f}")
    """
    if snapshots_df.empty:
        return {
            "psi_cpu_some_high": 0.6,
            "psi_io_some_high": 0.4,
        }

    # Преобразуем bad_responsiveness в числовой тип, если это необходимо
    if "bad_responsiveness" in snapshots_df.columns:
        if (
            snapshots_df["bad_responsiveness"].dtype == "object"
            or snapshots_df["bad_responsiveness"].dtype == "bool"
        ):
            snapshots_df = snapshots_df.copy()
            snapshots_df["bad_responsiveness"] = snapshots_df[
                "bad_responsiveness"
            ].astype(int)

    # Фильтруем снапшоты с bad_responsiveness = true
    bad_snapshots = snapshots_df[snapshots_df["bad_responsiveness"] == 1]

    # Если нет моментов bad_responsiveness, возвращаем значения по умолчанию
    if bad_snapshots.empty:
        return {
            "psi_cpu_some_high": 0.6,
            "psi_io_some_high": 0.4,
        }

    # Вычисляем перцентили PSI значений в плохих условиях
    psi_cpu_threshold = 0.6  # значение по умолчанию
    psi_io_threshold = 0.4  # значение по умолчанию

    if "psi_cpu_some_avg10" in bad_snapshots.columns:
        psi_cpu_values = bad_snapshots["psi_cpu_some_avg10"].dropna()
        if len(psi_cpu_values) > 0:
            psi_cpu_threshold = float(psi_cpu_values.quantile(percentile))
            # Ограничиваем диапазон [0.0, 1.0]
            psi_cpu_threshold = max(0.0, min(1.0, psi_cpu_threshold))

    if "psi_io_some_avg10" in bad_snapshots.columns:
        psi_io_values = bad_snapshots["psi_io_some_avg10"].dropna()
        if len(psi_io_values) > 0:
            psi_io_threshold = float(psi_io_values.quantile(percentile))
            # Ограничиваем диапазон [0.0, 1.0]
            psi_io_threshold = max(0.0, min(1.0, psi_io_threshold))

    return {
        "psi_cpu_some_high": psi_cpu_threshold,
        "psi_io_some_high": psi_io_threshold,
    }


def optimize_latency_thresholds(
    snapshots_df: pd.DataFrame, percentile: float = 0.95, multiplier: float = 1.5
) -> Dict[str, float]:
    """
    Оптимизирует пороги latency на основе перцентилей реальных значений в хороших условиях.

    Функция анализирует реальные значения latency (sched_latency_p99_ms, ui_loop_p95_ms)
    в моменты, когда система была отзывчивой (bad_responsiveness = false), и вычисляет
    перцентили этих значений. Оптимальные пороги устанавливаются на уровне указанного
    перцентиля, умноженного на multiplier, чтобы обеспечить запас для предсказания
    неотзывчивости до того, как она станет критической.

    Args:
        snapshots_df: DataFrame со снапшотами из БД (должен содержать колонки:
                     sched_latency_p99_ms, ui_loop_p95_ms, bad_responsiveness)
        percentile: Перцентиль для вычисления порогов (по умолчанию 0.95, т.е. P95)
        multiplier: Множитель для создания запаса над перцентилем (по умолчанию 1.5)

    Returns:
        Словарь с оптимальными порогами:
        - 'sched_latency_p99_threshold_ms': оптимальный порог P99 scheduling latency (float в мс)
        - 'ui_loop_p95_threshold_ms': оптимальный порог P95 UI loop latency (float в мс)

        Если данных недостаточно или нет моментов с хорошими условиями, возвращаются
        значения по умолчанию (20.0 мс для sched_latency_p99, 16.67 мс для ui_loop_p95).

    Examples:
        >>> import pandas as pd
        >>> from smoothtask_trainer.tune_policy import optimize_latency_thresholds
        >>>
        >>> # Создаём тестовый DataFrame с хорошими условиями
        >>> df = pd.DataFrame({
        ...     'sched_latency_p99_ms': [5.0, 10.0, 15.0, 20.0, 25.0],
        ...     'ui_loop_p95_ms': [10.0, 12.0, 14.0, 16.0, 18.0],
        ...     'bad_responsiveness': [0, 0, 0, 0, 0],
        ... })
        >>>
        >>> thresholds = optimize_latency_thresholds(df, percentile=0.95, multiplier=1.5)
        >>> print(f"Sched latency threshold: {thresholds['sched_latency_p99_threshold_ms']:.3f}")
        >>> print(f"UI loop threshold: {thresholds['ui_loop_p95_threshold_ms']:.3f}")
    """
    if snapshots_df.empty:
        return {
            "sched_latency_p99_threshold_ms": 20.0,
            "ui_loop_p95_threshold_ms": 16.67,
        }

    # Преобразуем bad_responsiveness в числовой тип, если это необходимо
    if "bad_responsiveness" in snapshots_df.columns:
        if (
            snapshots_df["bad_responsiveness"].dtype == "object"
            or snapshots_df["bad_responsiveness"].dtype == "bool"
        ):
            snapshots_df = snapshots_df.copy()
            snapshots_df["bad_responsiveness"] = snapshots_df[
                "bad_responsiveness"
            ].astype(int)

    # Фильтруем снапшоты с хорошими условиями (bad_responsiveness = false)
    good_snapshots = snapshots_df[snapshots_df["bad_responsiveness"] == 0]

    # Если нет моментов с хорошими условиями, возвращаем значения по умолчанию
    if good_snapshots.empty:
        return {
            "sched_latency_p99_threshold_ms": 20.0,
            "ui_loop_p95_threshold_ms": 16.67,
        }

    # Вычисляем перцентили latency значений в хороших условиях
    sched_latency_threshold = 20.0  # значение по умолчанию
    ui_loop_threshold = 16.67  # значение по умолчанию

    if "sched_latency_p99_ms" in good_snapshots.columns:
        sched_latency_values = good_snapshots["sched_latency_p99_ms"].dropna()
        if len(sched_latency_values) > 0:
            percentile_value = float(sched_latency_values.quantile(percentile))
            sched_latency_threshold = percentile_value * multiplier
            # Ограничиваем диапазон [1.0, 1000.0] мс
            sched_latency_threshold = max(1.0, min(1000.0, sched_latency_threshold))

    if "ui_loop_p95_ms" in good_snapshots.columns:
        ui_loop_values = good_snapshots["ui_loop_p95_ms"].dropna()
        if len(ui_loop_values) > 0:
            percentile_value = float(ui_loop_values.quantile(percentile))
            ui_loop_threshold = percentile_value * multiplier
            # Ограничиваем диапазон [1.0, 1000.0] мс
            ui_loop_threshold = max(1.0, min(1000.0, ui_loop_threshold))

    # Логическая валидация: P99 должен быть >= P95
    if sched_latency_threshold < ui_loop_threshold:
        sched_latency_threshold = ui_loop_threshold

    return {
        "sched_latency_p99_threshold_ms": sched_latency_threshold,
        "ui_loop_p95_threshold_ms": ui_loop_threshold,
    }


def save_optimized_config(
    config_dict: Dict, config_out: Path, config_in: Optional[Path] = None
) -> None:
    """
    Сохраняет оптимизированный конфиг в YAML файл.

    Функция сохраняет оптимизированные параметры в YAML файл, сохраняя остальные
    параметры из исходного конфига (если он существует). Если исходный конфиг
    не указан, создаётся новый конфиг только с оптимизированными параметрами.

    Args:
        config_dict: Словарь с оптимизированными параметрами (например,
                    {'psi_cpu_some_high': 0.7, 'sched_latency_p99_threshold_ms': 30.0})
        config_out: Путь к выходному YAML файлу (будет перезаписан, если существует)
        config_in: Опциональный путь к исходному YAML файлу для сохранения остальных параметров

    Raises:
        FileNotFoundError: если config_in указан, но файл не существует
        IOError: если не удалось записать config_out
        PermissionError: если нет прав на запись в config_out
        yaml.YAMLError: если исходный конфиг имеет некорректный формат YAML

    Examples:
        >>> from pathlib import Path
        >>> from smoothtask_trainer.tune_policy import save_optimized_config
        >>>
        >>> # Сохраняем оптимизированные параметры без исходного конфига
        >>> optimized = {
        ...     'thresholds': {
        ...         'psi_cpu_some_high': 0.7,
        ...         'psi_io_some_high': 0.5,
        ...         'sched_latency_p99_threshold_ms': 30.0,
        ...         'ui_loop_p95_threshold_ms': 20.0,
        ...     }
        ... }
        >>> save_optimized_config(optimized, Path('/tmp/optimized_config.yml'))
        >>>
        >>> # Сохраняем оптимизированные параметры с сохранением остальных из исходного конфига
        >>> save_optimized_config(
        ...     optimized,
        ...     Path('/tmp/optimized_config.yml'),
        ...     config_in=Path('/etc/smoothtask/config.yml')
        ... )
    """
    # Загружаем исходный конфиг, если он указан
    base_config = {}
    if config_in is not None:
        if not config_in.exists():
            raise FileNotFoundError(f"Исходный конфиг не найден: {config_in}")

        with open(config_in, "r") as f:
            base_config = yaml.safe_load(f) or {}

    # Объединяем исходный конфиг с оптимизированными параметрами
    # Оптимизированные параметры имеют приоритет над исходными
    merged_config = base_config.copy()

    # Рекурсивно обновляем вложенные словари
    def deep_update(base: Dict, updates: Dict) -> Dict:
        """Рекурсивно обновляет вложенные словари."""
        result = base.copy()
        for key, value in updates.items():
            if (
                key in result
                and isinstance(result[key], dict)
                and isinstance(value, dict)
            ):
                result[key] = deep_update(result[key], value)
            else:
                result[key] = value
        return result

    merged_config = deep_update(merged_config, config_dict)

    # Сохраняем объединённый конфиг в YAML файл
    try:
        with open(config_out, "w") as f:
            yaml.dump(merged_config, f, default_flow_style=False, sort_keys=False)
    except IOError as e:
        raise IOError(f"Не удалось записать конфиг в {config_out}: {e}") from e
    except PermissionError as e:
        raise PermissionError(f"Нет прав на запись в {config_out}: {e}") from e


def tune_policy(
    db_path: Path, config_out: Path, config_in: Optional[Path] = None
) -> None:
    """
    Подбирает оптимальные параметры политики (пороги PSI, latency и т.п.)
    на основе собранных снапшотов и метрик отзывчивости.

    Функция анализирует исторические данные из базы снапшотов и подбирает оптимальные
    значения параметров политики для улучшения отзывчивости системы. Оптимизация
    выполняется на основе анализа PSI и latency метрик в различных условиях
    отзывчивости системы.

    Args:
        db_path: Путь к SQLite базе данных со снапшотами (должна содержать таблицы
                `snapshots`, `processes`, `app_groups` с метриками отзывчивости)
        config_out: Путь к выходному YAML файлу с оптимизированными параметрами
                   (будет перезаписан, если существует)
        config_in: Опциональный путь к исходному YAML файлу для сохранения остальных
                   параметров (если не указан, создаётся новый конфиг только с
                   оптимизированными параметрами)

    Returns:
        None. Результат сохраняется в `config_out`.

    Raises:
        FileNotFoundError: Если `db_path` не существует или `config_in` указан, но не существует
        sqlite3.OperationalError: Если БД имеет некорректный формат или недоступна
        ValueError: Если данных недостаточно для оптимизации
        IOError: Если не удалось записать `config_out`
        PermissionError: Если нет прав на запись в `config_out`

    Examples:
        >>> from pathlib import Path
        >>> from smoothtask_trainer.tune_policy import tune_policy
        >>>
        >>> # Базовое использование
        >>> db_path = Path("/var/lib/smoothtask/snapshots.sqlite")
        >>> config_out = Path("/etc/smoothtask/tuned_config.yml")
        >>> tune_policy(db_path, config_out)
        >>>
        >>> # С сохранением остальных параметров из исходного конфига
        >>> config_in = Path("/etc/smoothtask/config.yml")
        >>> tune_policy(db_path, config_out, config_in=config_in)
    """
    # Базовая валидация входных данных
    _validate_db_path(db_path)

    with sqlite3.connect(db_path) as conn:
        _validate_db_schema(conn)

        # Проверяем минимальное количество снапшотов
        snapshot_count = _count_snapshots(conn, days_back=7)
        if snapshot_count < 100:
            raise ValueError(
                f"Недостаточно данных для тюнинга: найдено {snapshot_count} снапшотов, "
                "требуется минимум 100 за последние 7 дней"
            )

    # Загружаем снапшоты для тюнинга
    snapshots_df = load_snapshots_for_tuning(db_path, min_snapshots=100, days_back=7)

    # Оптимизируем пороги PSI
    psi_thresholds = optimize_psi_thresholds(snapshots_df, percentile=0.95)

    # Оптимизируем пороги latency
    latency_thresholds = optimize_latency_thresholds(
        snapshots_df, percentile=0.95, multiplier=1.5
    )

    # Формируем словарь с оптимизированными параметрами
    optimized_config = {
        "thresholds": {
            **psi_thresholds,
            **latency_thresholds,
        }
    }

    # Сохраняем оптимизированный конфиг
    save_optimized_config(optimized_config, config_out, config_in=config_in)
