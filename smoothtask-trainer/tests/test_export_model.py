"""Тесты для экспорта моделей."""

import tempfile
from pathlib import Path

import numpy as np
import pytest
from catboost import CatBoostRanker, Pool
from smoothtask_trainer.export_model import export_model


def create_test_model(model_path: Path, format: str = "json"):
    """Создаёт тестовую модель и сохраняет её."""
    # Создаём простой датасет для обучения
    X = np.random.rand(100, 10)
    y = np.random.rand(100)
    group_id = np.repeat([0, 1, 2, 3, 4], 20)

    train_pool = Pool(
        data=X,
        label=y,
        group_id=group_id,
    )

    model = CatBoostRanker(
        loss_function="YetiRank",
        depth=3,
        learning_rate=0.1,
        iterations=10,
        random_state=42,
        verbose=False,
    )
    model.fit(train_pool)

    model.save_model(model_path.as_posix(), format=format)
    return model


def test_export_model_json_to_onnx():
    """Тест экспорта модели из JSON в ONNX."""
    with tempfile.TemporaryDirectory() as tmpdir:
        model_json_path = Path(tmpdir) / "model.json"
        model_onnx_path = Path(tmpdir) / "model.onnx"

        # Создаём тестовую модель в JSON
        create_test_model(model_json_path, format="json")

        # Экспортируем в ONNX
        export_model(model_json_path, "onnx", model_onnx_path)

        # Проверяем, что файл создан и не пустой
        assert model_onnx_path.exists(), "ONNX файл должен быть создан"
        assert model_onnx_path.stat().st_size > 0, "ONNX файл не должен быть пустым"

        # Проверяем, что модель можно загрузить (если есть onnxruntime)
        # Это опционально, так как onnxruntime может быть не установлен


def test_export_model_json_to_cbm():
    """Тест экспорта модели из JSON в CBM."""
    with tempfile.TemporaryDirectory() as tmpdir:
        model_json_path = Path(tmpdir) / "model.json"
        model_cbm_path = Path(tmpdir) / "model.cbm"

        # Создаём тестовую модель в JSON
        create_test_model(model_json_path, format="json")

        # Экспортируем в CBM
        export_model(model_json_path, "cbm", model_cbm_path)

        # Проверяем, что файл создан и не пустой
        assert model_cbm_path.exists(), "CBM файл должен быть создан"
        assert model_cbm_path.stat().st_size > 0, "CBM файл не должен быть пустым"

        # Проверяем, что модель можно загрузить
        model = CatBoostRanker()
        model.load_model(model_cbm_path.as_posix(), format="cbm")
        assert model is not None


def test_export_model_cbm_to_json():
    """Тест экспорта модели из CBM в JSON."""
    with tempfile.TemporaryDirectory() as tmpdir:
        model_cbm_path = Path(tmpdir) / "model.cbm"
        model_json_path = Path(tmpdir) / "model.json"

        # Создаём тестовую модель в CBM
        create_test_model(model_cbm_path, format="cbm")

        # Экспортируем в JSON
        export_model(model_cbm_path, "json", model_json_path)

        # Проверяем, что файл создан и не пустой
        assert model_json_path.exists(), "JSON файл должен быть создан"
        assert model_json_path.stat().st_size > 0, "JSON файл не должен быть пустым"

        # Проверяем, что модель можно загрузить
        model = CatBoostRanker()
        model.load_model(model_json_path.as_posix(), format="json")
        assert model is not None


def test_export_model_unsupported_format():
    """Тест обработки неподдерживаемого формата."""
    with tempfile.TemporaryDirectory() as tmpdir:
        model_json_path = Path(tmpdir) / "model.json"
        output_path = Path(tmpdir) / "model.xyz"

        create_test_model(model_json_path, format="json")

        with pytest.raises(ValueError, match="Неподдерживаемый формат"):
            export_model(model_json_path, "xyz", output_path)


def test_export_model_file_not_found():
    """Тест обработки отсутствующего файла модели."""
    with tempfile.TemporaryDirectory() as tmpdir:
        model_path = Path(tmpdir) / "nonexistent.json"
        output_path = Path(tmpdir) / "model.onnx"

        with pytest.raises(FileNotFoundError, match="Модель не найдена"):
            export_model(model_path, "onnx", output_path)


def test_export_model_case_insensitive_format():
    """Тест, что формат регистронезависимый."""
    with tempfile.TemporaryDirectory() as tmpdir:
        model_json_path = Path(tmpdir) / "model.json"
        model_onnx_path = Path(tmpdir) / "model.onnx"

        create_test_model(model_json_path, format="json")

        # Пробуем разные варианты регистра
        export_model(model_json_path, "ONNX", model_onnx_path)
        assert model_onnx_path.exists()

        model_onnx_path2 = Path(tmpdir) / "model2.onnx"
        export_model(model_json_path, "Onnx", model_onnx_path2)
        assert model_onnx_path2.exists()
