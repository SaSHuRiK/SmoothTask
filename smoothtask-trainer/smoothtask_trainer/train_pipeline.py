"""Комплексный pipeline для сбора данных и обучения модели."""

from __future__ import annotations

import tempfile
from pathlib import Path
from typing import Optional, Union

import pandas as pd
from catboost import CatBoostRanker

from .collect_data import collect_data_from_snapshots, load_dataset, validate_dataset
from .export_model import export_model
from .features import build_feature_matrix
from .train_ranker import train_ranker


class TrainingPipeline:
    """
    Комплексный pipeline для сбора данных и обучения модели.
    
    Pipeline включает:
    1. Сбор данных из снапшотов
    2. Валидацию датасета
    3. Подготовку фич
    4. Обучение модели
    5. Экспорт модели в различные форматы
    """
    
    def __init__(
        self,
        snapshot_files: Optional[Union[Path, list[Path]]] = None,
        db_path: Optional[Path] = None,
        use_temp_db: bool = True,
        min_snapshots: int = 1,
        min_processes: int = 10,
        min_groups: int = 1
    ):
        """
        Инициализирует pipeline.
        
        Args:
            snapshot_files: Путь к файлу снапшота или список путей
            db_path: Путь к существующей базе данных (альтернатива snapshot_files)
            use_temp_db: Использовать временную базу данных
            min_snapshots: Минимальное количество снапшотов для валидации
            min_processes: Минимальное количество процессов для валидации
            min_groups: Минимальное количество групп для валидации
        """
        self.snapshot_files = snapshot_files
        self.db_path = db_path
        self.use_temp_db = use_temp_db
        self.min_snapshots = min_snapshots
        self.min_processes = min_processes
        self.min_groups = min_groups
        self._db_path: Optional[Path] = None
        self._dataset: Optional[pd.DataFrame] = None
        self._model: Optional[CatBoostRanker] = None
    
    def collect_data(self) -> Path:
        """
        Собирает данные из снапшотов или использует существующую базу данных.
        
        Returns:
            Путь к базе данных
            
        Raises:
            ValueError: если не удалось собрать данные
        """
        if self.db_path:
            # Используем существующую базу данных
            if not self.db_path.exists():
                raise FileNotFoundError(f"База данных не найдена: {self.db_path}")
            self._db_path = self.db_path
        elif self.snapshot_files:
            # Собираем данные из снапшотов
            self._db_path = collect_data_from_snapshots(
                self.snapshot_files, 
                use_temp_db=self.use_temp_db
            )
        else:
            raise ValueError("Не указаны ни snapshot_files, ни db_path")
        
        return self._db_path
    
    def validate_data(self) -> dict:
        """
        Валидирует собранные данные.
        
        Returns:
            Словарь со статистикой датасета
            
        Raises:
            ValueError: если датасет не проходит валидацию
        """
        if not self._db_path:
            raise ValueError("Сначала соберите данные с помощью collect_data()")
        
        stats = validate_dataset(
            self._db_path,
            min_snapshots=self.min_snapshots,
            min_processes=self.min_processes,
            min_groups=self.min_groups
        )
        
        return stats
    
    def load_data(self) -> pd.DataFrame:
        """
        Загружает данные для обучения.
        
        Returns:
            DataFrame с данными для обучения
            
        Raises:
            ValueError: если данные не собраны или не валидированы
        """
        if not self._db_path:
            raise ValueError("Сначала соберите данные с помощью collect_data()")
        
        self._dataset = load_dataset(
            self._db_path,
            validate=False  # Валидация уже выполнена в validate_data()
        )
        
        return self._dataset
    
    def prepare_features(self, use_categorical: bool = True) -> tuple:
        """
        Подготавливает матрицу фич для обучения.
        
        Args:
            use_categorical: Использовать категориальные фичи
            
        Returns:
            Кортеж с (X, y, group_id, cat_features)
            
        Raises:
            ValueError: если данные не загружены
        """
        if self._dataset is None:
            raise ValueError("Сначала загрузите данные с помощью load_data()")
        
        return build_feature_matrix(self._dataset, use_categorical=use_categorical)
    
    def train_model(
        self,
        model_path: Path,
        onnx_path: Optional[Path] = None,
        use_categorical: bool = True,
        **train_params
    ) -> CatBoostRanker:
        """
        Обучает модель CatBoostRanker.
        
        Args:
            model_path: Путь для сохранения модели в формате JSON
            onnx_path: Опциональный путь для сохранения модели в формате ONNX
            use_categorical: Использовать категориальные фичи
            **train_params: Дополнительные параметры для обучения
            
        Returns:
            Обученная модель CatBoostRanker
            
        Raises:
            ValueError: если не удалось обучить модель
        """
        if self._db_path is None:
            raise ValueError("Сначала соберите данные с помощью collect_data()")
        
        # Обучаем модель с использованием существующей функции
        train_ranker(
            self._db_path, 
            model_path, 
            onnx_out=onnx_path
        )
        
        # Загружаем обученную модель
        self._model = CatBoostRanker()
        self._model.load_model(model_path.as_posix(), format="json")
        
        return self._model
    
    def export_model(
        self,
        model_path: Path,
        export_format: str,
        output_path: Path
    ) -> None:
        """
        Экспортирует модель в указанный формат.
        
        Args:
            model_path: Путь к обученной модели
            export_format: Формат экспорта ('onnx', 'json', 'cbm')
            output_path: Путь для сохранения
            
        Raises:
            ValueError: если не удалось экспортировать модель
        """
        export_model(model_path, export_format, output_path)
    
    def run_complete_pipeline(
        self,
        model_path: Path,
        onnx_path: Optional[Path] = None,
        use_categorical: bool = True,
        **train_params
    ) -> CatBoostRanker:
        """
        Выполняет полный pipeline: сбор данных -> обучение -> экспорт.
        
        Args:
            model_path: Путь для сохранения модели в формате JSON
            onnx_path: Опциональный путь для сохранения модели в формате ONNX
            use_categorical: Использовать категориальные фичи
            **train_params: Дополнительные параметры для обучения
            
        Returns:
            Обученная модель CatBoostRanker
            
        Raises:
            ValueError: если не удалось выполнить pipeline
        """
        # Шаг 1: Сбор данных
        print("Шаг 1/4: Сбор данных...")
        self.collect_data()
        
        # Шаг 2: Валидация данных
        print("Шаг 2/4: Валидация данных...")
        stats = self.validate_data()
        print(f"Датасет валидирован: {stats['snapshot_count']} снапшотов, "
              f"{stats['process_count']} процессов, {stats['group_count']} групп")
        
        # Шаг 3: Обучение модели
        print("Шаг 3/4: Обучение модели...")
        model = self.train_model(model_path, onnx_path, use_categorical, **train_params)
        
        # Шаг 4: Завершение
        print("Шаг 4/4: Pipeline завершен успешно!")
        
        return model
    
    def cleanup(self) -> None:
        """
        Очищает временные ресурсы.
        
        Удаляет временную базу данных, если она была создана.
        """
        if (self._db_path and 
            self.use_temp_db and 
            self._db_path.exists() and
            self._db_path.name.endswith(".db")):
            try:
                self._db_path.unlink()
                print(f"Временная база данных удалена: {self._db_path}")
            except Exception as e:
                print(f"Не удалось удалить временную базу данных: {e}")


def train_from_snapshots(
    snapshot_files: Union[Path, list[Path]],
    model_path: Path,
    onnx_path: Optional[Path] = None,
    use_temp_db: bool = True,
    min_snapshots: int = 1,
    min_processes: int = 10,
    min_groups: int = 1,
    **train_params
) -> CatBoostRanker:
    """
    Удобная функция для обучения модели из снапшотов.
    
    Args:
        snapshot_files: Путь к файлу снапшота или список путей
        model_path: Путь для сохранения модели в формате JSON
        onnx_path: Опциональный путь для сохранения модели в формате ONNX
        use_temp_db: Использовать временную базу данных
        min_snapshots: Минимальное количество снапшотов для валидации
        min_processes: Минимальное количество процессов для валидации
        min_groups: Минимальное количество групп для валидации
        **train_params: Дополнительные параметры для обучения
        
    Returns:
        Обученная модель CatBoostRanker
        
    Raises:
        ValueError: если не удалось обучить модель
    """
    pipeline = TrainingPipeline(
        snapshot_files=snapshot_files,
        use_temp_db=use_temp_db,
        min_snapshots=min_snapshots,
        min_processes=min_processes,
        min_groups=min_groups
    )
    
    return pipeline.run_complete_pipeline(model_path, onnx_path, **train_params)


def train_from_database(
    db_path: Path,
    model_path: Path,
    onnx_path: Optional[Path] = None,
    min_snapshots: int = 1,
    min_processes: int = 10,
    min_groups: int = 1,
    **train_params
) -> CatBoostRanker:
    """
    Удобная функция для обучения модели из существующей базы данных.
    
    Args:
        db_path: Путь к существующей базе данных
        model_path: Путь для сохранения модели в формате JSON
        onnx_path: Опциональный путь для сохранения модели в формате ONNX
        min_snapshots: Минимальное количество снапшотов для валидации
        min_processes: Минимальное количество процессов для валидации
        min_groups: Минимальное количество групп для валидации
        **train_params: Дополнительные параметры для обучения
        
    Returns:
        Обученная модель CatBoostRanker
        
    Raises:
        ValueError: если не удалось обучить модель
    """
    pipeline = TrainingPipeline(
        db_path=db_path,
        use_temp_db=False,
        min_snapshots=min_snapshots,
        min_processes=min_processes,
        min_groups=min_groups
    )
    
    return pipeline.run_complete_pipeline(model_path, onnx_path, **train_params)


if __name__ == "__main__":
    import argparse
    
    parser = argparse.ArgumentParser(
        description="Комплексный pipeline для сбора данных и обучения модели"
    )
    
    parser.add_argument(
        "--snapshots", 
        type=Path, 
        nargs="+",
        help="Путь к файлу(ам) снапшотов (JSONL или GZ)"
    )
    
    parser.add_argument(
        "--db", 
        type=Path,
        help="Путь к существующей базе данных (альтернатива --snapshots)"
    )
    
    parser.add_argument(
        "--model-json", 
        type=Path, 
        required=True,
        help="Путь для сохранения модели в формате JSON"
    )
    
    parser.add_argument(
        "--model-onnx", 
        type=Path,
        help="Путь для сохранения модели в формате ONNX"
    )
    
    parser.add_argument(
        "--use-temp-db", 
        action="store_true",
        help="Использовать временную базу данных"
    )
    
    parser.add_argument(
        "--min-snapshots", 
        type=int, 
        default=1,
        help="Минимальное количество снапшотов для валидации"
    )
    
    parser.add_argument(
        "--min-processes", 
        type=int, 
        default=10,
        help="Минимальное количество процессов для валидации"
    )
    
    parser.add_argument(
        "--min-groups", 
        type=int, 
        default=1,
        help="Минимальное количество групп для валидации"
    )
    
    args = parser.parse_args()
    
    if args.snapshots and args.db:
        parser.error("Можно указать либо --snapshots, либо --db, но не оба одновременно")
    
    if not args.snapshots and not args.db:
        parser.error("Необходимо указать либо --snapshots, либо --db")
    
    if args.snapshots:
        train_from_snapshots(
            args.snapshots,
            args.model_json,
            args.model_onnx,
            use_temp_db=args.use_temp_db,
            min_snapshots=args.min_snapshots,
            min_processes=args.min_processes,
            min_groups=args.min_groups
        )
    else:
        train_from_database(
            args.db,
            args.model_json,
            args.model_onnx,
            min_snapshots=args.min_snapshots,
            min_processes=args.min_processes,
            min_groups=args.min_groups
        )