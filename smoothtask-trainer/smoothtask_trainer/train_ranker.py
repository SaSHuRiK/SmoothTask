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
        ValueError: если данные недостаточны для обучения или параметры невалидны
    """
    # Валидация входных параметров
    if not db_path.exists():
        raise FileNotFoundError(f"База данных не найдена: {db_path}")

    if not db_path.is_file():
        raise ValueError(f"Путь к базе данных должен указывать на файл: {db_path}")

    if model_out.exists() and model_out.is_dir():
        raise ValueError(f"Путь для сохранения модели указывает на директорию: {model_out}")

    if onnx_out is not None:
        if onnx_out.exists() and onnx_out.is_dir():
            raise ValueError(f"Путь для сохранения ONNX модели указывает на директорию: {onnx_out}")

    df = load_snapshots_as_frame(db_path)
    # Для ONNX экспорта не используем категориальные фичи
    use_categorical = onnx_out is None
    X, y, group_id, cat_features = build_feature_matrix(df, use_categorical=use_categorical)

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

    try:
        model.fit(train_pool, verbose=True)
    except Exception as e:
        raise ValueError(
            f"Ошибка при обучении модели: {e}. "
            "Проверьте, что данные содержат достаточное количество снапшотов и групп."
        ) from e

    # Создаём директорию для модели, если её нет
    model_out.parent.mkdir(parents=True, exist_ok=True)

    try:
        model.save_model(model_out.as_posix(), format="json")
    except Exception as e:
        raise ValueError(
            f"Ошибка при сохранении модели в JSON: {e}. "
            f"Проверьте, что путь доступен для записи: {model_out}"
        ) from e

    if onnx_out is not None:
        # Создаём директорию для ONNX модели, если её нет
        onnx_out.parent.mkdir(parents=True, exist_ok=True)
        try:
            model.save_model(onnx_out.as_posix(), format="onnx")
        except Exception as e:
            raise ValueError(
                f"Ошибка при сохранении модели в ONNX: {e}. "
                f"Проверьте, что путь доступен для записи: {onnx_out}"
            ) from e


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("--db", type=Path, required=True)
    parser.add_argument("--model-json", type=Path, required=True)
    parser.add_argument("--model-onnx", type=Path)
    args = parser.parse_args()

    train_ranker(args.db, args.model_json, args.model_onnx)
