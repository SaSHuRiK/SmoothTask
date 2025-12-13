#!/usr/bin/env python3
"""
Скрипт для генерации синтетических данных для обучения модели.
"""

import json
import sqlite3
import tempfile
from datetime import datetime, timezone, timedelta
from pathlib import Path
import sys
import random
import string

def create_training_database(db_path: Path, num_snapshots: int = 10, processes_per_snapshot: int = 20):
    """Создаёт базу данных с синтетическими данными для обучения."""
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
    
    # Генерация синтетических данных
    base_timestamp = datetime.now(timezone.utc)
    
    for snapshot_idx in range(num_snapshots):
        snapshot_id = 1000000 + snapshot_idx
        timestamp = (base_timestamp - timedelta(minutes=snapshot_idx)).isoformat()
        
        # Генерация данных для снапшота
        cpu_user = round(random.uniform(0.1, 0.5), 2)
        cpu_system = round(random.uniform(0.05, 0.2), 2)
        cpu_idle = round(1.0 - cpu_user - cpu_system - 0.05, 2)
        cpu_iowait = round(random.uniform(0.0, 0.1), 2)
        
        mem_total_kb = 16_384_256
        mem_used_kb = round(random.uniform(4_000_000, 12_000_000))
        mem_available_kb = mem_total_kb - mem_used_kb
        
        swap_total_kb = 8_192_000
        swap_used_kb = round(random.uniform(0, 2_000_000))
        
        load_avg_one = round(random.uniform(0.5, 3.0), 2)
        load_avg_five = round(random.uniform(0.3, 2.5), 2)
        load_avg_fifteen = round(random.uniform(0.2, 2.0), 2)
        
        psi_cpu_some_avg10 = round(random.uniform(0.0, 0.5), 3)
        psi_cpu_some_avg60 = round(random.uniform(0.0, 0.3), 3)
        psi_io_some_avg10 = round(random.uniform(0.0, 0.4), 3)
        psi_mem_some_avg10 = round(random.uniform(0.0, 0.2), 3)
        psi_mem_full_avg10 = round(random.uniform(0.0, 0.1), 3)
        
        user_active = 1 if snapshot_idx % 3 != 0 else 0
        time_since_last_input_ms = 5000 if user_active else 300000
        
        sched_latency_p95_ms = round(random.uniform(2.0, 20.0), 1)
        sched_latency_p99_ms = round(random.uniform(5.0, 30.0), 1)
        
        audio_xruns_delta = random.randint(0, 5) if random.random() < 0.1 else 0
        ui_loop_p95_ms = round(random.uniform(8.0, 25.0), 1) if random.random() < 0.8 else 0.0
        frame_jank_ratio = round(random.uniform(0.0, 0.3), 3) if random.random() < 0.3 else 0.0
        
        bad_responsiveness = 1 if sched_latency_p99_ms > 15.0 else 0
        responsiveness_score = round(random.uniform(0.5, 1.0), 2)
        
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
                cpu_user,
                cpu_system,
                cpu_idle,
                cpu_iowait,
                mem_total_kb,
                mem_used_kb,
                mem_available_kb,
                swap_total_kb,
                swap_used_kb,
                load_avg_one,
                load_avg_five,
                load_avg_fifteen,
                psi_cpu_some_avg10,
                psi_cpu_some_avg60,
                psi_io_some_avg10,
                psi_mem_some_avg10,
                psi_mem_full_avg10,
                user_active,
                time_since_last_input_ms,
                sched_latency_p95_ms,
                sched_latency_p99_ms,
                audio_xruns_delta,
                ui_loop_p95_ms,
                frame_jank_ratio,
                bad_responsiveness,
                responsiveness_score,
            ),
        )
        
        # Генерация процессов для этого снапшота
        process_ids = []
        app_groups_data = {}
        
        for process_idx in range(processes_per_snapshot):
            pid = 1000 + snapshot_idx * 1000 + process_idx
            ppid = 1 if process_idx == 0 else 1000 + snapshot_idx * 1000 + (process_idx - 1)
            
            # Тип процесса
            process_types = ["cli_interactive", "gui_interactive", "batch", "background", "audio"]
            process_type = random.choice(process_types)
            
            # Группа приложений
            if process_idx % 3 == 0:  # Новая группа каждые 3 процесса
                app_group_id = f"app-{snapshot_idx}-{process_idx // 3}"
                app_name = f"Application {process_idx // 3}"
                root_pid = pid
            else:
                app_group_id = f"app-{snapshot_idx}-{process_idx // 3}"
                app_name = f"Application {process_idx // 3}"
                root_pid = 1000 + snapshot_idx * 1000 + (process_idx // 3) * 3
            
            # Метрики процесса
            cpu_share_1s = round(random.uniform(0.01, 0.5), 3)
            cpu_share_10s = round(cpu_share_1s * random.uniform(0.8, 1.2), 3)
            
            io_read_bytes = round(random.uniform(1024, 1024 * 1024 * 10))
            io_write_bytes = round(random.uniform(512, 1024 * 1024 * 5))
            
            rss_mb = round(random.uniform(10, 500), 1)
            swap_mb = round(random.uniform(0, 100), 1)
            
            has_gui_window = 1 if process_type == "gui_interactive" else 0
            is_focused_window = 1 if has_gui_window and random.random() < 0.2 else 0
            
            is_audio_client = 1 if process_type == "audio" else 0
            has_active_stream = 1 if is_audio_client and random.random() < 0.5 else 0
            
            # Теги
            tags = []
            if process_type == "cli_interactive":
                tags.append("terminal")
            elif process_type == "gui_interactive":
                tags.append("gui")
            elif process_type == "audio":
                tags.append("audio")
            
            if process_type in ["cli_interactive", "gui_interactive"]:
                tags.append("interactive")
            else:
                tags.append("background")
            
            # Приоритет и оценка
            if process_type == "cli_interactive":
                teacher_priority_class = "INTERACTIVE"
                teacher_score = round(random.uniform(0.6, 0.9), 2)
            elif process_type == "gui_interactive":
                teacher_priority_class = "INTERACTIVE"
                teacher_score = round(random.uniform(0.7, 0.95), 2)
            elif process_type == "audio":
                teacher_priority_class = "LATENCY_CRITICAL"
                teacher_score = round(random.uniform(0.8, 1.0), 2)
            else:
                teacher_priority_class = "BACKGROUND"
                teacher_score = round(random.uniform(0.1, 0.4), 2)
            
            # Вставляем процесс
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
                    snapshot_id,  # snapshot_id
                    pid,  # pid
                    ppid,  # ppid
                    1000,  # uid
                    1000,  # gid
                    f"/usr/bin/{process_type.replace('_', '-')}",  # exe
                    f"{process_type} --arg1 --arg2",  # cmdline
                    f"/user.slice/user-1000.slice/session-{snapshot_idx}.scope",  # cgroup_path
                    None,  # systemd_unit
                    app_group_id,  # app_group_id
                    "R",  # state
                    1000000 + process_idx * 1000,  # start_time
                    random.randint(60, 3600),  # uptime_sec
                    0 if process_type == "background" else random.randint(1, 5),  # tty_nr
                    1 if process_type != "background" else 0,  # has_tty
                    cpu_share_1s,  # cpu_share_1s
                    cpu_share_10s,  # cpu_share_10s
                    io_read_bytes,  # io_read_bytes
                    io_write_bytes,  # io_write_bytes
                    rss_mb,  # rss_mb
                    swap_mb,  # swap_mb
                    random.randint(100, 10000),  # voluntary_ctx
                    random.randint(10, 500),  # involuntary_ctx
                    has_gui_window,  # has_gui_window
                    is_focused_window,  # is_focused_window
                    "normal" if has_gui_window else None,  # window_state
                    1 if process_type != "background" else 0,  # env_has_display
                    1 if process_type == "gui_interactive" else 0,  # env_has_wayland
                    "xterm" if process_type == "cli_interactive" else None,  # env_term
                    0,  # env_ssh
                    is_audio_client,  # is_audio_client
                    has_active_stream,  # has_active_stream
                    process_type,  # process_type
                    json.dumps(tags),  # tags
                    0,  # nice
                    2,  # ionice_class
                    4,  # ionice_prio
                    teacher_priority_class,  # teacher_priority_class
                    teacher_score,  # teacher_score
                ),
            )
            
            process_ids.append(pid)
            
            # Накапливаем данные для группы
            if app_group_id not in app_groups_data:
                app_groups_data[app_group_id] = {
                    'root_pid': root_pid,
                    'process_ids': [pid],
                    'app_name': app_name,
                    'total_cpu_share': cpu_share_1s,
                    'total_io_read_bytes': io_read_bytes,
                    'total_io_write_bytes': io_write_bytes,
                    'total_rss_mb': rss_mb,
                    'has_gui_window': has_gui_window,
                    'is_focused_group': is_focused_window,
                    'tags': tags.copy(),
                    'priority_class': teacher_priority_class
                }
            else:
                app_groups_data[app_group_id]['process_ids'].append(pid)
                app_groups_data[app_group_id]['total_cpu_share'] += cpu_share_1s
                app_groups_data[app_group_id]['total_io_read_bytes'] += io_read_bytes
                app_groups_data[app_group_id]['total_io_write_bytes'] += io_write_bytes
                app_groups_data[app_group_id]['total_rss_mb'] += rss_mb
                if has_gui_window:
                    app_groups_data[app_group_id]['has_gui_window'] = 1
                if is_focused_window:
                    app_groups_data[app_group_id]['is_focused_group'] = 1
        
        # Вставляем группы приложений
        for app_group_id, group_data in app_groups_data.items():
            cursor.execute(
                """
                INSERT INTO app_groups (
                    snapshot_id, app_group_id, root_pid, process_ids,
                    app_name, total_cpu_share, total_io_read_bytes, total_io_write_bytes,
                    total_rss_mb, has_gui_window, is_focused_group, tags, priority_class
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    snapshot_id,
                    app_group_id,
                    group_data['root_pid'],
                    json.dumps(group_data['process_ids']),
                    group_data['app_name'],
                    group_data['total_cpu_share'],
                    group_data['total_io_read_bytes'],
                    group_data['total_io_write_bytes'],
                    group_data['total_rss_mb'],
                    group_data['has_gui_window'],
                    group_data['is_focused_group'],
                    json.dumps(group_data['tags']),
                    group_data['priority_class'],
                ),
            )
    
    # Создаем индексы
    cursor.execute('CREATE INDEX IF NOT EXISTS idx_snapshots_timestamp ON snapshots(timestamp)')
    cursor.execute('CREATE INDEX IF NOT EXISTS idx_processes_pid ON processes(pid)')
    cursor.execute('CREATE INDEX IF NOT EXISTS idx_processes_app_group ON processes(app_group_id)')
    cursor.execute('CREATE INDEX IF NOT EXISTS idx_app_groups_name ON app_groups(app_name)')
    
    conn.commit()
    conn.close()
    
    print(f"Generated training database with {num_snapshots} snapshots and {num_snapshots * processes_per_snapshot} processes")

def main():
    print("=== Generating Synthetic Training Data ===")
    
    db_path = Path("training_data.sqlite")
    
    if db_path.exists():
        db_path.unlink()
    
    # Generate training data
    create_training_database(db_path, num_snapshots=15, processes_per_snapshot=10)
    
    # Validate the generated database
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    
    cursor.execute('SELECT COUNT(*) FROM snapshots')
    snapshot_count = cursor.fetchone()[0]
    
    cursor.execute('SELECT COUNT(*) FROM processes')
    process_count = cursor.fetchone()[0]
    
    cursor.execute('SELECT COUNT(*) FROM app_groups')
    group_count = cursor.fetchone()[0]
    
    cursor.execute('SELECT COUNT(DISTINCT pid) FROM processes')
    unique_processes = cursor.fetchone()[0]
    
    cursor.execute('SELECT COUNT(DISTINCT app_group_id) FROM app_groups')
    unique_groups = cursor.fetchone()[0]
    
    # Check priority classes distribution
    cursor.execute('SELECT priority_class, COUNT(*) FROM app_groups GROUP BY priority_class')
    priority_distribution = cursor.fetchall()
    
    conn.close()
    
    print(f"\nGenerated database statistics:")
    print(f"  Snapshots: {snapshot_count}")
    print(f"  Processes: {process_count}")
    print(f"  Groups: {group_count}")
    print(f"  Unique processes: {unique_processes}")
    print(f"  Unique groups: {unique_groups}")
    print(f"\nPriority class distribution:")
    for priority_class, count in priority_distribution:
        print(f"    {priority_class}: {count} groups")
    
    print("\n✅ Synthetic training data generation completed successfully!")
    return 0

if __name__ == "__main__":
    sys.exit(main())
