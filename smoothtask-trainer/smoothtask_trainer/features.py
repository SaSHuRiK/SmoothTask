"""Трансформация и нормализация фич для CatBoostRanker."""

import pandas as pd
import numpy as np


def build_feature_matrix(df: pd.DataFrame):
    """
    Строит матрицу фич X, таргеты y, group_id и список категориальных фич.
    
    Returns:
        X: матрица фич
        y: таргеты (teacher_score или responsiveness_score)
        group_id: query_id для группировки (snapshot_id)
        cat_features: список индексов категориальных фич
    """
    # TODO: извлечение и трансформация фич
    # TODO: определение категориальных фич
    raise NotImplementedError("TODO: реализовать построение фич")


