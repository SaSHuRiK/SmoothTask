"""Обучение CatBoostRanker для ранжирования процессов."""

from pathlib import Path

from catboost import CatBoostRanker, Pool

from .dataset import load_snapshots_as_frame
from .features import build_feature_matrix


def train_ranker(db_path: Path, model_out: Path, onnx_out: Path | None = None):
    """
    Обучает CatBoostRanker на снапшотах и сохраняет модель.
    
    Функция загружает снапшоты из SQLite базы данных, строит матрицу фич
    и обучает CatBoostRanker с параметрами по умолчанию (YetiRank loss,
    depth=6, learning_rate=0.1, iterations=500).
    
    Args:
        db_path: Путь к SQLite базе данных со снапшотами
        model_out: Путь для сохранения модели в формате JSON
        onnx_out: Опциональный путь для сохранения модели в формате ONNX
        
    Raises:
        FileNotFoundError: если база данных не существует
        ValueError: если данные недостаточны для обучения
    """
    df = load_snapshots_as_frame(db_path)
    X, y, group_id, cat_features = build_feature_matrix(df)

    train_pool = Pool(
        data=X,
        label=y,
        group_id=group_id,
        cat_features=cat_features,
    )

    model = CatBoostRanker(
        loss_function="YetiRank",
        depth=6,
        learning_rate=0.1,
        iterations=500,
        random_state=42,
    )
    model.fit(train_pool, verbose=True)

    model.save_model(model_out.as_posix(), format="json")

    if onnx_out is not None:
        model.save_model(onnx_out.as_posix(), format="onnx")


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("--db", type=Path, required=True)
    parser.add_argument("--model-json", type=Path, required=True)
    parser.add_argument("--model-onnx", type=Path)
    args = parser.parse_args()

    train_ranker(args.db, args.model_json, args.model_onnx)

