"""Тесты для тюнинга параметров политики."""

import sqlite3
import tempfile
from datetime import datetime, timezone
from pathlib import Path

import pytest
import yaml

from smoothtask_trainer.tune_policy import (
    compute_policy_correlations,
    load_snapshots_for_tuning,
    optimize_psi_thresholds,
    tune_policy,
    _count_snapshots,
    _validate_db_path,
    _validate_db_schema,
)


def create_test_db(db_path: Path, num_snapshots: int = 5) -> None:
    """Создаёт тестовую БД с несколькими снапшотами для тюнинга."""
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    # Создаём схему
    cursor.execute(
        """
        CREATE TABLE IF NOT EXISTS snapshots (
            snapshot_id INTEGER PRIMARY KEY,
            timestamp TEXT NOT NULL,
            cpu_user REAL,
            cpu_system REAL,
            cpu_idle REAL,
            cpu_iowait REAL,
            mem_total_kb INTEGER,
            mem_used_kb INTEGER,
            mem_available_kb INTEGER,
            swap_total_kb INTEGER,
            swap_used_kb INTEGER,
            load_avg_one REAL,
            load_avg_five REAL,
            load_avg_fifteen REAL,
            psi_cpu_some_avg10 REAL,
            psi_cpu_some_avg60 REAL,
            psi_io_some_avg10 REAL,
            psi_mem_some_avg10 REAL,
            psi_mem_full_avg10 REAL,
            user_active INTEGER,
            time_since_last_input_ms INTEGER,
            sched_latency_p95_ms REAL,
            sched_latency_p99_ms REAL,
            audio_xruns_delta INTEGER,
            ui_loop_p95_ms REAL,
            frame_jank_ratio REAL,
            bad_responsiveness INTEGER,
            responsiveness_score REAL
        )
        """
    )

    cursor.execute(
        """
        CREATE TABLE IF NOT EXISTS processes (
            snapshot_id INTEGER NOT NULL,
            pid INTEGER NOT NULL,
            ppid INTEGER,
            app_group_id TEXT,
            exe_path TEXT,
            cgroup_path TEXT,
            process_type TEXT,
            tags TEXT,
            cpu_share REAL,
            io_read_bytes INTEGER,
            io_write_bytes INTEGER,
            rss_mb REAL,
            has_tty INTEGER,
            has_gui_window INTEGER,
            is_focused_window INTEGER,
            env_has_display INTEGER,
            env_has_wayland INTEGER,
            env_ssh INTEGER,
            is_audio_client INTEGER,
            has_active_stream INTEGER,
            FOREIGN KEY (snapshot_id) REFERENCES snapshots(snapshot_id)
        )
        """
    )

    cursor.execute(
        """
        CREATE TABLE IF NOT EXISTS app_groups (
            snapshot_id INTEGER NOT NULL,
            app_group_id TEXT NOT NULL,
            root_pid INTEGER,
            process_ids TEXT,
            app_name TEXT,
            total_cpu_share REAL,
            total_io_read_bytes INTEGER,
            total_io_write_bytes INTEGER,
            total_rss_mb REAL,
            has_gui_window INTEGER,
            is_focused_group INTEGER,
            tags TEXT,
            priority_class TEXT,
            FOREIGN KEY (snapshot_id) REFERENCES snapshots(snapshot_id)
        )
        """
    )

    # Вставляем тестовые данные
    base_time = datetime.now(timezone.utc)
    for i in range(num_snapshots):
        timestamp = (base_time.timestamp() + i * 60) * 1000
        snapshot_id = int(timestamp)

        # Вставляем снапшот с различными метриками отзывчивости
        cursor.execute(
            """
            INSERT INTO snapshots (
                snapshot_id, timestamp, cpu_user, cpu_system, cpu_idle, cpu_iowait,
                mem_total_kb, mem_used_kb, mem_available_kb, swap_total_kb, swap_used_kb,
                load_avg_one, load_avg_five, load_avg_fifteen,
                psi_cpu_some_avg10, psi_cpu_some_avg60, psi_io_some_avg10,
                psi_mem_some_avg10, psi_mem_full_avg10,
                user_active, time_since_last_input_ms,
                sched_latency_p95_ms, sched_latency_p99_ms,
                audio_xruns_delta, ui_loop_p95_ms, frame_jank_ratio,
                bad_responsiveness, responsiveness_score
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                snapshot_id,
                datetime.fromtimestamp(timestamp / 1000, tz=timezone.utc).isoformat(),
                0.3 + i * 0.1,  # cpu_user
                0.1,  # cpu_system
                0.5,  # cpu_idle
                0.1,  # cpu_iowait
                16_384_256,  # mem_total_kb
                8_000_000,  # mem_used_kb
                8_384_256,  # mem_available_kb
                8_192_000,  # swap_total_kb
                1_000_000,  # swap_used_kb
                1.5,  # load_avg_one
                1.2,  # load_avg_five
                1.0,  # load_avg_fifteen
                0.1 + i * 0.05,  # psi_cpu_some_avg10
                0.15,  # psi_cpu_some_avg60
                0.2,  # psi_io_some_avg10
                0.05,  # psi_mem_some_avg10
                None,  # psi_mem_full_avg10
                1,  # user_active
                5000,  # time_since_last_input_ms
                5.0,  # sched_latency_p95_ms
                10.0 + i * 2.0,  # sched_latency_p99_ms (увеличиваем для теста)
                0,  # audio_xruns_delta
                16.67,  # ui_loop_p95_ms
                0.0,  # frame_jank_ratio
                0 if i < 2 else 1,  # bad_responsiveness (первые 2 хорошие, остальные плохие)
                0.9 - i * 0.1,  # responsiveness_score
            ),
        )

    conn.commit()
    conn.close()


def create_test_config(config_path: Path) -> None:
    """Создаёт тестовый конфиг."""
    config = {
        "polling_interval_ms": 500,
        "max_candidates": 150,
        "dry_run_default": False,
        "policy_mode": "rules-only",
        "paths": {
            "snapshot_db_path": "/tmp/test_snapshots.sqlite",
            "patterns_dir": "/etc/smoothtask/patterns",
        },
        "thresholds": {
            "psi_cpu_some_high": 0.6,
            "psi_io_some_high": 0.4,
            "user_idle_timeout_sec": 120,
            "interactive_build_grace_sec": 10,
            "noisy_neighbour_cpu_share": 0.7,
            "crit_interactive_percentile": 0.9,
            "interactive_percentile": 0.6,
            "normal_percentile": 0.3,
            "background_percentile": 0.1,
            "sched_latency_p99_threshold_ms": 10.0,
            "ui_loop_p95_threshold_ms": 16.67,
        },
    }

    with open(config_path, "w") as f:
        yaml.dump(config, f)


def test_tune_policy_not_implemented():
    """Тест, что функция пока не реализована."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "test.db"
        config_path = Path(tmpdir) / "config.yml"

        # Создаём БД с достаточным количеством снапшотов для прохождения валидации
        create_test_db(db_path, num_snapshots=150)
        create_test_config(config_path)

        # Функция должна пройти валидацию и выбросить NotImplementedError
        with pytest.raises(NotImplementedError, match="TODO: реализовать тюнинг политики"):
            tune_policy(db_path, config_path)


def test_tune_policy_signature():
    """Тест, что функция принимает правильные параметры."""
    import inspect

    sig = inspect.signature(tune_policy)
    params = list(sig.parameters.keys())

    assert len(params) == 2, "tune_policy должна принимать 2 параметра"
    assert params[0] == "db_path", "Первый параметр должен быть db_path"
    assert params[1] == "config_out", "Второй параметр должен быть config_out"

    # Проверяем типы параметров
    assert sig.parameters["db_path"].annotation == Path or sig.parameters["db_path"].annotation == inspect.Parameter.empty
    assert sig.parameters["config_out"].annotation == Path or sig.parameters["config_out"].annotation == inspect.Parameter.empty


def test_tune_policy_with_nonexistent_db():
    """Тест обработки несуществующей БД."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "nonexistent.db"
        config_path = Path(tmpdir) / "config.yml"

        create_test_config(config_path)

        # Функция должна выбросить FileNotFoundError при валидации
        with pytest.raises(FileNotFoundError, match="База данных не найдена"):
            tune_policy(db_path, config_path)


def test_tune_policy_with_empty_db():
    """Тест обработки пустой БД."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "empty.db"
        config_path = Path(tmpdir) / "config.yml"

        # Создаём пустую БД
        conn = sqlite3.connect(db_path)
        conn.close()

        create_test_config(config_path)

        # Функция должна выбросить ValueError из-за отсутствия необходимых таблиц
        with pytest.raises(ValueError, match="База данных не содержит необходимые таблицы"):
            tune_policy(db_path, config_path)


def test_validate_db_path_with_existing_file():
    """Тест валидации существующего файла БД."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "test.db"
        create_test_db(db_path, num_snapshots=5)
        
        # Функция не должна выбрасывать исключение для существующего файла
        _validate_db_path(db_path)


def test_validate_db_path_with_nonexistent_file():
    """Тест валидации несуществующего файла БД."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "nonexistent.db"
        
        # Функция должна выбросить FileNotFoundError
        with pytest.raises(FileNotFoundError):
            _validate_db_path(db_path)


def test_validate_db_schema_with_valid_schema():
    """Тест валидации схемы БД с валидными таблицами."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "test.db"
        create_test_db(db_path, num_snapshots=5)
        
        with sqlite3.connect(db_path) as conn:
            # Функция не должна выбрасывать исключение для валидной схемы
            _validate_db_schema(conn)


def test_validate_db_schema_with_missing_tables():
    """Тест валидации схемы БД с отсутствующими таблицами."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "test.db"
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        
        # Создаём БД только с таблицей snapshots (без processes и app_groups)
        cursor.execute(
            """
            CREATE TABLE snapshots (
                snapshot_id INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL
            )
            """
        )
        conn.commit()
        conn.close()
        
        with sqlite3.connect(db_path) as conn:
            # Функция должна выбросить ValueError
            with pytest.raises(ValueError, match="не содержит необходимые таблицы"):
                _validate_db_schema(conn)


def test_count_snapshots_with_filter():
    """Тест подсчёта снапшотов с фильтрацией по времени."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "test.db"
        create_test_db(db_path, num_snapshots=5)
        
        with sqlite3.connect(db_path) as conn:
            count = _count_snapshots(conn, days_back=7)
            assert count == 5


def test_count_snapshots_without_filter():
    """Тест подсчёта снапшотов без фильтрации по времени."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "test.db"
        create_test_db(db_path, num_snapshots=5)
        
        with sqlite3.connect(db_path) as conn:
            count = _count_snapshots(conn, days_back=0)
            assert count == 5


def test_load_snapshots_for_tuning_with_sufficient_data():
    """Тест загрузки снапшотов с достаточным количеством данных."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "test.db"
        # Создаём БД с достаточным количеством снапшотов (100+)
        create_test_db(db_path, num_snapshots=150)
        
        df = load_snapshots_for_tuning(db_path, min_snapshots=100, days_back=7)
        
        assert len(df) == 150
        assert "snapshot_id" in df.columns
        assert "timestamp" in df.columns


def test_load_snapshots_for_tuning_with_insufficient_data():
    """Тест загрузки снапшотов с недостаточным количеством данных."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "test.db"
        create_test_db(db_path, num_snapshots=50)
        
        # Функция должна выбросить ValueError
        with pytest.raises(ValueError, match="Недостаточно данных для тюнинга"):
            load_snapshots_for_tuning(db_path, min_snapshots=100, days_back=7)


def test_load_snapshots_for_tuning_with_nonexistent_db():
    """Тест загрузки снапшотов из несуществующей БД."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "nonexistent.db"
        
        # Функция должна выбросить FileNotFoundError
        with pytest.raises(FileNotFoundError):
            load_snapshots_for_tuning(db_path, min_snapshots=100, days_back=7)


def test_tune_policy_with_insufficient_snapshots():
    """Тест тюнинга политики с недостаточным количеством снапшотов."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "test.db"
        config_path = Path(tmpdir) / "config.yml"
        
        create_test_db(db_path, num_snapshots=50)  # Меньше минимума (100)
        create_test_config(config_path)
        
        # Функция должна выбросить ValueError перед NotImplementedError
        with pytest.raises(ValueError, match="Недостаточно данных для тюнинга"):
            tune_policy(db_path, config_path)


def test_tune_policy_with_sufficient_snapshots():
    """Тест тюнинга политики с достаточным количеством снапшотов."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "test.db"
        config_path = Path(tmpdir) / "config.yml"
        
        create_test_db(db_path, num_snapshots=150)  # Больше минимума (100)
        create_test_config(config_path)
        
        # Функция должна пройти валидацию и выбросить NotImplementedError
        with pytest.raises(NotImplementedError, match="TODO: реализовать тюнинг политики"):
            tune_policy(db_path, config_path)


def test_compute_policy_correlations_basic():
    """Тест базового вычисления корреляций."""
    import pandas as pd
    
    # Создаём тестовый DataFrame с положительной корреляцией между PSI и bad_responsiveness
    df = pd.DataFrame({
        'psi_cpu_some_avg10': [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
        'psi_io_some_avg10': [0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4],
        'sched_latency_p99_ms': [5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0],
        'ui_loop_p95_ms': [10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0, 45.0],
        'bad_responsiveness': [0, 0, 0, 0, 1, 1, 1, 1],
        'responsiveness_score': [1.0, 0.9, 0.8, 0.7, 0.5, 0.4, 0.3, 0.2],
    })
    
    correlations = compute_policy_correlations(df)
    
    # Проверяем, что все корреляции вычислены
    assert 'psi_cpu_vs_bad_responsiveness' in correlations
    assert 'psi_io_vs_bad_responsiveness' in correlations
    assert 'sched_latency_vs_bad_responsiveness' in correlations
    assert 'ui_latency_vs_bad_responsiveness' in correlations
    assert 'psi_cpu_vs_responsiveness_score' in correlations
    assert 'psi_io_vs_responsiveness_score' in correlations
    assert 'sched_latency_vs_responsiveness_score' in correlations
    assert 'ui_latency_vs_responsiveness_score' in correlations
    
    # Проверяем, что корреляции находятся в допустимом диапазоне [-1, 1]
    for key, value in correlations.items():
        if not pd.isna(value):
            assert -1.0 <= value <= 1.0, f"Корреляция {key} = {value} вне диапазона [-1, 1]"
    
    # Проверяем, что корреляции с bad_responsiveness положительные (больше PSI/latency -> больше bad_responsiveness)
    if not pd.isna(correlations['psi_cpu_vs_bad_responsiveness']):
        assert correlations['psi_cpu_vs_bad_responsiveness'] > 0
    
    if not pd.isna(correlations['sched_latency_vs_bad_responsiveness']):
        assert correlations['sched_latency_vs_bad_responsiveness'] > 0
    
    # Проверяем, что корреляции с responsiveness_score отрицательные (больше PSI/latency -> меньше score)
    if not pd.isna(correlations['psi_cpu_vs_responsiveness_score']):
        assert correlations['psi_cpu_vs_responsiveness_score'] < 0
    
    if not pd.isna(correlations['sched_latency_vs_responsiveness_score']):
        assert correlations['sched_latency_vs_responsiveness_score'] < 0


def test_compute_policy_correlations_empty_dataframe():
    """Тест вычисления корреляций для пустого DataFrame."""
    import pandas as pd
    
    df = pd.DataFrame()
    correlations = compute_policy_correlations(df)
    
    # Все корреляции должны быть NaN
    for key, value in correlations.items():
        assert pd.isna(value), f"Корреляция {key} должна быть NaN для пустого DataFrame, но получили {value}"


def test_compute_policy_correlations_missing_columns():
    """Тест вычисления корреляций при отсутствии некоторых колонок."""
    import pandas as pd
    
    # DataFrame без некоторых колонок
    df = pd.DataFrame({
        'psi_cpu_some_avg10': [0.1, 0.2, 0.3],
        'bad_responsiveness': [0, 0, 1],
        'responsiveness_score': [1.0, 0.9, 0.7],
    })
    
    correlations = compute_policy_correlations(df)
    
    # Корреляции для существующих колонок должны быть вычислены
    assert not pd.isna(correlations['psi_cpu_vs_bad_responsiveness'])
    assert not pd.isna(correlations['psi_cpu_vs_responsiveness_score'])
    
    # Корреляции для отсутствующих колонок должны быть NaN
    assert pd.isna(correlations['psi_io_vs_bad_responsiveness'])
    assert pd.isna(correlations['sched_latency_vs_bad_responsiveness'])
    assert pd.isna(correlations['ui_latency_vs_bad_responsiveness'])


def test_compute_policy_correlations_with_nulls():
    """Тест вычисления корреляций при наличии NULL значений."""
    import pandas as pd
    
    df = pd.DataFrame({
        'psi_cpu_some_avg10': [0.1, 0.2, None, 0.4, 0.5],
        'psi_io_some_avg10': [0.05, None, 0.15, 0.2, 0.25],
        'sched_latency_p99_ms': [5.0, 10.0, 15.0, None, 25.0],
        'ui_loop_p95_ms': [10.0, 15.0, None, 25.0, 30.0],
        'bad_responsiveness': [0, 0, 1, 1, 1],
        'responsiveness_score': [1.0, 0.9, None, 0.5, 0.3],
    })
    
    correlations = compute_policy_correlations(df)
    
    # Функция должна корректно обработать NULL значения (dropna перед вычислением корреляции)
    # Проверяем, что функция не падает и возвращает корректные значения или NaN
    for key, value in correlations.items():
        if not pd.isna(value):
            assert -1.0 <= value <= 1.0, f"Корреляция {key} = {value} вне диапазона [-1, 1]"


def test_compute_policy_correlations_single_value():
    """Тест вычисления корреляций при наличии только одного значения (недостаточно для корреляции)."""
    import pandas as pd
    
    df = pd.DataFrame({
        'psi_cpu_some_avg10': [0.1],
        'psi_io_some_avg10': [0.05],
        'sched_latency_p99_ms': [5.0],
        'ui_loop_p95_ms': [10.0],
        'bad_responsiveness': [0],
        'responsiveness_score': [1.0],
    })
    
    correlations = compute_policy_correlations(df)
    
    # Все корреляции должны быть NaN, так как недостаточно данных (нужно минимум 2 точки)
    for key, value in correlations.items():
        assert pd.isna(value), f"Корреляция {key} должна быть NaN для одного значения, но получили {value}"


def test_compute_policy_correlations_with_real_data():
    """Тест вычисления корреляций с данными из реальной БД."""
    import pandas as pd
    
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "test.db"
        create_test_db(db_path, num_snapshots=150)
        
        # Загружаем снапшоты
        df = load_snapshots_for_tuning(db_path, min_snapshots=100, days_back=7)
        
        # Вычисляем корреляции
        correlations = compute_policy_correlations(df)
        
        # Проверяем, что все корреляции вычислены (могут быть NaN, если данных недостаточно)
        assert len(correlations) == 8
        
        # Проверяем, что корреляции находятся в допустимом диапазоне
        for key, value in correlations.items():
            if not pd.isna(value):
                assert -1.0 <= value <= 1.0, f"Корреляция {key} = {value} вне диапазона [-1, 1]"


def test_optimize_psi_thresholds_basic():
    """Тест базовой оптимизации порогов PSI."""
    import pandas as pd
    
    # Создаём тестовый DataFrame с моментами bad_responsiveness
    df = pd.DataFrame({
        'psi_cpu_some_avg10': [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
        'psi_io_some_avg10': [0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4],
        'bad_responsiveness': [0, 0, 0, 0, 1, 1, 1, 1],
    })
    
    thresholds = optimize_psi_thresholds(df, percentile=0.95)
    
    # Проверяем, что пороги вычислены
    assert 'psi_cpu_some_high' in thresholds
    assert 'psi_io_some_high' in thresholds
    
    # Проверяем, что пороги находятся в допустимом диапазоне [0.0, 1.0]
    assert 0.0 <= thresholds['psi_cpu_some_high'] <= 1.0
    assert 0.0 <= thresholds['psi_io_some_high'] <= 1.0
    
    # Проверяем, что пороги выше значений в хороших условиях
    # (в плохих условиях PSI выше, поэтому порог должен быть выше среднего)
    assert thresholds['psi_cpu_some_high'] > 0.4  # выше среднего значения в плохих условиях
    assert thresholds['psi_io_some_high'] > 0.2  # выше среднего значения в плохих условиях


def test_optimize_psi_thresholds_empty_dataframe():
    """Тест оптимизации порогов PSI для пустого DataFrame."""
    import pandas as pd
    
    df = pd.DataFrame()
    thresholds = optimize_psi_thresholds(df)
    
    # Должны вернуться значения по умолчанию
    assert thresholds['psi_cpu_some_high'] == 0.6
    assert thresholds['psi_io_some_high'] == 0.4


def test_optimize_psi_thresholds_no_bad_responsiveness():
    """Тест оптимизации порогов PSI когда нет моментов bad_responsiveness."""
    import pandas as pd
    
    # Создаём DataFrame только с хорошими условиями
    df = pd.DataFrame({
        'psi_cpu_some_avg10': [0.1, 0.2, 0.3, 0.4],
        'psi_io_some_avg10': [0.05, 0.1, 0.15, 0.2],
        'bad_responsiveness': [0, 0, 0, 0],
    })
    
    thresholds = optimize_psi_thresholds(df)
    
    # Должны вернуться значения по умолчанию
    assert thresholds['psi_cpu_some_high'] == 0.6
    assert thresholds['psi_io_some_high'] == 0.4


def test_optimize_psi_thresholds_missing_columns():
    """Тест оптимизации порогов PSI при отсутствии некоторых колонок."""
    import pandas as pd
    
    # DataFrame без колонки psi_io_some_avg10
    df = pd.DataFrame({
        'psi_cpu_some_avg10': [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
        'bad_responsiveness': [0, 0, 0, 0, 1, 1, 1, 1],
    })
    
    thresholds = optimize_psi_thresholds(df)
    
    # psi_cpu_some_high должен быть вычислен
    assert 'psi_cpu_some_high' in thresholds
    assert 0.0 <= thresholds['psi_cpu_some_high'] <= 1.0
    
    # psi_io_some_high должен быть значением по умолчанию
    assert thresholds['psi_io_some_high'] == 0.4


def test_optimize_psi_thresholds_with_nulls():
    """Тест оптимизации порогов PSI при наличии NULL значений."""
    import pandas as pd
    
    df = pd.DataFrame({
        'psi_cpu_some_avg10': [0.1, 0.2, None, 0.4, 0.5, 0.6, 0.7, 0.8],
        'psi_io_some_avg10': [0.05, None, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4],
        'bad_responsiveness': [0, 0, 0, 0, 1, 1, 1, 1],
    })
    
    thresholds = optimize_psi_thresholds(df)
    
    # Функция должна корректно обработать NULL значения (dropna перед вычислением)
    assert 0.0 <= thresholds['psi_cpu_some_high'] <= 1.0
    assert 0.0 <= thresholds['psi_io_some_high'] <= 1.0


def test_optimize_psi_thresholds_percentile():
    """Тест оптимизации порогов PSI с различными перцентилями."""
    import pandas as pd
    
    df = pd.DataFrame({
        'psi_cpu_some_avg10': [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
        'psi_io_some_avg10': [0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4],
        'bad_responsiveness': [0, 0, 0, 0, 1, 1, 1, 1],
    })
    
    # Тестируем с различными перцентилями
    thresholds_p50 = optimize_psi_thresholds(df, percentile=0.5)
    thresholds_p95 = optimize_psi_thresholds(df, percentile=0.95)
    thresholds_p99 = optimize_psi_thresholds(df, percentile=0.99)
    
    # P95 должен быть выше P50
    assert thresholds_p95['psi_cpu_some_high'] >= thresholds_p50['psi_cpu_some_high']
    assert thresholds_p95['psi_io_some_high'] >= thresholds_p50['psi_io_some_high']
    
    # P99 должен быть выше или равен P95
    assert thresholds_p99['psi_cpu_some_high'] >= thresholds_p95['psi_cpu_some_high']
    assert thresholds_p99['psi_io_some_high'] >= thresholds_p95['psi_io_some_high']


def test_optimize_psi_thresholds_with_real_data():
    """Тест оптимизации порогов PSI с данными из реальной БД."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "test.db"
        create_test_db(db_path, num_snapshots=150)
        
        # Загружаем снапшоты
        df = load_snapshots_for_tuning(db_path, min_snapshots=100, days_back=7)
        
        # Оптимизируем пороги PSI
        thresholds = optimize_psi_thresholds(df, percentile=0.95)
        
        # Проверяем, что пороги вычислены и находятся в допустимом диапазоне
        assert 'psi_cpu_some_high' in thresholds
        assert 'psi_io_some_high' in thresholds
        assert 0.0 <= thresholds['psi_cpu_some_high'] <= 1.0
        assert 0.0 <= thresholds['psi_io_some_high'] <= 1.0
