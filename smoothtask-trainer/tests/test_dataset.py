"""Тесты для чтения снапшотов из SQLite."""

import json
import warnings
import sqlite3
import tempfile
from datetime import datetime, timezone
from pathlib import Path

import pandas as pd
import pytest
from smoothtask_trainer.dataset import _json_list, _to_bool, load_snapshots_as_frame


def create_test_db(db_path: Path) -> None:
    """Создаёт тестовую БД с одним снапшотом."""
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    # Создаём схему (копируем из Rust-кода)
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

    # Вставляем тестовые данные
    timestamp = datetime.now(timezone.utc).isoformat()
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
            1234567890,
            timestamp,
            0.25,
            0.15,
            0.55,
            0.05,
            16_384_256,
            8_000_000,
            8_384_256,
            8_192_000,
            1_000_000,
            1.5,
            1.2,
            1.0,
            0.1,
            0.15,
            0.2,
            0.05,
            None,
            1,  # user_active = True
            5000,  # time_since_last_input_ms
            5.0,  # sched_latency_p95_ms
            10.0,  # sched_latency_p99_ms
            None,  # audio_xruns_delta
            None,  # ui_loop_p95_ms
            None,  # frame_jank_ratio
            0,  # bad_responsiveness = False
            0.9,  # responsiveness_score
        ),
    )

    # Процесс
    process_tags = json.dumps(["terminal"])
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
            1234567890,  # snapshot_id
            1234,  # pid
            1,  # ppid
            1000,  # uid
            1000,  # gid
            "/usr/bin/test",  # exe
            "test --flag",  # cmdline
            "/user.slice/user-1000.slice",  # cgroup_path
            None,  # systemd_unit
            "test-app",  # app_group_id
            "R",  # state
            1000000,  # start_time
            3600,  # uptime_sec
            0,  # tty_nr
            0,  # has_tty = False
            0.1,  # cpu_share_1s
            0.08,  # cpu_share_10s
            1024 * 1024,  # io_read_bytes
            512 * 1024,  # io_write_bytes
            100,  # rss_mb
            None,  # swap_mb
            1000,  # voluntary_ctx
            50,  # involuntary_ctx
            0,  # has_gui_window = False
            0,  # is_focused_window = False
            None,  # window_state
            0,  # env_has_display = False
            0,  # env_has_wayland = False
            None,  # env_term
            0,  # env_ssh = False
            0,  # is_audio_client = False
            0,  # has_active_stream = False
            "cli_interactive",  # process_type
            process_tags,  # tags
            0,  # nice
            2,  # ionice_class
            4,  # ionice_prio
            "INTERACTIVE",  # teacher_priority_class
            0.75,  # teacher_score
        ),
    )

    # Группа приложений
    group_process_ids = json.dumps([1234, 1235])
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
            1234567890,  # snapshot_id
            "test-app",  # app_group_id
            1234,  # root_pid
            group_process_ids,  # process_ids
            "test",  # app_name
            0.15,  # total_cpu_share
            2 * 1024 * 1024,  # total_io_read_bytes
            1024 * 1024,  # total_io_write_bytes
            200,  # total_rss_mb
            0,  # has_gui_window = False
            0,  # is_focused_group = False
            group_tags,  # tags
            "INTERACTIVE",  # priority_class
        ),
    )

    conn.commit()
    conn.close()


def test_load_snapshots_as_frame_basic():
    """Тест базового чтения снапшотов."""
    with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as tmp:
        db_path = Path(tmp.name)

    try:
        create_test_db(db_path)
        df = load_snapshots_as_frame(db_path)

        assert not df.empty, "DataFrame не должен быть пустым"
        assert len(df) == 1, "Должен быть один процесс"

        # Проверяем, что булевые колонки правильно распарсены
        assert df["user_active"].dtype == "boolean"
        assert df["user_active"].iloc[0] == True
        assert df["bad_responsiveness"].dtype == "boolean"
        assert df["bad_responsiveness"].iloc[0] == False

        assert df["has_tty"].dtype == "boolean"
        assert df["has_tty"].iloc[0] == False
        assert df["has_gui_window"].dtype == "boolean"
        assert df["has_gui_window"].iloc[0] == False

        # Проверяем JSON-поля
        assert isinstance(df["tags"].iloc[0], list)
        assert df["tags"].iloc[0] == ["terminal"]

        # Проверяем, что данные из снапшота доступны
        assert "cpu_user" in df.columns
        assert abs(df["cpu_user"].iloc[0] - 0.25) < 1e-6
        assert "mem_total_kb" in df.columns
        assert df["mem_total_kb"].iloc[0] == 16_384_256

        # Проверяем, что данные из группы доступны
        assert "app_name" in df.columns
        assert df["app_name"].iloc[0] == "test"
        assert isinstance(df["process_ids"].iloc[0], list)
        assert df["process_ids"].iloc[0] == [1234, 1235]

        # Проверяем timestamp
        assert "timestamp" in df.columns
        assert pd.api.types.is_datetime64_any_dtype(df["timestamp"])

    finally:
        db_path.unlink(missing_ok=True)


def test_load_snapshots_as_frame_empty_db():
    """Тест чтения из пустой БД."""
    with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as tmp:
        db_path = Path(tmp.name)

    try:
        # Создаём пустую БД со схемой
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()
        cursor.execute(
            "CREATE TABLE IF NOT EXISTS snapshots (snapshot_id INTEGER PRIMARY KEY)"
        )
        cursor.execute(
            "CREATE TABLE IF NOT EXISTS processes (snapshot_id INTEGER, pid INTEGER)"
        )
        cursor.execute(
            "CREATE TABLE IF NOT EXISTS app_groups (snapshot_id INTEGER, app_group_id TEXT)"
        )
        conn.commit()
        conn.close()

        df = load_snapshots_as_frame(db_path)
        assert df.empty, "DataFrame должен быть пустым для пустой БД"

    finally:
        db_path.unlink(missing_ok=True)


def test_load_snapshots_as_frame_file_not_found():
    """Тест обработки отсутствующего файла."""
    non_existent = Path("/tmp/non_existent_snapshots.db")
    with pytest.raises(FileNotFoundError):
        load_snapshots_as_frame(non_existent)


def test_load_snapshots_as_frame_multiple_snapshots():
    """Тест чтения нескольких снапшотов."""
    with tempfile.NamedTemporaryFile(suffix=".db", delete=False) as tmp:
        db_path = Path(tmp.name)

    try:
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()

        # Создаём схему
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS snapshots (
                snapshot_id INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL,
                cpu_user REAL,
                user_active INTEGER
            )
            """
        )
        cursor.execute(
            """
            CREATE TABLE IF NOT EXISTS processes (
                snapshot_id INTEGER NOT NULL,
                pid INTEGER NOT NULL,
                app_group_id TEXT,
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

        # Вставляем два снапшота
        timestamp = datetime.now(timezone.utc).isoformat()
        cursor.execute(
            "INSERT INTO snapshots (snapshot_id, timestamp, cpu_user, user_active) VALUES (?, ?, ?, ?)",
            (1000, timestamp, 0.1, 1),
        )
        cursor.execute(
            "INSERT INTO snapshots (snapshot_id, timestamp, cpu_user, user_active) VALUES (?, ?, ?, ?)",
            (2000, timestamp, 0.2, 0),
        )

        cursor.execute(
            "INSERT INTO processes (snapshot_id, pid, app_group_id) VALUES (?, ?, ?)",
            (1000, 100, "app1"),
        )
        cursor.execute(
            "INSERT INTO processes (snapshot_id, pid, app_group_id) VALUES (?, ?, ?)",
            (2000, 200, "app2"),
        )

        conn.commit()
        conn.close()

        df = load_snapshots_as_frame(db_path)

        assert len(df) == 2, "Должно быть два процесса"
        assert set(df["snapshot_id"]) == {1000, 2000}
        assert set(df["pid"]) == {100, 200}

    finally:
        db_path.unlink(missing_ok=True)


def test_json_list_handles_empty_and_none():
    """_json_list должен безопасно обрабатывать None и пустые строки."""
    assert _json_list(None) == []
    assert _json_list("") == []
    assert _json_list("   ") == []
    assert _json_list("[1, 2, 3]") == [1, 2, 3]


def test_json_list_rejects_non_list_payload():
    """_json_list должен отдавать ValueError, если JSON не список."""
    with pytest.raises(ValueError):
        _json_list('{"key": "value"}')


def test_to_bool_converts_present_columns_only():
    """_to_bool конвертирует только существующие колонки, сохраняя NaN."""
    df = pd.DataFrame(
        {
            "flag_int": [1, 0, None],
            "already_bool": [True, False, None],
        }
    )

    _to_bool(df, ["flag_int", "missing"])

    assert df["flag_int"].dtype == "boolean"
    assert list(df["flag_int"]) == [True, False, pd.NA]
    assert list(df["already_bool"]) == [True, False, None]


def test_to_bool_no_warnings_for_nullable_bools():
    """_to_bool не должен выдавать предупреждений при nullable boolean."""
    df = pd.DataFrame(
        {
            "flag_int": pd.Series([1, 0, pd.NA], dtype="Int64"),
            "flag_bool": pd.Series([True, False, pd.NA], dtype="boolean"),
        }
    )

    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("error")
        _to_bool(df, ["flag_int", "flag_bool"])

    assert not caught
    assert df["flag_int"].dtype == "boolean"
    assert df["flag_bool"].dtype == "boolean"
