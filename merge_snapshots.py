#!/usr/bin/env python3
"""
Script to merge multiple snapshot files into a single database for ML training.
"""

import sqlite3
import gzip
import os
import tempfile
from pathlib import Path

def merge_snapshots(input_files, output_file):
    """
    Merge multiple snapshot SQLite databases into a single database.
    
    Args:
        input_files: List of input snapshot files (gzipped SQLite databases)
        output_file: Path to the output merged database
    """
    print(f"Merging {len(input_files)} snapshot files into {output_file}")
    
    # Create output database
    output_conn = sqlite3.connect(output_file)
    output_cursor = output_conn.cursor()
    
    # Create tables if they don't exist
    output_cursor.execute("""
        CREATE TABLE IF NOT EXISTS snapshots (
            snapshot_id INTEGER PRIMARY KEY,
            timestamp INTEGER NOT NULL,
            uptime_seconds REAL,
            load_avg_1 REAL,
            load_avg_5 REAL,
            load_avg_15 REAL,
            cpu_usage_pct REAL,
            mem_total_mb INTEGER,
            mem_free_mb INTEGER,
            mem_available_mb INTEGER,
            swap_total_mb INTEGER,
            swap_free_mb INTEGER,
            psi_cpu_some_avg10 REAL,
            psi_cpu_some_avg60 REAL,
            psi_cpu_some_avg300 REAL,
            psi_cpu_full_avg10 REAL,
            psi_cpu_full_avg60 REAL,
            psi_cpu_full_avg300 REAL,
            psi_io_some_avg10 REAL,
            psi_io_some_avg60 REAL,
            psi_io_some_avg300 REAL,
            psi_io_full_avg10 REAL,
            psi_io_full_avg60 REAL,
            psi_io_full_avg300 REAL,
            psi_mem_some_avg10 REAL,
            psi_mem_some_avg60 REAL,
            psi_mem_some_avg300 REAL,
            psi_mem_full_avg10 REAL,
            psi_mem_full_avg60 REAL,
            psi_mem_full_avg300 REAL
        )
    """)
    
    output_cursor.execute("""
        CREATE TABLE IF NOT EXISTS processes (
            snapshot_id INTEGER NOT NULL,
            pid INTEGER NOT NULL,
            ppid INTEGER,
            uid INTEGER,
            gid INTEGER,
            name TEXT,
            cmdline TEXT,
            exe TEXT,
            cwd TEXT,
            root TEXT,
            status TEXT,
            cpu_share_1s REAL,
            cpu_share_5s REAL,
            cpu_share_15s REAL,
            mem_rss_mb REAL,
            mem_vms_mb REAL,
            io_read_bytes INTEGER,
            io_write_bytes INTEGER,
            threads INTEGER,
            nice INTEGER,
            ionice_class INTEGER,
            ionice_level INTEGER,
            latency_nice INTEGER,
            cgroup_path TEXT,
            app_group_id TEXT,
            is_foreground INTEGER,
            tags TEXT,
            PRIMARY KEY (snapshot_id, pid),
            FOREIGN KEY (snapshot_id) REFERENCES snapshots(snapshot_id)
        )
    """)
    
    output_cursor.execute("""
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
    """)
    
    # Process each input file
    for i, input_file in enumerate(input_files):
        print(f"Processing file {i+1}/{len(input_files)}: {input_file}")
        
        # Open the gzipped SQLite database
        with gzip.open(input_file, 'rb') as gz_file:
            # Create a temporary file to extract the SQLite database
            with tempfile.NamedTemporaryFile(suffix='.db') as temp_file:
                temp_file.write(gz_file.read())
                temp_file.flush()
                
                # Connect to the temporary database
                temp_conn = sqlite3.connect(temp_file.name)
                temp_cursor = temp_conn.cursor()
                
                # Copy data from temporary database to output database
                
                # Copy snapshots
                temp_cursor.execute("SELECT * FROM snapshots")
                snapshots_data = temp_cursor.fetchall()
                if snapshots_data:
                    output_cursor.executemany(
                        "INSERT OR IGNORE INTO snapshots VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
                        snapshots_data
                    )
                    print(f"  Copied {len(snapshots_data)} snapshots")
                
                # Copy processes
                temp_cursor.execute("SELECT * FROM processes")
                processes_data = temp_cursor.fetchall()
                if processes_data:
                    output_cursor.executemany(
                        "INSERT OR IGNORE INTO processes VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
                        processes_data
                    )
                    print(f"  Copied {len(processes_data)} processes")
                
                # Copy app_groups
                temp_cursor.execute("SELECT * FROM app_groups")
                app_groups_data = temp_cursor.fetchall()
                if app_groups_data:
                    output_cursor.executemany(
                        "INSERT OR IGNORE INTO app_groups VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
                        app_groups_data
                    )
                    print(f"  Copied {len(app_groups_data)} app groups")
                
                temp_conn.close()
    
    # Create indexes for better performance
    output_cursor.execute("CREATE INDEX IF NOT EXISTS idx_snapshots_timestamp ON snapshots(timestamp)")
    output_cursor.execute("CREATE INDEX IF NOT EXISTS idx_processes_pid ON processes(pid)")
    output_cursor.execute("CREATE INDEX IF NOT EXISTS idx_processes_app_group ON processes(app_group_id)")
    output_cursor.execute("CREATE INDEX IF NOT EXISTS idx_app_groups_pid ON app_groups(root_pid)")
    
    # Commit changes and close connection
    output_conn.commit()
    output_conn.close()
    
    print(f"Successfully merged {len(input_files)} snapshot files into {output_file}")

def main():
    # Find all snapshot files
    snapshot_files = list(Path("smoothtask-core").glob("snapshots.*.gz"))
    
    if not snapshot_files:
        print("No snapshot files found in smoothtask-core/")
        return
    
    print(f"Found {len(snapshot_files)} snapshot files:")
    for f in snapshot_files:
        print(f"  - {f}")
    
    # Merge snapshots into a single database
    output_db = "merged_snapshots.db"
    merge_snapshots(snapshot_files, output_db)
    
    # Verify the merged database
    conn = sqlite3.connect(output_db)
    cursor = conn.cursor()
    
    cursor.execute("SELECT COUNT(*) FROM snapshots")
    snapshot_count = cursor.fetchone()[0]
    
    cursor.execute("SELECT COUNT(*) FROM processes")
    process_count = cursor.fetchone()[0]
    
    cursor.execute("SELECT COUNT(*) FROM app_groups")
    app_group_count = cursor.fetchone()[0]
    
    print(f"\nMerged database statistics:")
    print(f"  Snapshots: {snapshot_count}")
    print(f"  Processes: {process_count}")
    print(f"  App Groups: {app_group_count}")
    
    conn.close()

if __name__ == "__main__":
    main()