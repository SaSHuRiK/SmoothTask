"""SmoothTask Trainer - инструменты для обучения CatBoostRanker."""

__version__ = "0.0.1"

# Импортируем основные функции для удобного доступа
from .collect_data import (
    collect_data_from_snapshots,
    load_dataset,
    validate_dataset,
)
from .export_model import export_model
from .features import build_feature_matrix
from .train_pipeline import (
    TrainingPipeline,
    train_from_snapshots,
    train_from_database,
)
from .train_ranker import train_ranker

__all__ = [
    "collect_data_from_snapshots",
    "load_dataset",
    "validate_dataset",
    "export_model",
    "build_feature_matrix",
    "TrainingPipeline",
    "train_from_snapshots",
    "train_from_database",
    "train_ranker",
]
