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

    # Определяем формат исходной модели по расширению
    model_format = "json" if model_path.suffix == ".json" else "cbm"

    # Загружаем модель
    model = CatBoostRanker()
    model.load_model(model_path.as_posix(), format=model_format)

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
    model.save_model(output_path.as_posix(), format=export_format)
