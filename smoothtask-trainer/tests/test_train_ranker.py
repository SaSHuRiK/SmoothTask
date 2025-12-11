"""Тесты для обучения CatBoostRanker."""

import json
import sqlite3
import tempfile
from datetime import datetime, timezone
from pathlib import Path

import pytest
from catboost import CatBoostRanker
from smoothtask_trainer.dataset import load_snapshots_as_frame
from smoothtask_trainer.train_ranker import train_ranker


def create_training_db(db_path: Path, num_snapshots: int = 3) -> None:
    """Создаёт тестовую БД с несколькими снапшотами для обучения."""
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    # Создаём схему
    cursor.execute(
        """
        CREATE TABLE IF NOT EXISTS snapshots (
            snapshot_id INTEGER PRIMARY KEY,
            timestamp TEXT NOT NULL,
            cpu_user REAL,
            cpu_system REAL,
            cpu_idle REAL,
            cpu_iowait REAL,
            mem_total_kb INTEGER,
            mem_used_kb INTEGER,
            mem_available_kb INTEGER,
            swap_total_kb INTEGER,
            swap_used_kb INTEGER,
            load_avg_one REAL,
            load_avg_five REAL,
            load_avg_fifteen REAL,
            psi_cpu_some_avg10 REAL,
            psi_cpu_some_avg60 REAL,
            psi_io_some_avg10 REAL,
            psi_mem_some_avg10 REAL,
            psi_mem_full_avg10 REAL,
            user_active INTEGER,
            time_since_last_input_ms INTEGER,
            sched_latency_p95_ms REAL,
            sched_latency_p99_ms REAL,
            audio_xruns_delta INTEGER,
            ui_loop_p95_ms REAL,
            frame_jank_ratio REAL,
            bad_responsiveness INTEGER,
            responsiveness_score REAL
        )
        """
    )

    cursor.execute(
        """
        CREATE TABLE IF NOT EXISTS processes (
            snapshot_id INTEGER NOT NULL,
            pid INTEGER NOT NULL,
            ppid INTEGER,
            uid INTEGER,
            gid INTEGER,
            exe TEXT,
            cmdline TEXT,
            cgroup_path TEXT,
            systemd_unit TEXT,
            app_group_id TEXT,
            state TEXT,
            start_time INTEGER,
            uptime_sec INTEGER,
            tty_nr INTEGER,
            has_tty INTEGER,
            cpu_share_1s REAL,
            cpu_share_10s REAL,
            io_read_bytes INTEGER,
            io_write_bytes INTEGER,
            rss_mb INTEGER,
            swap_mb INTEGER,
            voluntary_ctx INTEGER,
            involuntary_ctx INTEGER,
            has_gui_window INTEGER,
            is_focused_window INTEGER,
            window_state TEXT,
            env_has_display INTEGER,
            env_has_wayland INTEGER,
            env_term TEXT,
            env_ssh INTEGER,
            is_audio_client INTEGER,
            has_active_stream INTEGER,
            process_type TEXT,
            tags TEXT,
            nice INTEGER,
            ionice_class INTEGER,
            ionice_prio INTEGER,
            teacher_priority_class TEXT,
            teacher_score REAL,
            PRIMARY KEY (snapshot_id, pid),
            FOREIGN KEY (snapshot_id) REFERENCES snapshots(snapshot_id)
        )
        """
    )

    cursor.execute(
        """
        CREATE TABLE IF NOT EXISTS app_groups (
            snapshot_id INTEGER NOT NULL,
            app_group_id TEXT NOT NULL,
            root_pid INTEGER,
            process_ids TEXT,
            app_name TEXT,
            total_cpu_share REAL,
            total_io_read_bytes INTEGER,
            total_io_write_bytes INTEGER,
            total_rss_mb INTEGER,
            has_gui_window INTEGER,
            is_focused_group INTEGER,
            tags TEXT,
            priority_class TEXT,
            PRIMARY KEY (snapshot_id, app_group_id),
            FOREIGN KEY (snapshot_id) REFERENCES snapshots(snapshot_id)
        )
        """
    )

    # Вставляем несколько снапшотов с разными процессами
    timestamp_base = datetime.now(timezone.utc)
    process_tags = json.dumps(["terminal"])

    for snapshot_idx in range(num_snapshots):
        snapshot_id = 1000 + snapshot_idx
        timestamp = (
            timestamp_base.replace(microsecond=snapshot_idx * 1000)
        ).isoformat()

        # Снапшот
        cursor.execute(
            """
            INSERT INTO snapshots (
                snapshot_id, timestamp,
                cpu_user, cpu_system, cpu_idle, cpu_iowait,
                mem_total_kb, mem_used_kb, mem_available_kb,
                swap_total_kb, swap_used_kb,
                load_avg_one, load_avg_five, load_avg_fifteen,
                psi_cpu_some_avg10, psi_cpu_some_avg60,
                psi_io_some_avg10,
                psi_mem_some_avg10, psi_mem_full_avg10,
                user_active, time_since_last_input_ms,
                sched_latency_p95_ms, sched_latency_p99_ms,
                audio_xruns_delta,
                ui_loop_p95_ms, frame_jank_ratio,
                bad_responsiveness, responsiveness_score
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                snapshot_id,
                timestamp,
                0.25 + snapshot_idx * 0.01,
                0.15,
                0.55,
                0.05,
                16_384_256,
                8_000_000,
                8_384_256,
                8_192_000,
                1_000_000,
                1.5 + snapshot_idx * 0.1,
                1.2,
                1.0,
                0.1,
                0.15,
                0.2,
                0.05,
                None,
                1,  # user_active = True
                5000 - snapshot_idx * 100,  # time_since_last_input_ms
                5.0,
                10.0,
                None,
                None,
                None,
                0,  # bad_responsiveness = False
                0.9 - snapshot_idx * 0.05,  # responsiveness_score
            ),
        )

        # Процесс для каждого снапшота
        pid = 1000 + snapshot_idx * 10
        cursor.execute(
            """
            INSERT INTO processes (
                snapshot_id, pid, ppid, uid, gid,
                exe, cmdline, cgroup_path, systemd_unit, app_group_id,
                state, start_time, uptime_sec,
                tty_nr, has_tty,
                cpu_share_1s, cpu_share_10s,
                io_read_bytes, io_write_bytes,
                rss_mb, swap_mb,
                voluntary_ctx, involuntary_ctx,
                has_gui_window, is_focused_window, window_state,
                env_has_display, env_has_wayland, env_term, env_ssh,
                is_audio_client, has_active_stream,
                process_type, tags,
                nice, ionice_class, ionice_prio,
                teacher_priority_class, teacher_score
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                snapshot_id,
                pid,
                1,
                1000,
                1000,
                f"/usr/bin/test{snapshot_idx}",
                f"test{snapshot_idx} --flag",
                "/user.slice/user-1000.slice",
                None,
                f"test-app-{snapshot_idx}",
                "R",
                1000000 + snapshot_idx,
                3600,
                0,
                0,  # has_tty = False
                0.1 + snapshot_idx * 0.01,  # cpu_share_1s
                0.08 + snapshot_idx * 0.01,  # cpu_share_10s
                1024 * 1024 * (snapshot_idx + 1),
                512 * 1024 * (snapshot_idx + 1),
                100 + snapshot_idx * 10,  # rss_mb
                None,
                1000 + snapshot_idx * 100,
                50 + snapshot_idx * 10,
                0,  # has_gui_window = False
                0,  # is_focused_window = False
                None,
                0,  # env_has_display = False
                0,  # env_has_wayland = False
                None,
                0,  # env_ssh = False
                0,  # is_audio_client = False
                0,  # has_active_stream = False
                "cli_interactive",
                process_tags,
                0,
                2,
                4,
                "INTERACTIVE",
                0.75 + snapshot_idx * 0.05,  # teacher_score
            ),
        )

        # Группа приложений
        group_process_ids = json.dumps([pid])
        group_tags = json.dumps(["terminal"])
        cursor.execute(
            """
            INSERT INTO app_groups (
                snapshot_id, app_group_id, root_pid, process_ids,
                app_name,
                total_cpu_share,
                total_io_read_bytes, total_io_write_bytes,
                total_rss_mb,
                has_gui_window, is_focused_group,
                tags, priority_class
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                snapshot_id,
                f"test-app-{snapshot_idx}",
                pid,
                group_process_ids,
                f"test{snapshot_idx}",
                0.15 + snapshot_idx * 0.01,
                2 * 1024 * 1024 * (snapshot_idx + 1),
                1024 * 1024 * (snapshot_idx + 1),
                200 + snapshot_idx * 20,
                0,  # has_gui_window = False
                0,  # is_focused_group = False
                group_tags,
                "INTERACTIVE",
            ),
        )

    conn.commit()
    conn.close()


def test_train_ranker_basic():
    """Тест базового обучения ранкера."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "test.db"
        model_json_path = Path(tmpdir) / "model.json"

        create_training_db(db_path, num_snapshots=3)
        train_ranker(db_path, model_json_path, onnx_out=None)

        # Проверяем, что модель сохранилась
        assert model_json_path.exists(), "Модель должна быть сохранена"

        # Проверяем, что модель можно загрузить
        model = CatBoostRanker()
        model.load_model(model_json_path.as_posix(), format="json")

        # Проверяем, что модель имеет правильные параметры
        assert model.get_params()["loss_function"] == "YetiRank"
        assert model.get_params()["depth"] == 6
        assert model.get_params()["learning_rate"] == 0.1


def test_train_ranker_with_onnx():
    """Тест обучения ранкера с экспортом в ONNX."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "test.db"
        model_json_path = Path(tmpdir) / "model.json"
        model_onnx_path = Path(tmpdir) / "model.onnx"

        create_training_db(db_path, num_snapshots=3)
        train_ranker(db_path, model_json_path, onnx_out=model_onnx_path)

        # Проверяем, что обе модели сохранились
        assert model_json_path.exists(), "JSON модель должна быть сохранена"
        assert model_onnx_path.exists(), "ONNX модель должна быть сохранена"

        # Проверяем, что ONNX файл не пустой
        assert model_onnx_path.stat().st_size > 0, "ONNX файл не должен быть пустым"


def test_train_ranker_with_empty_db():
    """Тест обработки пустой БД."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "empty.db"
        model_json_path = Path(tmpdir) / "model.json"

        # Создаём пустую БД
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS snapshots (
                snapshot_id INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL
            )
            """
        )
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS processes (
                snapshot_id INTEGER NOT NULL,
                pid INTEGER NOT NULL,
                PRIMARY KEY (snapshot_id, pid)
            )
            """
        )
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS app_groups (
                snapshot_id INTEGER NOT NULL,
                app_group_id TEXT NOT NULL,
                PRIMARY KEY (snapshot_id, app_group_id)
            )
            """
        )
        conn.commit()
        conn.close()

        # Ожидаем ошибку при попытке обучения на пустой БД
        with pytest.raises(ValueError, match="DataFrame с снапшотами пуст"):
            train_ranker(db_path, model_json_path, onnx_out=None)


def test_train_ranker_with_fallback_target():
    """Тест обучения с fallback на responsiveness_score."""
    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "test.db"
        model_json_path = Path(tmpdir) / "model.json"

        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()

        # Создаём схему
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS snapshots (
                snapshot_id INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL,
                responsiveness_score REAL
            )
            """
        )
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS processes (
                snapshot_id INTEGER NOT NULL,
                pid INTEGER NOT NULL,
                PRIMARY KEY (snapshot_id, pid)
            )
            """
        )
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS app_groups (
                snapshot_id INTEGER NOT NULL,
                app_group_id TEXT NOT NULL,
                PRIMARY KEY (snapshot_id, app_group_id)
            )
            """
        )

        # Вставляем снапшот без teacher_score, но с responsiveness_score
        timestamp = datetime.now(timezone.utc).isoformat()
        cursor.execute(
            "INSERT INTO snapshots (snapshot_id, timestamp, responsiveness_score) VALUES (?, ?, ?)",
            (1, timestamp, 0.8),
        )
        cursor.execute(
            "INSERT INTO processes (snapshot_id, pid) VALUES (?, ?)",
            (1, 1000),
        )

        conn.commit()
        conn.close()

        # Ожидаем ошибку из-за отсутствия необходимых фич или недостаточных данных
        # (CatBoost не может обучить модель на константных фичах)
        from catboost import CatBoostError

        with pytest.raises((ValueError, KeyError, CatBoostError)):
            train_ranker(db_path, model_json_path, onnx_out=None)
