"""Тесты для модуля training pipeline."""

import tempfile
from pathlib import Path

import pytest
from catboost import CatBoostRanker
from smoothtask_trainer.train_pipeline import (
    TrainingPipeline,
    train_from_database,
    train_from_snapshots,
)


def create_test_snapshot_file(snapshot_file: Path, num_snapshots: int = 3, processes_per_snapshot: int = 1) -> None:
    """Создает тестовый файл снапшотов."""
    import json
    from datetime import datetime, timezone
    
    snapshots = []
    
    for snapshot_idx in range(num_snapshots):
        snapshot_id = 1000 + snapshot_idx
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


def test_training_pipeline_from_snapshots():
    """Тест полного pipeline из снапшотов."""
    with tempfile.TemporaryDirectory() as tmpdir:
        snapshot_file = Path(tmpdir) / "test_snapshots.jsonl"
        model_json_path = Path(tmpdir) / "model.json"
        
        create_test_snapshot_file(snapshot_file, num_snapshots=3)
        
        # Создаем и выполняем pipeline
        pipeline = TrainingPipeline(
            snapshot_files=snapshot_file,
            use_temp_db=True,
            min_snapshots=1,
            min_processes=1,
            min_groups=1
        )
        
        # Выполняем полный pipeline
        model = pipeline.run_complete_pipeline(model_json_path)
        
        # Проверяем, что модель сохранена
        assert model_json_path.exists(), "Модель должна быть сохранена"
        
        # Проверяем, что модель можно загрузить
        assert isinstance(model, CatBoostRanker), "Должна быть возвращена модель CatBoostRanker"
        
        # Проверяем параметры модели
        assert model.get_params()["loss_function"] == "YetiRank"
        assert model.get_params()["depth"] == 6
        
        # Очистка
        pipeline.cleanup()


def test_training_pipeline_from_database():
    """Тест полного pipeline из существующей базы данных."""
    from smoothtask_trainer.collect_data import collect_data_from_snapshots
    
    with tempfile.TemporaryDirectory() as tmpdir:
        snapshot_file = Path(tmpdir) / "test_snapshots.jsonl"
        db_path = Path(tmpdir) / "test.db"
        model_json_path = Path(tmpdir) / "model.json"
        
        create_test_snapshot_file(snapshot_file, num_snapshots=3, processes_per_snapshot=4)
        
        # Создаем базу данных
        collect_data_from_snapshots(snapshot_file, output_db=db_path)
        
        # Создаем и выполняем pipeline
        pipeline = TrainingPipeline(
            db_path=db_path,
            use_temp_db=False,
            min_snapshots=1,
            min_processes=1,
            min_groups=1
        )
        
        # Выполняем полный pipeline
        model = pipeline.run_complete_pipeline(model_json_path)
        
        # Проверяем, что модель сохранена
        assert model_json_path.exists(), "Модель должна быть сохранена"
        
        # Проверяем, что модель можно загрузить
        assert isinstance(model, CatBoostRanker), "Должна быть возвращена модель CatBoostRanker"
        
        # Удаляем базу данных
        db_path.unlink()


def test_training_pipeline_with_onnx():
    """Тест pipeline с экспортом в ONNX."""
    with tempfile.TemporaryDirectory() as tmpdir:
        snapshot_file = Path(tmpdir) / "test_snapshots.jsonl"
        model_json_path = Path(tmpdir) / "model.json"
        model_onnx_path = Path(tmpdir) / "model.onnx"
        
        create_test_snapshot_file(snapshot_file, num_snapshots=3)
        
        # Создаем и выполняем pipeline
        pipeline = TrainingPipeline(
            snapshot_files=snapshot_file,
            use_temp_db=True,
            min_snapshots=1,
            min_processes=1,
            min_groups=1
        )
        
        # Выполняем полный pipeline с ONNX
        model = pipeline.run_complete_pipeline(model_json_path, model_onnx_path)
        
        # Проверяем, что обе модели сохранены
        assert model_json_path.exists(), "JSON модель должна быть сохранена"
        assert model_onnx_path.exists(), "ONNX модель должна быть сохранена"
        
        # Проверяем, что ONNX файл не пустой
        assert model_onnx_path.stat().st_size > 0, "ONNX файл не должен быть пустым"
        
        # Очистка
        pipeline.cleanup()


def test_training_pipeline_step_by_step():
    """Тест пошагового выполнения pipeline."""
    with tempfile.TemporaryDirectory() as tmpdir:
        snapshot_file = Path(tmpdir) / "test_snapshots.jsonl"
        model_json_path = Path(tmpdir) / "model.json"
        
        create_test_snapshot_file(snapshot_file, num_snapshots=3)
        
        # Создаем pipeline
        pipeline = TrainingPipeline(
            snapshot_files=snapshot_file,
            use_temp_db=True,
            min_snapshots=1,
            min_processes=1,
            min_groups=1
        )
        
        # Шаг 1: Сбор данных
        db_path = pipeline.collect_data()
        assert db_path.exists(), "База данных должна быть создана"
        
        # Шаг 2: Валидация данных
        stats = pipeline.validate_data()
        assert stats["snapshot_count"] == 3, "Ожидалось 3 снапшота"
        assert stats["validation_passed"] is True, "Валидация должна пройти"
        
        # Шаг 3: Загрузка данных
        df = pipeline.load_data()
        assert not df.empty, "DataFrame не должен быть пустым"
        
        # Шаг 4: Подготовка фич
        X, y, group_id, cat_features = pipeline.prepare_features(use_categorical=False)
        assert not X.empty, "Матрица фич не должна быть пустой"
        assert len(y) > 0, "Таргет не должен быть пустым"
        assert len(group_id) > 0, "Group ID не должен быть пустым"
        
        # Шаг 5: Обучение модели
        model = pipeline.train_model(model_json_path)
        assert model_json_path.exists(), "Модель должна быть сохранена"
        assert isinstance(model, CatBoostRanker), "Должна быть возвращена модель CatBoostRanker"
        
        # Очистка
        pipeline.cleanup()


def test_training_pipeline_error_handling():
    """Тест обработки ошибок в pipeline."""
    with tempfile.TemporaryDirectory() as tmpdir:
        # Проверяем обработку ошибки при отсутствии данных
        pipeline = TrainingPipeline(
            snapshot_files=None,
            db_path=None,
            use_temp_db=True
        )
        
        with pytest.raises(ValueError):
            pipeline.collect_data()
        
        # Проверяем обработку ошибки при попытке валидации без данных
        with pytest.raises(ValueError):
            pipeline.validate_data()
        
        # Проверяем обработку ошибки при попытке загрузки без данных
        with pytest.raises(ValueError):
            pipeline.load_data()


def test_train_from_snapshots_function():
    """Тест удобной функции train_from_snapshots."""
    with tempfile.TemporaryDirectory() as tmpdir:
        snapshot_file = Path(tmpdir) / "test_snapshots.jsonl"
        model_json_path = Path(tmpdir) / "model.json"
        
        create_test_snapshot_file(snapshot_file, num_snapshots=3)
        
        # Обучаем модель с подходящими минимальными требованиями
        model = train_from_snapshots(
            snapshot_file, 
            model_json_path,
            min_snapshots=1,
            min_processes=1,
            min_groups=1
        )
        
        # Проверяем, что модель сохранена
        assert model_json_path.exists(), "Модель должна быть сохранена"
        assert isinstance(model, CatBoostRanker), "Должна быть возвращена модель CatBoostRanker"


def test_train_from_database_function():
    """Тест удобной функции train_from_database."""
    from smoothtask_trainer.collect_data import collect_data_from_snapshots
    
    with tempfile.TemporaryDirectory() as tmpdir:
        snapshot_file = Path(tmpdir) / "test_snapshots.jsonl"
        db_path = Path(tmpdir) / "test.db"
        model_json_path = Path(tmpdir) / "model.json"
        
        create_test_snapshot_file(snapshot_file, num_snapshots=3, processes_per_snapshot=4)
        collect_data_from_snapshots(snapshot_file, output_db=db_path)
        
        # Обучаем модель
        model = train_from_database(db_path, model_json_path)
        
        # Проверяем, что модель сохранена
        assert model_json_path.exists(), "Модель должна быть сохранена"
        assert isinstance(model, CatBoostRanker), "Должна быть возвращена модель CatBoostRanker"
        
        # Удаляем базу данных
        db_path.unlink()


def test_training_pipeline_with_custom_parameters():
    """Тест pipeline с пользовательскими параметрами."""
    with tempfile.TemporaryDirectory() as tmpdir:
        snapshot_file = Path(tmpdir) / "test_snapshots.jsonl"
        model_json_path = Path(tmpdir) / "model.json"
        
        create_test_snapshot_file(snapshot_file, num_snapshots=3)
        
        # Создаем pipeline с пользовательскими параметрами
        pipeline = TrainingPipeline(
            snapshot_files=snapshot_file,
            use_temp_db=True,
            min_snapshots=2,
            min_processes=2,
            min_groups=2
        )
        
        # Выполняем pipeline
        model = pipeline.run_complete_pipeline(model_json_path)
        
        # Проверяем, что модель сохранена
        assert model_json_path.exists(), "Модель должна быть сохранена"
        
        # Очистка
        pipeline.cleanup()


def test_training_pipeline_export_model():
    """Тест экспорта модели через pipeline."""
    with tempfile.TemporaryDirectory() as tmpdir:
        snapshot_file = Path(tmpdir) / "test_snapshots.jsonl"
        model_json_path = Path(tmpdir) / "model.json"
        export_path = Path(tmpdir) / "exported_model.onnx"
        
        create_test_snapshot_file(snapshot_file, num_snapshots=3)
        
        # Создаем и выполняем pipeline
        pipeline = TrainingPipeline(
            snapshot_files=snapshot_file,
            use_temp_db=True,
            min_snapshots=1,
            min_processes=1,
            min_groups=1
        )
        
        # Выполняем pipeline
        pipeline.run_complete_pipeline(model_json_path)
        
        # Экспортируем модель
        pipeline.export_model(model_json_path, "onnx", export_path)
        
        # Проверяем, что модель экспортирована
        assert export_path.exists(), "Модель должна быть экспортирована"
        assert export_path.stat().st_size > 0, "Экспортированный файл не должен быть пустым"
        
        # Очистка
        pipeline.cleanup()