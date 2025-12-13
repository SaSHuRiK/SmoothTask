"""Сбор и подготовка данных для обучения ML-моделей."""

from __future__ import annotations

import gzip
import json
import sqlite3
import tempfile
from pathlib import Path
from typing import Iterable, List, Optional, Union

import pandas as pd
from tqdm import tqdm

from .dataset import load_snapshots_as_frame


def _parse_snapshot_line(line: str) -> dict:
    """
    Парсит строку из JSONL файла снапшота.
    
    Args:
        line: Строка из JSONL файла
        
    Returns:
        Словарь с данными снапшота
        
    Raises:
        ValueError: если строка не является валидным JSON
    """
    try:
        return json.loads(line)
    except json.JSONDecodeError as exc:
        raise ValueError(f"Некорректный JSON в строке снапшота: {exc}") from exc


def _extract_snapshot_data(snapshot: dict) -> dict:
    """
    Извлекает данные из структуры снапшота и нормализует их.
    
    Args:
        snapshot: Словарь с данными снапшота
        
    Returns:
        Нормализованный словарь с данными для DataFrame
    """
    # Извлекаем основные данные снапшота
    snapshot_data = {
        "snapshot_id": snapshot.get("snapshot_id", 0),
        "timestamp": snapshot.get("timestamp", ""),
        **{f"{key}": value for key, value in snapshot.items() 
           if key not in ["snapshot_id", "timestamp", "processes", "app_groups"]}
    }
    
    # Извлекаем процессы
    processes = []
    for proc in snapshot.get("processes", []):
        process_data = {
            "snapshot_id": snapshot_data["snapshot_id"],
            **proc
        }
        # Конвертируем списки в JSON строки для совместимости с SQLite
        if "tags" in process_data and isinstance(process_data["tags"], list):
            process_data["tags"] = json.dumps(process_data["tags"])
        processes.append(process_data)
    
    # Извлекаем группы приложений
    app_groups = []
    for group in snapshot.get("app_groups", []):
        group_data = {
            "snapshot_id": snapshot_data["snapshot_id"],
            **group
        }
        # Конвертируем списки в JSON строки для совместимости с SQLite
        if "tags" in group_data and isinstance(group_data["tags"], list):
            group_data["tags"] = json.dumps(group_data["tags"])
        if "process_ids" in group_data and isinstance(group_data["process_ids"], list):
            group_data["process_ids"] = json.dumps(group_data["process_ids"])
        app_groups.append(group_data)
    
    return {
        "snapshot": snapshot_data,
        "processes": processes,
        "app_groups": app_groups
    }


def _create_sqlite_from_snapshots(
    snapshot_files: Iterable[Path], 
    db_path: Path, 
    chunk_size: int = 1000
) -> None:
    """
    Создает SQLite базу данных из JSONL файлов снапшотов.
    
    Args:
        snapshot_files: Итератор с путями к файлам снапшотов
        db_path: Путь для сохранения SQLite базы данных
        chunk_size: Размер пакета для вставки данных
        
    Raises:
        ValueError: если не удалось создать базу данных
    """
    try:
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        
        # Создаем таблицы
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
                uid INTEGER,
                gid INTEGER,
                exe TEXT,
                cmdline TEXT,
                cgroup_path TEXT,
                systemd_unit TEXT,
                app_group_id TEXT,
                state TEXT,
                start_time INTEGER,
                uptime_sec INTEGER,
                tty_nr INTEGER,
                has_tty INTEGER,
                cpu_share_1s REAL,
                cpu_share_10s REAL,
                io_read_bytes INTEGER,
                io_write_bytes INTEGER,
                rss_mb INTEGER,
                swap_mb INTEGER,
                voluntary_ctx INTEGER,
                involuntary_ctx INTEGER,
                has_gui_window INTEGER,
                is_focused_window INTEGER,
                window_state TEXT,
                env_has_display INTEGER,
                env_has_wayland INTEGER,
                env_term TEXT,
                env_ssh INTEGER,
                is_audio_client INTEGER,
                has_active_stream INTEGER,
                process_type TEXT,
                tags TEXT,
                nice INTEGER,
                ionice_class INTEGER,
                ionice_prio INTEGER,
                teacher_priority_class TEXT,
                teacher_score REAL,
                PRIMARY KEY (snapshot_id, pid)
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
                total_rss_mb INTEGER,
                has_gui_window INTEGER,
                is_focused_group INTEGER,
                tags TEXT,
                priority_class TEXT,
                PRIMARY KEY (snapshot_id, app_group_id)
            )
            """
        )
        
        # Подготавливаем данные для вставки
        snapshot_data = []
        process_data = []
        app_group_data = []
        
        for snapshot_file in tqdm(snapshot_files, desc="Обработка файлов снапшотов"):
            if snapshot_file.suffix == ".gz":
                with gzip.open(snapshot_file, 'rt', encoding='utf-8') as f:
                    lines = f.readlines()
            else:
                with open(snapshot_file, 'r', encoding='utf-8') as f:
                    lines = f.readlines()
            
            for line in tqdm(lines, desc=f"Обработка {snapshot_file.name}", leave=False):
                try:
                    snapshot = _parse_snapshot_line(line)
                    extracted = _extract_snapshot_data(snapshot)
                    
                    # Добавляем данные снапшота
                    snapshot_data.append(extracted["snapshot"])
                    
                    # Добавляем данные процессов
                    process_data.extend(extracted["processes"])
                    
                    # Добавляем данные групп
                    app_group_data.extend(extracted["app_groups"])
                    
                    # Вставляем пакетами для оптимизации
                    if len(snapshot_data) >= chunk_size:
                        _insert_data_batch(cursor, snapshot_data, process_data, app_group_data)
                        snapshot_data = []
                        process_data = []
                        app_group_data = []
                        
                except Exception as e:
                    print(f"Ошибка при обработке строки: {e}")
                    continue
        
        # Вставляем оставшиеся данные
        if snapshot_data:
            _insert_data_batch(cursor, snapshot_data, process_data, app_group_data)
        
        conn.commit()
        conn.close()
        
    except Exception as e:
        if conn:
            conn.close()
        raise ValueError(f"Ошибка при создании базы данных: {e}") from e


def _insert_data_batch(
    cursor: sqlite3.Cursor, 
    snapshot_data: List[dict], 
    process_data: List[dict], 
    app_group_data: List[dict]
) -> None:
    """
    Вставляет пакет данных в базу данных.
    
    Args:
        cursor: Курсор SQLite
        snapshot_data: Данные снапшотов для вставки
        process_data: Данные процессов для вставки
        app_group_data: Данные групп для вставки
    """
    # Вставляем снапшоты
    if snapshot_data:
        columns = list(snapshot_data[0].keys())
        placeholders = ", ".join(["?"] * len(columns))
        cursor.executemany(
            f"INSERT OR REPLACE INTO snapshots ({', '.join(columns)}) VALUES ({placeholders})",
            [list(row.values()) for row in snapshot_data]
        )
    
    # Вставляем процессы
    if process_data:
        columns = list(process_data[0].keys())
        placeholders = ", ".join(["?"] * len(columns))
        cursor.executemany(
            f"INSERT OR REPLACE INTO processes ({', '.join(columns)}) VALUES ({placeholders})",
            [list(row.values()) for row in process_data]
        )
    
    # Вставляем группы
    if app_group_data:
        columns = list(app_group_data[0].keys())
        placeholders = ", ".join(["?"] * len(columns))
        cursor.executemany(
            f"INSERT OR REPLACE INTO app_groups ({', '.join(columns)}) VALUES ({placeholders})",
            [list(row.values()) for row in app_group_data]
        )


def collect_data_from_snapshots(
    snapshot_files: Union[Path, Iterable[Path]], 
    output_db: Optional[Path] = None,
    use_temp_db: bool = True
) -> Path:
    """
    Собирает данные из JSONL файлов снапшотов и создает SQLite базу данных.
    
    Args:
        snapshot_files: Путь к файлу снапшота или итератор с путями
        output_db: Путь для сохранения SQLite базы данных
        use_temp_db: Использовать временную базу данных, если output_db не указан
        
    Returns:
        Путь к созданной базе данных
        
    Raises:
        ValueError: если не удалось обработать файлы снапшотов
        FileNotFoundError: если файлы снапшотов не найдены
    """
    # Преобразуем одиночный путь в итератор
    if isinstance(snapshot_files, Path):
        snapshot_files = [snapshot_files]
    
    # Проверяем существование файлов
    file_list = list(snapshot_files)
    if not file_list:
        raise ValueError("Не указаны файлы снапшотов")
    
    for file_path in file_list:
        if not file_path.exists():
            raise FileNotFoundError(f"Файл снапшота не найден: {file_path}")
    
    # Определяем путь для базы данных
    if output_db:
        db_path = output_db
        use_temp_db = False
    elif use_temp_db:
        db_path = Path(tempfile.NamedTemporaryFile(suffix=".db", delete=False).name)
    else:
        raise ValueError("Не указан путь для сохранения базы данных")
    
    try:
        # Создаем базу данных из снапшотов
        _create_sqlite_from_snapshots(file_list, db_path)
        
        # Проверяем, что база данных создана успешно
        if not db_path.exists():
            raise ValueError("Не удалось создать базу данных")
        
        return db_path
        
    except Exception as e:
        # Удаляем временную базу данных в случае ошибки
        if use_temp_db and db_path.exists():
            db_path.unlink()
        raise ValueError(f"Ошибка при сборе данных: {e}") from e


def validate_dataset(
    db_path: Path,
    min_snapshots: int = 1,
    min_processes: int = 10,
    min_groups: int = 1
) -> dict:
    """
    Валидирует датасет и возвращает статистику.
    
    Args:
        db_path: Путь к SQLite базе данных
        min_snapshots: Минимальное количество снапшотов
        min_processes: Минимальное количество процессов
        min_groups: Минимальное количество групп
        
    Returns:
        Словарь со статистикой датасета
        
    Raises:
        ValueError: если датасет не проходит валидацию
        FileNotFoundError: если база данных не найдена
    """
    if not db_path.exists():
        raise FileNotFoundError(f"База данных не найдена: {db_path}")
    
    try:
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        
        # Получаем статистику
        cursor.execute("SELECT COUNT(*) FROM snapshots")
        snapshot_count = cursor.fetchone()[0]
        
        cursor.execute("SELECT COUNT(*) FROM processes")
        process_count = cursor.fetchone()[0]
        
        cursor.execute("SELECT COUNT(*) FROM app_groups")
        group_count = cursor.fetchone()[0]
        
        # Проверяем минимальные требования
        errors = []
        if snapshot_count < min_snapshots:
            errors.append(f"Недостаточно снапшотов: {snapshot_count} < {min_snapshots}")
        
        if process_count < min_processes:
            errors.append(f"Недостаточно процессов: {process_count} < {min_processes}")
        
        if group_count < min_groups:
            errors.append(f"Недостаточно групп: {group_count} < {min_groups}")
        
        if errors:
            raise ValueError("Датасет не проходит валидацию: " + "; ".join(errors))
        
        # Получаем дополнительную статистику
        cursor.execute("SELECT MIN(timestamp), MAX(timestamp) FROM snapshots")
        time_range = cursor.fetchone()
        
        cursor.execute("SELECT COUNT(DISTINCT pid) FROM processes")
        unique_processes = cursor.fetchone()[0]
        
        cursor.execute("SELECT COUNT(DISTINCT app_group_id) FROM app_groups")
        unique_groups = cursor.fetchone()[0]
        
        conn.close()
        
        return {
            "snapshot_count": snapshot_count,
            "process_count": process_count,
            "group_count": group_count,
            "unique_processes": unique_processes,
            "unique_groups": unique_groups,
            "time_range": {
                "start": time_range[0],
                "end": time_range[1]
            },
            "validation_passed": True
        }
        
    except Exception as e:
        if conn:
            conn.close()
        raise ValueError(f"Ошибка при валидации датасета: {e}") from e


def load_dataset(
    db_path: Path,
    validate: bool = True,
    min_snapshots: int = 1,
    min_processes: int = 10,
    min_groups: int = 1
) -> pd.DataFrame:
    """
    Загружает и валидирует датасет для обучения.
    
    Args:
        db_path: Путь к SQLite базе данных
        validate: Выполнять валидацию датасета
        min_snapshots: Минимальное количество снапшотов
        min_processes: Минимальное количество процессов
        min_groups: Минимальное количество групп
        
    Returns:
        DataFrame с данными для обучения
        
    Raises:
        ValueError: если датасет не проходит валидацию
        FileNotFoundError: если база данных не найдена
    """
    if validate:
        stats = validate_dataset(db_path, min_snapshots, min_processes, min_groups)
        print(f"Датасет валидирован: {stats['snapshot_count']} снапшотов, "
              f"{stats['process_count']} процессов, {stats['group_count']} групп")
    
    # Загружаем данные с использованием существующей функции
    return load_snapshots_as_frame(db_path)