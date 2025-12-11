"""Тесты для построения фич тренера."""

import numpy as np
import pandas as pd
import pytest

from smoothtask_trainer.features import build_feature_matrix


def test_build_feature_matrix_basic():
    df = pd.DataFrame(
        {
            "snapshot_id": [1],
            "teacher_score": [0.8],
            "responsiveness_score": [0.5],
            "cpu_share_1s": [0.12],
            "load_avg_one": [1.0],
            "has_tty": [True],
            "has_gui_window": [False],
            "user_active": [True],
            "process_type": ["cli_interactive"],
            "app_name": [None],
            "priority_class": ["INTERACTIVE"],
            "teacher_priority_class": ["INTERACTIVE"],
            "env_term": ["xterm-256color"],
            "tags": [["terminal", "ssh"]],
        }
    )

    X, y, group_id, cat_idx = build_feature_matrix(df)

    assert list(group_id) == [1]
    assert y.iloc[0] == pytest.approx(0.8)

    # Булевые колонки должны конвертироваться в 0/1
    assert X.loc[0, "has_tty"] == 1
    assert X.loc[0, "has_gui_window"] == 0
    assert X.loc[0, "user_active"] == 1

    # Категориальные фичи присутствуют и попадают в cat_idx
    for col in ("process_type", "app_name", "tags_joined"):
        idx = X.columns.get_loc(col)
        assert idx in cat_idx
    assert X.loc[0, "app_name"] == "unknown"  # заполняется значением по умолчанию
    assert X.loc[0, "tags_joined"] == "ssh|terminal"  # сортировка и join

    # Числовые фичи сохраняют значения
    assert X.loc[0, "cpu_share_1s"] == pytest.approx(0.12)
    assert X.loc[0, "load_avg_one"] == pytest.approx(1.0)


def test_build_feature_matrix_fallback_and_defaults():
    df = pd.DataFrame(
        {
            "snapshot_id": [10, 11],
            "teacher_score": [np.nan, np.nan],
            "responsiveness_score": [0.2, np.nan],
        }
    )

    X, y, group_id, _ = build_feature_matrix(df)

    # Вторая строка должна быть отброшена из-за отсутствия таргета
    assert len(y) == 1
    assert y.iloc[0] == pytest.approx(0.2)
    assert group_id.iloc[0] == 10

    # Отсутствующие числовые фичи должны заполняться нулями
    zero_cols = ["cpu_share_1s", "load_avg_one", "total_cpu_share"]
    for col in zero_cols:
        assert col in X.columns
        assert X.loc[0, col] == 0.0


def test_build_feature_matrix_tags_and_cat_defaults():
    df = pd.DataFrame(
        {
            "snapshot_id": [21, 22],
            "responsiveness_score": [0.3, 0.4],
            "tags": [np.nan, ["app"]],
            "app_name": [np.nan, "player"],
            "has_tty": [np.nan, True],
        }
    )

    X, y, group_id, cat_idx = build_feature_matrix(df)

    # Оба таргета сохраняются и совпадают с snapshot_id
    assert list(y) == [pytest.approx(0.3), pytest.approx(0.4)]
    assert list(group_id) == [21, 22]

    # Отсутствующие теги и app_name заменяются на unknown
    assert X.loc[0, "tags_joined"] == "unknown"
    assert X.loc[0, "app_name"] == "unknown"
    # Переданные теги упорядочиваются и соединяются
    assert X.loc[1, "tags_joined"] == "app"

    # Булевые фичи при отсутствии значений превращаются в 0
    assert X.loc[0, "has_tty"] == 0
    assert X.loc[1, "has_tty"] == 1

    # Индексы категориальных фичей указывают на существующие колонки
    for col in ("process_type", "tags_joined", "app_name"):
        assert X.columns.get_loc(col) in cat_idx


def test_build_feature_matrix_requires_snapshot_id():
    df = pd.DataFrame({"teacher_score": [0.5]})
    with pytest.raises(ValueError):
        build_feature_matrix(df)


def test_build_feature_matrix_empty_dataframe():
    df = pd.DataFrame()
    with pytest.raises(ValueError):
        build_feature_matrix(df)


def test_build_feature_matrix_empty_and_set_tags():
    df = pd.DataFrame(
        {
            "snapshot_id": [31, 32, 33],
            "teacher_score": [0.5, 0.6, 0.7],
            "tags": [[], {"b", "a"}, ("one", "two")],
            "app_name": ["alpha", "beta", None],
        }
    )

    X, y, group_id, cat_idx = build_feature_matrix(df)

    assert list(group_id) == [31, 32, 33]
    assert list(y) == [
        pytest.approx(0.5),
        pytest.approx(0.6),
        pytest.approx(0.7),
    ]

    assert X.loc[0, "tags_joined"] == "unknown"  # пустая коллекция -> unknown
    assert X.loc[1, "tags_joined"] == "a|b"  # set сортируется детерминированно
    assert X.loc[2, "tags_joined"] == "one|two"  # кортеж поддерживается

    expected_cat_idx = [
        X.columns.get_loc("process_type"),
        X.columns.get_loc("app_name"),
        X.columns.get_loc("priority_class"),
        X.columns.get_loc("teacher_priority_class"),
        X.columns.get_loc("env_term"),
        X.columns.get_loc("tags_joined"),
    ]
    assert cat_idx == expected_cat_idx


def test_build_feature_matrix_cat_indices_order():
    df = pd.DataFrame(
        {
            "snapshot_id": [1, 1],
            "teacher_score": [1.0, 0.9],
            "process_type": ["gui", "terminal"],
            "app_name": ["player", "shell"],
            "priority_class": ["INTERACTIVE", "BACKGROUND"],
            "teacher_priority_class": ["INTERACTIVE", "BACKGROUND"],
            "env_term": ["xterm", "xterm"],
            "tags": [["media", "video"], ["ssh"]],
        }
    )

    X, _, _, cat_idx = build_feature_matrix(df)

    expected_cat_idx = [
        X.columns.get_loc("process_type"),
        X.columns.get_loc("app_name"),
        X.columns.get_loc("priority_class"),
        X.columns.get_loc("teacher_priority_class"),
        X.columns.get_loc("env_term"),
        X.columns.get_loc("tags_joined"),
    ]
    assert cat_idx == expected_cat_idx
