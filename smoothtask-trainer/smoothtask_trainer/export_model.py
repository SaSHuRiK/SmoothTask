"""Экспорт обученной модели в различные форматы."""

from pathlib import Path

from catboost import CatBoostRanker


def export_model(model_path: Path, format: str, output_path: Path):
    """
    Экспортирует модель в указанный формат.

    Args:
        model_path: путь к обученной модели (поддерживаются форматы: json, cbm)
        format: формат экспорта ('onnx', 'json', 'cbm')
        output_path: путь для сохранения

    Raises:
        ValueError: если формат не поддерживается
        FileNotFoundError: если модель не найдена
    """
    if not model_path.exists():
        raise FileNotFoundError(f"Модель не найдена: {model_path}")

    if output_path.exists() and output_path.is_dir():
        raise ValueError(f"Выходной путь указывает на директорию: {output_path}")

    # Создаём вложенные директории для результата, если их ещё нет.
    output_path.parent.mkdir(parents=True, exist_ok=True)

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
