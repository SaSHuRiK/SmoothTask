"""Экспорт обученной модели в различные форматы."""

from pathlib import Path


def export_model(model_path: Path, format: str, output_path: Path):
    """
    Экспортирует модель в указанный формат.
    
    Args:
        model_path: путь к обученной модели
        format: формат экспорта ('onnx', 'json', 'cbm')
        output_path: путь для сохранения
    """
    # TODO: реализовать экспорт модели
    raise NotImplementedError("TODO: реализовать экспорт модели")


