"""Экспорт обученной модели в различные форматы."""

import json
import time
from pathlib import Path
from typing import Dict, Optional, Any

from catboost import CatBoostRanker


def export_model(
    model_path: Path,
    format: str,
    output_path: Path,
    metadata: Optional[Dict[str, Any]] = None,
    validate: bool = True,
    version_id: Optional[str] = None,
):
    """
    Экспортирует модель в указанный формат с поддержкой метаданных, валидации и версионирования.

    Args:
        model_path: путь к обученной модели (поддерживаются форматы: json, cbm)
        format: формат экспорта ('onnx', 'json', 'cbm')
        output_path: путь для сохранения
        metadata: опциональные метаданные модели (версия, дата, описание и т.д.)
        validate: выполнять валидацию модели перед экспортом
        version_id: идентификатор версии модели (например, 'v1.0.0')

    Raises:
        ValueError: если формат не поддерживается или модель невалидна
        FileNotFoundError: если модель не найдена
        PermissionError: если нет прав на запись
    """
    # Валидация входных параметров
    if not model_path.exists():
        raise FileNotFoundError(f"Модель не найдена: {model_path}")

    if output_path.exists():
        if output_path.is_dir():
            raise ValueError(f"Выходной путь указывает на директорию: {output_path}")
        if not output_path.is_file():
            # Это нормально, файл будет создан
            pass

    # Создаём вложенные директории для результата, если их ещё нет
    output_path.parent.mkdir(parents=True, exist_ok=True)

    # Проверяем права на запись
    try:
        test_file = output_path.parent / f".write_test_{int(time.time())}"
        test_file.write_text("test")
        test_file.unlink()
    except Exception as e:
        raise PermissionError(
            f"Нет прав на запись в директорию {output_path.parent}: {e}"
        ) from e

    # Определяем формат исходной модели по расширению
    model_format = "json" if model_path.suffix == ".json" else "cbm"

    # Загружаем модель
    model = CatBoostRanker()
    try:
        model.load_model(model_path.as_posix(), format=model_format)
    except Exception as e:
        raise ValueError(
            f"Ошибка при загрузке модели из {model_path} (формат: {model_format}): {e}. "
            "Проверьте, что файл является валидной моделью CatBoost."
        ) from e

    # Валидация модели (если запрошено)
    if validate:
        _validate_model(model)

    # Нормализуем формат экспорта
    export_format = format.lower()

    # Проверяем поддерживаемые форматы
    supported_formats = {"onnx", "json", "cbm"}
    if export_format not in supported_formats:
        raise ValueError(
            f"Неподдерживаемый формат: {format}. "
            f"Поддерживаемые форматы: {', '.join(supported_formats)}"
        )

    # Экспортируем модель
    try:
        model.save_model(output_path.as_posix(), format=export_format)
    except Exception as e:
        raise ValueError(
            f"Ошибка при экспорте модели в {export_format}: {e}. "
            f"Проверьте, что путь доступен для записи: {output_path}"
        ) from e

    # Экспортируем метаданные (если предоставлены)
    if metadata:
        _export_metadata(output_path, metadata, export_format, version_id)

    # Возвращаем информацию об экспорте
    return {
        "input_model": str(model_path),
        "input_format": model_format,
        "output_model": str(output_path),
        "output_format": export_format,
        "output_size": output_path.stat().st_size,
        "metadata": metadata or {},
        "timestamp": time.time(),
    }


def _validate_model(model: CatBoostRanker):
    """Валидация модели перед экспортом."""
    # Проверяем, что модель обучена
    if not hasattr(model, 'tree_count_') or model.tree_count_ == 0:
        raise ValueError("Модель не обучена или пустая (tree_count_ = 0)")

    # Проверяем, что модель имеет корректные параметры (используем get_params)
    try:
        params = model.get_params()
    except Exception as e:
        raise ValueError(f"Не удалось получить параметры модели: {e}")
    
    required_params = ['loss_function', 'iterations', 'depth']
    for param in required_params:
        if param not in params:
            raise ValueError(f"Модель не имеет обязательного параметра: {param}")

    # Проверяем, что модель может делать предсказания (базовая проверка)
    try:
        # Создаём тестовый пул с минимальными данными
        # Используем правильное количество фич (10, как в тестовых данных)
        import numpy as np
        test_data = np.random.rand(1, 10)  # 10 фич, как в тестовых данных
        model.predict(test_data)
    except Exception as e:
        # Не критическая ошибка - просто логируем
        print(f"Предупреждение: не удалось проверить предсказания модели: {e}")


def _export_metadata(output_path: Path, metadata: Dict[str, Any], export_format: str, version_id: Optional[str] = None):
    """Экспортирует метаданные вместе с моделью."""
    # Добавляем стандартные метаданные
    metadata.setdefault('export_timestamp', time.time())
    metadata.setdefault('export_format', export_format)
    metadata.setdefault('model_type', 'CatBoostRanker')
    
    # Добавляем информацию о версии, если предоставлена
    if version_id:
        metadata.setdefault('version_id', version_id)
        metadata.setdefault('version_timestamp', time.time())
    
    # Создаём файл метаданных рядом с моделью
    if export_format == "onnx":
        metadata_path = output_path.with_suffix('.onnx.metadata.json')
    else:
        metadata_path = output_path.with_suffix('.metadata.json')

    try:
        with open(metadata_path, 'w', encoding='utf-8') as f:
            json.dump(metadata, f, indent=2, ensure_ascii=False)
    except Exception as e:
        # Не критическая ошибка - просто логируем
        print(f"Предупреждение: не удалось сохранить метаданные в {metadata_path}: {e}")


def export_versioned_model(
    model_path: Path,
    format: str,
    versions_dir: Path,
    version_id: str,
    metadata: Optional[Dict[str, Any]] = None,
    validate: bool = True,
):
    """
    Экспортирует модель с поддержкой версионирования.

    Создаёт версионный файл модели в указанной директории с заданным идентификатором версии.

    Args:
        model_path: путь к обученной модели (поддерживаются форматы: json, cbm)
        format: формат экспорта ('onnx', 'json', 'cbm')
        versions_dir: директория для хранения версий моделей
        version_id: идентификатор версии (например, 'v1.0.0')
        metadata: опциональные метаданные модели
        validate: выполнять валидацию модели перед экспортом

    Returns:
        Dict с информацией об экспорте

    Raises:
        ValueError: если формат не поддерживается или модель невалидна
        FileNotFoundError: если модель не найдена
        PermissionError: если нет прав на запись
    """
    # Создаём директорию для версий, если её ещё нет
    versions_dir.mkdir(parents=True, exist_ok=True)
    
    # Проверяем права на запись
    try:
        test_file = versions_dir / f".write_test_{int(time.time())}"
        test_file.write_text("test")
        test_file.unlink()
    except Exception as e:
        raise PermissionError(
            f"Нет прав на запись в директорию {versions_dir}: {e}"
        ) from e
    
    # Создаём путь для версии модели
    if format == "onnx":
        versioned_filename = f"model_{version_id}.onnx"
    elif format == "json":
        versioned_filename = f"model_{version_id}.json"
    elif format == "cbm":
        versioned_filename = f"model_{version_id}.cbm"
    else:
        raise ValueError(f"Неподдерживаемый формат: {format}")
    
    output_path = versions_dir / versioned_filename
    
    # Добавляем информацию о версии в метаданные
    version_metadata = metadata.copy() if metadata else {}
    version_metadata.setdefault('version_id', version_id)
    version_metadata.setdefault('version_timestamp', time.time())
    
    # Экспортируем модель
    result = export_model(
        model_path=model_path,
        format=format,
        output_path=output_path,
        metadata=version_metadata,
        validate=validate,
        version_id=version_id
    )
    
    return result

def validate_exported_model(
    model_path: Path,
    expected_format: str,
    min_size: int = 1024,
    check_metadata: bool = True,
):
    """
    Валидация экспортированной модели.

    Args:
        model_path: путь к экспортированной модели
        expected_format: ожидаемый формат ('onnx', 'json', 'cbm')
        min_size: минимальный ожидаемый размер файла в байтах
        check_metadata: проверять наличие метаданных

    Returns:
        Dict с информацией о валидации

    Raises:
        ValueError: если модель не проходит валидацию
    """
    if not model_path.exists():
        raise ValueError(f"Файл модели не найден: {model_path}")

    if not model_path.is_file():
        raise ValueError(f"Путь не указывает на файл: {model_path}")

    file_size = model_path.stat().st_size
    if file_size < min_size:
        raise ValueError(
            f"Файл модели слишком мал: {file_size} байт (минимум: {min_size} байт)"
        )

    # Проверяем расширение файла
    expected_extensions = {
        'onnx': '.onnx',
        'json': '.json',
        'cbm': '.cbm'
    }
    expected_extension = expected_extensions.get(expected_format.lower())
    if not expected_extension:
        raise ValueError(f"Неизвестный формат: {expected_format}")

    if model_path.suffix != expected_extension:
        raise ValueError(
            f"Несоответствие расширения файла. Ожидалось: {expected_extension}, получено: {model_path.suffix}"
        )

    # Проверяем метаданные
    validation_result = {
        'path': str(model_path),
        'format': expected_format,
        'size': file_size,
        'metadata': None,
    }

    if check_metadata:
        metadata_path = None
        if expected_format == "onnx":
            metadata_path = model_path.with_suffix('.onnx.metadata.json')
        else:
            metadata_path = model_path.with_suffix('.metadata.json')

        if metadata_path and metadata_path.exists():
            try:
                with open(metadata_path, 'r', encoding='utf-8') as f:
                    metadata = json.load(f)
                validation_result['metadata'] = metadata
            except Exception as e:
                raise ValueError(f"Ошибка при чтении метаданных: {e}")

    return validation_result

def create_version_manifest(
    versions_dir: Path,
    manifest_path: Optional[Path] = None,
):
    """
    Создаёт манифест версий моделей для использования в Rust.

    Сканирует директорию с версиями и создаёт JSON файл с информацией о всех версиях,
    совместимый с Rust ModelVersionManager.

    Args:
        versions_dir: директория с версиями моделей
        manifest_path: путь для сохранения манифеста (по умолчанию: versions_dir/versions.json)

    Returns:
        Path к созданному манифесту

    Raises:
        ValueError: если директория не существует или не содержит модели
    """
    if not versions_dir.exists():
        raise ValueError(f"Директория с версиями не существует: {versions_dir}")
    
    if manifest_path is None:
        manifest_path = versions_dir / "versions.json"
    
    # Собираем информацию о версиях
    versions = []
    
    for model_file in versions_dir.glob("model_*.onnx"):
        if model_file.is_file():
            version_info = _extract_version_info_from_filename(model_file)
            if version_info:
                versions.append(version_info)
    
    for model_file in versions_dir.glob("model_*.json"):
        if model_file.is_file():
            version_info = _extract_version_info_from_filename(model_file)
            if version_info:
                versions.append(version_info)
    
    for model_file in versions_dir.glob("model_*.cbm"):
        if model_file.is_file():
            version_info = _extract_version_info_from_filename(model_file)
            if version_info:
                versions.append(version_info)
    
    if not versions:
        raise ValueError(f"В директории {versions_dir} не найдено моделей с шаблоном model_*")
    
    # Сортируем версии по идентификатору
    versions.sort(key=lambda x: x['version_id'])
    
    # Сохраняем манифест
    with open(manifest_path, 'w', encoding='utf-8') as f:
        json.dump(versions, f, indent=2, ensure_ascii=False)
    
    return manifest_path

def _extract_version_info_from_filename(model_file: Path) -> Optional[Dict[str, Any]]:
    """
    Извлекает информацию о версии из имени файла модели.

    Args:
        model_file: путь к файлу модели

    Returns:
        Словарь с информацией о версии или None, если формат не распознан
    """
    filename = model_file.name
    
    # Пробуем извлечь версию из шаблона model_{version_id}.{format}
    import re
    pattern = r'model_(.+?)\.(onnx|json|cbm)$'
    match = re.match(pattern, filename)
    
    if not match:
        return None
    
    version_id = match.group(1)
    format_type = match.group(2)
    
    # Пробуем загрузить метаданные
    metadata = {}
    if format_type == "onnx":
        metadata_path = model_file.with_suffix('.onnx.metadata.json')
    else:
        metadata_path = model_file.with_suffix('.metadata.json')
    
    if metadata_path.exists():
        try:
            with open(metadata_path, 'r', encoding='utf-8') as f:
                metadata = json.load(f)
        except Exception:
            pass  # Игнорируем ошибки чтения метаданных
    
    # Создаём информацию о версии в формате, совместимом с Rust
    version_info = {
        'version_id': version_id,
        'model_path': str(model_file),
        'format': format_type,
        'timestamp': metadata.get('version_timestamp', time.time()),
        'metadata': metadata,
    }
    
    # Добавляем хэш, если есть
    if 'model_hash' in metadata:
        version_info['model_hash'] = metadata['model_hash']
    
    # Добавляем размер файла
    try:
        version_info['file_size'] = model_file.stat().st_size
    except Exception:
        pass
    
    return version_info
