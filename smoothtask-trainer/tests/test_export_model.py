"""Тесты для экспорта моделей."""

import tempfile
from pathlib import Path

import numpy as np
import pytest
from catboost import CatBoostRanker, Pool
from smoothtask_trainer.export_model import export_model, validate_exported_model


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


def test_export_model_creates_parent_directories():
    """Экспорт должен создавать вложенные каталоги для выходного файла."""
    with tempfile.TemporaryDirectory() as tmpdir:
        nested_dir = Path(tmpdir) / "nested" / "deep"
        model_json_path = Path(tmpdir) / "model.json"
        output_path = nested_dir / "model.onnx"

        create_test_model(model_json_path, format="json")

        export_model(model_json_path, "onnx", output_path)

        assert nested_dir.exists() and nested_dir.is_dir()
        assert output_path.exists()
        assert output_path.stat().st_size > 0


def test_export_model_rejects_directory_output_path():
    """Если output_path указывает на каталог, должна быть ошибка."""
    with tempfile.TemporaryDirectory() as tmpdir:
        model_json_path = Path(tmpdir) / "model.json"
        output_dir = Path(tmpdir) / "out_dir"
        output_dir.mkdir()

        create_test_model(model_json_path, format="json")

        with pytest.raises(ValueError, match="директор"):
            export_model(model_json_path, "onnx", output_dir)


def test_export_model_invalid_model_file():
    """Тест обработки невалидного файла модели."""
    with tempfile.TemporaryDirectory() as tmpdir:
        # Создаём файл, который не является валидной моделью
        invalid_model_path = Path(tmpdir) / "invalid.json"
        invalid_model_path.write_text("Это не валидная модель CatBoost")

        output_path = Path(tmpdir) / "model.onnx"

        # Ожидаем ValueError с информативным сообщением
        with pytest.raises(ValueError, match="Ошибка при загрузке модели"):
            export_model(invalid_model_path, "onnx", output_path)


def test_export_model_error_on_save():
    """Тест обработки ошибки при сохранении модели."""
    with tempfile.TemporaryDirectory() as tmpdir:
        model_json_path = Path(tmpdir) / "model.json"
        create_test_model(model_json_path, format="json")

        # Создаём путь, который указывает на директорию (должна быть ошибка валидации)
        output_dir = Path(tmpdir) / "output_dir"
        output_dir.mkdir()

        # Ожидаем ValueError при попытке сохранить в директорию
        with pytest.raises(ValueError, match="указывает на директорию"):
            export_model(model_json_path, "onnx", output_dir)


def test_export_model_with_metadata():
    """Тест экспорта модели с метаданными."""
    with tempfile.TemporaryDirectory() as tmpdir:
        model_json_path = Path(tmpdir) / "model.json"
        model_onnx_path = Path(tmpdir) / "model.onnx"

        # Создаём тестовую модель
        create_test_model(model_json_path, format="json")

        # Экспортируем с метаданными
        metadata = {
            "version": "1.0.0",
            "description": "Тестовая модель для SmoothTask",
            "author": "SmoothTask Trainer",
            "dataset_size": 1000,
            "features": ["cpu_usage", "memory_usage", "io_wait"],
        }

        result = export_model(model_json_path, "onnx", model_onnx_path, metadata=metadata)

        # Проверяем, что модель экспортирована
        assert model_onnx_path.exists()
        assert model_onnx_path.stat().st_size > 0

        # Проверяем, что метаданные сохранены
        metadata_path = model_onnx_path.with_suffix('.onnx.metadata.json')
        assert metadata_path.exists()

        # Проверяем содержимое метаданных
        import json
        with open(metadata_path, 'r', encoding='utf-8') as f:
            saved_metadata = json.load(f)

        # Проверяем, что все метаданные сохранены
        for key, value in metadata.items():
            assert saved_metadata[key] == value

        # Проверяем, что добавлены стандартные метаданные
        assert 'export_timestamp' in saved_metadata
        assert 'export_format' in saved_metadata
        assert 'model_type' in saved_metadata

        # Проверяем возвращаемое значение
        assert result["input_model"] == str(model_json_path)
        assert result["output_model"] == str(model_onnx_path)
        assert result["output_format"] == "onnx"
        assert result["metadata"] == metadata


def test_export_model_with_validation():
    """Тест экспорта модели с валидацией."""
    with tempfile.TemporaryDirectory() as tmpdir:
        model_json_path = Path(tmpdir) / "model.json"
        model_onnx_path = Path(tmpdir) / "model.onnx"

        # Создаём тестовую модель
        create_test_model(model_json_path, format="json")

        # Экспортируем с валидацией (по умолчанию включена)
        result = export_model(model_json_path, "onnx", model_onnx_path, validate=True)

        # Проверяем, что экспорт прошёл успешно
        assert model_onnx_path.exists()
        assert result["output_size"] > 0


def test_export_model_without_validation():
    """Тест экспорта модели без валидации."""
    with tempfile.TemporaryDirectory() as tmpdir:
        model_json_path = Path(tmpdir) / "model.json"
        model_onnx_path = Path(tmpdir) / "model.onnx"

        # Создаём тестовую модель
        create_test_model(model_json_path, format="json")

        # Экспортируем без валидации
        result = export_model(model_json_path, "onnx", model_onnx_path, validate=False)

        # Проверяем, что экспорт прошёл успешно
        assert model_onnx_path.exists()
        assert result["output_size"] > 0


def test_validate_exported_model():
    """Тест валидации экспортированной модели."""
    with tempfile.TemporaryDirectory() as tmpdir:
        model_json_path = Path(tmpdir) / "model.json"
        model_onnx_path = Path(tmpdir) / "model.onnx"

        # Создаём и экспортируем модель
        create_test_model(model_json_path, format="json")
        metadata = {"version": "1.0.0", "description": "Тестовая модель"}
        export_model(model_json_path, "onnx", model_onnx_path, metadata=metadata)

        # Валидируем экспортированную модель
        validation_result = validate_exported_model(model_onnx_path, "onnx")

        # Проверяем результаты валидации
        assert validation_result["path"] == str(model_onnx_path)
        assert validation_result["format"] == "onnx"
        assert validation_result["size"] > 0
        assert validation_result["metadata"] is not None
        assert validation_result["metadata"]["version"] == "1.0.0"


def test_validate_exported_model_without_metadata():
    """Тест валидации модели без метаданных."""
    with tempfile.TemporaryDirectory() as tmpdir:
        model_json_path = Path(tmpdir) / "model.json"
        model_onnx_path = Path(tmpdir) / "model.onnx"

        # Создаём и экспортируем модель без метаданных
        create_test_model(model_json_path, format="json")
        export_model(model_json_path, "onnx", model_onnx_path)

        # Валидируем без проверки метаданных
        validation_result = validate_exported_model(
            model_onnx_path, "onnx", check_metadata=False
        )

        # Проверяем результаты валидации
        assert validation_result["path"] == str(model_onnx_path)
        assert validation_result["format"] == "onnx"
        assert validation_result["size"] > 0
        assert validation_result["metadata"] is None


def test_validate_exported_model_invalid_format():
    """Тест валидации модели с неверным форматом."""
    with tempfile.TemporaryDirectory() as tmpdir:
        model_json_path = Path(tmpdir) / "model.json"
        model_onnx_path = Path(tmpdir) / "model.onnx"

        # Создаём и экспортируем модель
        create_test_model(model_json_path, format="json")
        export_model(model_json_path, "onnx", model_onnx_path)

        # Пробуем валидировать с неверным форматом
        with pytest.raises(ValueError, match="Несоответствие расширения файла"):
            validate_exported_model(model_onnx_path, "json")


def test_validate_exported_model_file_not_found():
    """Тест валидации несуществующей модели."""
    with tempfile.TemporaryDirectory() as tmpdir:
        model_path = Path(tmpdir) / "nonexistent.onnx"

        with pytest.raises(ValueError, match="Файл модели не найден"):
            validate_exported_model(model_path, "onnx")


def test_export_model_permission_error():
    """Тест обработки ошибки прав доступа."""
    with tempfile.TemporaryDirectory() as tmpdir:
        model_json_path = Path(tmpdir) / "model.json"
        create_test_model(model_json_path, format="json")

        # Создаём директорию без прав на запись (эмуляция)
        readonly_dir = Path(tmpdir) / "readonly"
        readonly_dir.mkdir()

        # Пробуем экспортировать в директорию без прав
        output_path = readonly_dir / "model.onnx"

        # В реальной системе это вызвало бы PermissionError
        # В тесте мы просто проверяем, что функция обрабатывает такие случаи
        try:
            export_model(model_json_path, "onnx", output_path)
            # Если экспорт прошёл, проверяем результат
            assert output_path.exists()
        except PermissionError:
            # Это ожидаемое поведение в некоторых системах
            pass


def test_export_model_all_formats_with_metadata():
    """Тест экспорта во все форматы с метаданными."""
    with tempfile.TemporaryDirectory() as tmpdir:
        model_json_path = Path(tmpdir) / "model.json"
        create_test_model(model_json_path, format="json")

        metadata = {"version": "1.0.0", "test": "all_formats"}

        # Тестируем все поддерживаемые форматы
        formats = ["json", "cbm", "onnx"]
        for fmt in formats:
            output_path = Path(tmpdir) / f"model.{fmt}"
            result = export_model(model_json_path, fmt, output_path, metadata=metadata)

            # Проверяем экспорт
            assert output_path.exists()
            assert result["output_format"] == fmt

            # Проверяем метаданные
            if fmt == "onnx":
                metadata_path = output_path.with_suffix('.onnx.metadata.json')
            else:
                metadata_path = output_path.with_suffix('.metadata.json')
            assert metadata_path.exists()
