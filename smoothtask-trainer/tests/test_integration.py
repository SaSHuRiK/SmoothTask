"""Интеграционные тесты для полного pipeline от сбора данных до обучения."""

import tempfile
from pathlib import Path

import pytest
from smoothtask_trainer import (
    TrainingPipeline,
    train_from_snapshots,
    train_from_database,
    collect_data_from_snapshots,
    validate_dataset,
    load_dataset,
)
from tests.test_train_ranker import create_training_db


def test_full_pipeline_from_snapshots():
    """Тест полного pipeline: снапшоты -> база данных -> обучение -> экспорт."""
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir_path = Path(tmpdir)
        
        # Создаём тестовую базу данных
        db_path = tmpdir_path / "test.db"
        create_training_db(db_path, num_snapshots=2)
        
        # Создаём pipeline с существующей базой данных
        pipeline = TrainingPipeline(
            db_path=db_path,
            min_snapshots=1,
            min_processes=1,
            min_groups=1,
        )
        
        # Выполняем полный pipeline
        model = pipeline.run_complete_pipeline(
            model_path=tmpdir_path / "model.json",
            onnx_path=tmpdir_path / "model.onnx"
        )
        
        # Проверяем результаты
        assert model is not None
        assert (tmpdir_path / "model.json").exists()
        assert (tmpdir_path / "model.json").stat().st_size > 0
        assert (tmpdir_path / "model.onnx").exists()
        assert (tmpdir_path / "model.onnx").stat().st_size > 0


def test_full_pipeline_with_validation_errors():
    """Тест pipeline с ошибками валидации."""
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir_path = Path(tmpdir)
        
        # Создаём слишком мало снапшотов
        db_path = tmpdir_path / "test.db"
        create_training_db(db_path, num_snapshots=2)
        
        # Пробуем выполнить pipeline с высокими требованиями
        pipeline = TrainingPipeline(
            db_path=db_path,
            min_snapshots=10,  # Требуем больше, чем есть (2 снапшота)
            min_processes=5,   # Требуем больше, чем есть (2 процесса)
            min_groups=3,     # Требуем больше, чем есть (2 группы)
        )
        
        # Ожидаем ошибку валидации
        with pytest.raises(ValueError, match="Ошибка при валидации датасета"):
            pipeline.run_complete_pipeline(
                model_path=tmpdir_path / "model.json",
                onnx_path=tmpdir_path / "model.onnx"
            )


def test_integration_with_existing_database():
    """Тест интеграции с существующей базой данных."""
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir_path = Path(tmpdir)
        
        # Создаём тестовую базу данных
        db_path = tmpdir_path / "test.db"
        create_training_db(db_path, num_snapshots=2)
        
        # Теперь используем базу данных для обучения
        pipeline = TrainingPipeline(
            db_path=db_path,
            min_snapshots=1,
            min_processes=1,
            min_groups=1,
        )
        
        # Выполняем pipeline из существующей базы данных
        model = pipeline.run_complete_pipeline(
            model_path=tmpdir_path / "model.json",
            onnx_path=tmpdir_path / "model.onnx"
        )
        
        # Проверяем результаты
        assert model is not None
        assert (tmpdir_path / "model.json").exists()
        assert (tmpdir_path / "model.onnx").exists()


def test_integration_with_validation_functions():
    """Тест интеграции с функциями валидации."""
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir_path = Path(tmpdir)
        
        # Создаём тестовую базу данных
        db_path = tmpdir_path / "test.db"
        create_training_db(db_path, num_snapshots=2)
        
        # Валидируем данные
        stats = validate_dataset(
            db_path=db_path,
            min_snapshots=1,
            min_processes=1,
            min_groups=1
        )
        
        # Проверяем статистику
        assert stats["snapshot_count"] >= 1
        assert stats["process_count"] >= 1
        assert stats["group_count"] >= 1
        
        # Загружаем данные
        df = load_dataset(db_path, validate=True, min_snapshots=1, min_processes=1, min_groups=1)
        assert df is not None
        assert len(df) > 0
        
        # Теперь обучаем модель с использованием удобной функции
        train_from_database(
            db_path=db_path,
            model_path=tmpdir_path / "model.json",
            onnx_path=tmpdir_path / "model.onnx",
            min_snapshots=1,
            min_processes=1,
            min_groups=1
        )
        
        # Проверяем результаты
        assert (tmpdir_path / "model.json").exists()
        assert (tmpdir_path / "model.onnx").exists()


def test_integration_with_snapshots_function():
    """Тест интеграции с функцией обучения из снапшотов."""
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir_path = Path(tmpdir)
        
        # Создаём тестовую базу данных
        db_path = tmpdir_path / "test.db"
        create_training_db(db_path, num_snapshots=2)
        
        # Обучаем модель напрямую из базы данных
        train_from_database(
            db_path=db_path,
            model_path=tmpdir_path / "model.json",
            onnx_path=tmpdir_path / "model.onnx",
            min_snapshots=1,
            min_processes=1,
            min_groups=1
        )
        
        # Проверяем результаты
        assert (tmpdir_path / "model.json").exists()
        assert (tmpdir_path / "model.onnx").exists()


def test_integration_error_handling():
    """Тест обработки ошибок в интеграционных сценариях."""
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir_path = Path(tmpdir)
        
        # Тест 1: Несуществующий файл базы данных
        with pytest.raises(FileNotFoundError):
            train_from_database(
                db_path=tmpdir_path / "nonexistent.db",
                model_path=tmpdir_path / "model.json"
            )
        
        # Тест 2: Невалидные данные
        db_path = tmpdir_path / "test.db"
        create_training_db(db_path, num_snapshots=2)  # Слишком мало
        
        with pytest.raises(ValueError, match="Ошибка при валидации датасета"):
            train_from_database(
                db_path=db_path,
                model_path=tmpdir_path / "model.json",
                min_snapshots=10  # Требуем больше, чем есть
            )


def test_integration_performance():
    """Тест производительности интеграционного pipeline."""
    import time
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir_path = Path(tmpdir)
        
        # Создаём тестовую базу данных (ограничиваем до 10 снапшотов из-за ограничения на секунды)
        db_path = tmpdir_path / "test.db"
        create_training_db(db_path, num_snapshots=2)
        
        # Замеряем время выполнения
        start_time = time.time()
        
        # Выполняем полный pipeline
        pipeline = TrainingPipeline(
            db_path=db_path,
            min_snapshots=1,
            min_processes=1,
            min_groups=1,
        )
        
        model = pipeline.run_complete_pipeline(
            model_path=tmpdir_path / "model.json",
            onnx_path=tmpdir_path / "model.onnx"
        )
        
        end_time = time.time()
        execution_time = end_time - start_time
        
        # Проверяем, что pipeline выполнился за разумное время
        assert execution_time < 60  # Должно выполняться менее чем за 60 секунд
        assert model is not None
        assert (tmpdir_path / "model.json").exists()
        assert (tmpdir_path / "model.onnx").exists()