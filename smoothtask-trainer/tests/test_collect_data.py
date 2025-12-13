"""Тесты для модуля сбора данных."""

import gzip
import json
import sqlite3
import tempfile
from datetime import datetime, timezone
from pathlib import Path

import pytest
from smoothtask_trainer.collect_data import (
    collect_data_from_snapshots,
    load_dataset,
    validate_dataset,
)


def create_test_snapshot_file(snapshot_file: Path, num_snapshots: int = 2, base_snapshot_id: int = 1000, processes_per_snapshot: int = 1) -> None:
    """Создает тестовый файл снапшотов."""
    snapshots = []
    
    for snapshot_idx in range(num_snapshots):
        snapshot_id = base_snapshot_id + snapshot_idx
        timestamp = datetime.now(timezone.utc).replace(second=snapshot_idx).isoformat()
        
        # Создаем несколько процессов для каждого снапшота
        processes = []
        process_ids = []
        for proc_idx in range(processes_per_snapshot):
            pid = 1000 + snapshot_idx * 10 + proc_idx
            processes.append({
                "pid": pid,
                "ppid": 1,
                "uid": 1000,
                "gid": 1000,
                "exe": f"/usr/bin/test{snapshot_idx}_{proc_idx}",
                "cmdline": f"test{snapshot_idx}_{proc_idx} --flag",
                "cgroup_path": "/user.slice/user-1000.slice",
                "systemd_unit": None,
                "app_group_id": f"test-app-{snapshot_idx}_{proc_idx}",
                "state": "R",
                "start_time": 1000000 + snapshot_idx + proc_idx,
                "uptime_sec": 3600,
                "tty_nr": 0,
                "has_tty": 0,
                "cpu_share_1s": 0.1 + snapshot_idx * 0.01 + proc_idx * 0.001,
                "cpu_share_10s": 0.08 + snapshot_idx * 0.01 + proc_idx * 0.001,
                "io_read_bytes": 1024 * 1024 * (snapshot_idx + 1 + proc_idx),
                "io_write_bytes": 512 * 1024 * (snapshot_idx + 1 + proc_idx),
                "rss_mb": 100 + snapshot_idx * 10 + proc_idx * 5,
                "swap_mb": 0,
                "voluntary_ctx": 1000 + snapshot_idx * 100 + proc_idx * 10,
                "involuntary_ctx": 50 + snapshot_idx * 10 + proc_idx * 2,
                "has_gui_window": 0,
                "is_focused_window": 0,
                "window_state": None,
                "env_has_display": 0,
                "env_has_wayland": 0,
                "env_term": None,
                "env_ssh": 0,
                "is_audio_client": 0,
                "has_active_stream": 0,
                "process_type": "cli_interactive",
                "tags": ["terminal"],
                "nice": 0,
                "ionice_class": 2,
                "ionice_prio": 4,
                "teacher_priority_class": "INTERACTIVE",
                "teacher_score": 0.75 + snapshot_idx * 0.05 + proc_idx * 0.01,
            })
            process_ids.append(pid)
        
        # Создаем группы приложений
        app_groups = []
        for proc_idx in range(processes_per_snapshot):
            app_groups.append({
                "app_group_id": f"test-app-{snapshot_idx}_{proc_idx}",
                "root_pid": 1000 + snapshot_idx * 10 + proc_idx,
                "process_ids": [1000 + snapshot_idx * 10 + proc_idx],
                "app_name": f"test{snapshot_idx}_{proc_idx}",
                "total_cpu_share": 0.15 + snapshot_idx * 0.01 + proc_idx * 0.001,
                "total_io_read_bytes": 2 * 1024 * 1024 * (snapshot_idx + 1 + proc_idx),
                "total_io_write_bytes": 1024 * 1024 * (snapshot_idx + 1 + proc_idx),
                "total_rss_mb": 200 + snapshot_idx * 20 + proc_idx * 10,
                "has_gui_window": 0,
                "is_focused_group": 0,
                "tags": ["terminal"],
                "priority_class": "INTERACTIVE",
            })
        
        snapshot = {
            "snapshot_id": snapshot_id,
            "timestamp": timestamp,
            "cpu_user": 0.25 + snapshot_idx * 0.01,
            "cpu_system": 0.15,
            "cpu_idle": 0.55,
            "cpu_iowait": 0.05,
            "mem_total_kb": 16_384_256,
            "mem_used_kb": 8_000_000,
            "mem_available_kb": 8_384_256,
            "swap_total_kb": 8_192_000,
            "swap_used_kb": 1_000_000,
            "load_avg_one": 1.5 + snapshot_idx * 0.1,
            "load_avg_five": 1.2,
            "load_avg_fifteen": 1.0,
            "psi_cpu_some_avg10": 0.1,
            "psi_cpu_some_avg60": 0.15,
            "psi_io_some_avg10": 0.2,
            "psi_mem_some_avg10": 0.05,
            "psi_mem_full_avg10": 0.0,
            "user_active": 1,
            "time_since_last_input_ms": 5000 - snapshot_idx * 100,
            "sched_latency_p95_ms": 5.0,
            "sched_latency_p99_ms": 10.0,
            "audio_xruns_delta": None,
            "ui_loop_p95_ms": None,
            "frame_jank_ratio": None,
            "bad_responsiveness": 0,
            "responsiveness_score": 0.9 - snapshot_idx * 0.05,
            "processes": processes,
            "app_groups": app_groups
        }
        
        snapshots.append(snapshot)
    
    # Записываем снапшоты в файл
    with open(snapshot_file, 'w', encoding='utf-8') as f:
        for snapshot in snapshots:
            f.write(json.dumps(snapshot) + '\n')


def create_test_snapshot_gz_file(snapshot_file: Path, num_snapshots: int = 2) -> None:
    """Создает тестовый GZIP файл снапшотов."""
    with tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='.jsonl') as temp_file:
        create_test_snapshot_file(Path(temp_file.name), num_snapshots)
        temp_path = Path(temp_file.name)
    
    # Сжимаем файл
    with open(temp_path, 'rb') as f_in:
        with gzip.open(snapshot_file, 'wb') as f_out:
            f_out.writelines(f_in)
    
    # Удаляем временный файл
    temp_path.unlink()


def test_collect_data_from_snapshots_basic():
    """Тест базового сбора данных из снапшотов."""
    with tempfile.TemporaryDirectory() as tmpdir:
        snapshot_file = Path(tmpdir) / "test_snapshots.jsonl"
        create_test_snapshot_file(snapshot_file, num_snapshots=2)
        
        # Собираем данные
        db_path = collect_data_from_snapshots(snapshot_file)
        
        # Проверяем, что база данных создана
        assert db_path.exists(), "База данных должна быть создана"
        
        # Проверяем, что база данных содержит таблицы
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        
        cursor.execute("SELECT name FROM sqlite_master WHERE type='table'")
        tables = [row[0] for row in cursor.fetchall()]
        
        assert "snapshots" in tables, "Таблица snapshots должна существовать"
        assert "processes" in tables, "Таблица processes должна существовать"
        assert "app_groups" in tables, "Таблица app_groups должна существовать"
        
        # Проверяем количество записей
        cursor.execute("SELECT COUNT(*) FROM snapshots")
        snapshot_count = cursor.fetchone()[0]
        assert snapshot_count == 2, f"Ожидалось 2 снапшота, получено {snapshot_count}"
        
        cursor.execute("SELECT COUNT(*) FROM processes")
        process_count = cursor.fetchone()[0]
        assert process_count == 2, f"Ожидалось 2 процесса, получено {process_count}"
        
        cursor.execute("SELECT COUNT(*) FROM app_groups")
        group_count = cursor.fetchone()[0]
        assert group_count == 2, f"Ожидалось 2 группы, получено {group_count}"
        
        conn.close()
        
        # Удаляем базу данных
        db_path.unlink()


def test_collect_data_from_gz_snapshots():
    """Тест сбора данных из сжатых снапшотов."""
    with tempfile.TemporaryDirectory() as tmpdir:
        snapshot_file = Path(tmpdir) / "test_snapshots.jsonl.gz"
        create_test_snapshot_gz_file(snapshot_file, num_snapshots=3)
        
        # Собираем данные
        db_path = collect_data_from_snapshots(snapshot_file)
        
        # Проверяем, что база данных создана
        assert db_path.exists(), "База данных должна быть создана"
        
        # Проверяем количество записей
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        
        cursor.execute("SELECT COUNT(*) FROM snapshots")
        snapshot_count = cursor.fetchone()[0]
        assert snapshot_count == 3, f"Ожидалось 3 снапшота, получено {snapshot_count}"
        
        conn.close()
        
        # Удаляем базу данных
        db_path.unlink()


def test_collect_data_from_multiple_snapshots():
    """Тест сбора данных из нескольких файлов снапшотов."""
    with tempfile.TemporaryDirectory() as tmpdir:
        snapshot_file1 = Path(tmpdir) / "test_snapshots1.jsonl"
        snapshot_file2 = Path(tmpdir) / "test_snapshots2.jsonl"
        
        create_test_snapshot_file(snapshot_file1, num_snapshots=2, base_snapshot_id=1000)
        create_test_snapshot_file(snapshot_file2, num_snapshots=3, base_snapshot_id=2000)
        
        # Собираем данные
        db_path = collect_data_from_snapshots([snapshot_file1, snapshot_file2])
        
        # Проверяем, что база данных создана
        assert db_path.exists(), "База данных должна быть создана"
        
        # Проверяем количество записей
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        
        cursor.execute("SELECT COUNT(*) FROM snapshots")
        snapshot_count = cursor.fetchone()[0]
        assert snapshot_count == 5, f"Ожидалось 5 снапшотов, получено {snapshot_count}"
        
        conn.close()
        
        # Удаляем базу данных
        db_path.unlink()


def test_collect_data_with_output_path():
    """Тест сбора данных с указанием выходного пути."""
    with tempfile.TemporaryDirectory() as tmpdir:
        snapshot_file = Path(tmpdir) / "test_snapshots.jsonl"
        output_db = Path(tmpdir) / "output.db"
        
        create_test_snapshot_file(snapshot_file, num_snapshots=2)
        
        # Собираем данные с указанием выходного пути
        db_path = collect_data_from_snapshots(snapshot_file, output_db=output_db)
        
        # Проверяем, что база данных создана по указанному пути
        assert db_path == output_db, "Путь к базе данных должен совпадать с указанным"
        assert output_db.exists(), "База данных должна быть создана"
        
        # Удаляем базу данных
        output_db.unlink()


def test_collect_data_error_handling():
    """Тест обработки ошибок при сборе данных."""
    with tempfile.TemporaryDirectory() as tmpdir:
        nonexistent_file = Path(tmpdir) / "nonexistent.jsonl"
        
        # Проверяем обработку несуществующего файла
        with pytest.raises(FileNotFoundError):
            collect_data_from_snapshots(nonexistent_file)
        
        # Проверяем обработку пустого списка файлов
        with pytest.raises(ValueError):
            collect_data_from_snapshots([])


def test_validate_dataset_basic():
    """Тест базовой валидации датасета."""
    with tempfile.TemporaryDirectory() as tmpdir:
        snapshot_file = Path(tmpdir) / "test_snapshots.jsonl"
        db_path = Path(tmpdir) / "test.db"
        
        create_test_snapshot_file(snapshot_file, num_snapshots=3)
        collect_data_from_snapshots(snapshot_file, output_db=db_path)
        
        # Валидируем датасет с подходящими минимальными требованиями
        stats = validate_dataset(db_path, min_snapshots=1, min_processes=1, min_groups=1)
        
        # Проверяем статистику
        assert stats["snapshot_count"] == 3
        assert stats["process_count"] == 3
        assert stats["group_count"] == 3
        assert stats["validation_passed"] is True
        
        # Удаляем базу данных
        db_path.unlink()


def test_validate_dataset_with_min_requirements():
    """Тест валидации с минимальными требованиями."""
    with tempfile.TemporaryDirectory() as tmpdir:
        snapshot_file = Path(tmpdir) / "test_snapshots.jsonl"
        db_path = Path(tmpdir) / "test.db"
        
        create_test_snapshot_file(snapshot_file, num_snapshots=5)
        collect_data_from_snapshots(snapshot_file, output_db=db_path)
        
        # Валидируем с высокими минимальными требованиями
        stats = validate_dataset(
            db_path,
            min_snapshots=3,
            min_processes=4,
            min_groups=2
        )
        
        # Проверяем, что валидация прошла
        assert stats["validation_passed"] is True
        
        # Проверяем обработку ошибки при недостаточных данных
        with pytest.raises(ValueError):
            validate_dataset(
                db_path,
                min_snapshots=10,  # Слишком высокое требование
                min_processes=10,
                min_groups=10
            )
        
        # Удаляем базу данных
        db_path.unlink()


def test_validate_dataset_error_handling():
    """Тест обработки ошибок при валидации датасета."""
    with tempfile.TemporaryDirectory() as tmpdir:
        nonexistent_db = Path(tmpdir) / "nonexistent.db"
        
        # Проверяем обработку несуществующей базы данных
        with pytest.raises(FileNotFoundError):
            validate_dataset(nonexistent_db)


def test_load_dataset_basic():
    """Тест базовой загрузки датасета."""
    with tempfile.TemporaryDirectory() as tmpdir:
        snapshot_file = Path(tmpdir) / "test_snapshots.jsonl"
        db_path = Path(tmpdir) / "test.db"
        
        create_test_snapshot_file(snapshot_file, num_snapshots=2)
        collect_data_from_snapshots(snapshot_file, output_db=db_path)
        
        # Загружаем датасет
        df = load_dataset(db_path, validate=False)
        
        # Проверяем, что DataFrame не пустой
        assert not df.empty, "DataFrame не должен быть пустым"
        
        # Проверяем наличие ожидаемых столбцов
        assert "snapshot_id" in df.columns, "Столбец snapshot_id должен присутствовать"
        assert "pid" in df.columns, "Столбец pid должен присутствовать"
        assert "app_group_id" in df.columns, "Столбец app_group_id должен присутствовать"
        
        # Удаляем базу данных
        db_path.unlink()


def test_load_dataset_with_validation():
    """Тест загрузки датасета с валидацией."""
    with tempfile.TemporaryDirectory() as tmpdir:
        snapshot_file = Path(tmpdir) / "test_snapshots.jsonl"
        db_path = Path(tmpdir) / "test.db"
        
        create_test_snapshot_file(snapshot_file, num_snapshots=3, processes_per_snapshot=4)
        collect_data_from_snapshots(snapshot_file, output_db=db_path)
        
        # Загружаем датасет с валидацией
        df = load_dataset(db_path, validate=True)
        
        # Проверяем, что DataFrame не пустой
        assert not df.empty, "DataFrame не должен быть пустым"
        
        # Удаляем базу данных
        db_path.unlink()


def test_load_dataset_error_handling():
    """Тест обработки ошибок при загрузке датасета."""
    with tempfile.TemporaryDirectory() as tmpdir:
        nonexistent_db = Path(tmpdir) / "nonexistent.db"
        
        # Проверяем обработку несуществующей базы данных
        with pytest.raises(FileNotFoundError):
            load_dataset(nonexistent_db, validate=False)
        
        # Проверяем обработку невалидного датасета
        empty_db = Path(tmpdir) / "empty.db"
        conn = sqlite3.connect(empty_db)
        cursor = conn.cursor()
        
        # Создаем пустые таблицы
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS snapshots (
                snapshot_id INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL
            )
            """
        )
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS processes (
                snapshot_id INTEGER NOT NULL,
                pid INTEGER NOT NULL,
                PRIMARY KEY (snapshot_id, pid)
            )
            """
        )
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS app_groups (
                snapshot_id INTEGER NOT NULL,
                app_group_id TEXT NOT NULL,
                PRIMARY KEY (snapshot_id, app_group_id)
            )
            """
        )
        
        conn.commit()
        conn.close()
        
        # Проверяем обработку пустого датасета
        with pytest.raises(ValueError):
            load_dataset(empty_db, validate=True)