#!/usr/bin/env python3
"""
Скрипт для объединения нескольких SQLite баз данных снапшотов в одну.
"""

import sqlite3
import gzip
import shutil
from pathlib import Path
import tempfile
import sys

def extract_gzipped_sqlite(gz_file: Path, output_file: Path):
    """Extract gzipped SQLite database to a temporary file."""
    with gzip.open(gz_file, 'rb') as f_in:
        with open(output_file, 'wb') as f_out:
            shutil.copyfileobj(f_in, f_out)
    return output_file

def merge_sqlite_databases(input_files: list[Path], output_file: Path):
    """Merge multiple SQLite databases into one."""
    # Create output database
    output_conn = sqlite3.connect(output_file)
    output_cursor = output_conn.cursor()
    
    # Create tables if they don't exist
    output_cursor.execute('''
        CREATE TABLE IF NOT EXISTS snapshots (
            snapshot_id INTEGER PRIMARY KEY,
            timestamp TEXT NOT NULL,
            system_cpu_usage REAL,
            system_memory_usage REAL,
            system_io_wait REAL,
            system_load_avg REAL,
            psi_cpu_some REAL,
            psi_cpu_full REAL,
            psi_io_some REAL,
            psi_io_full REAL,
            total_processes INTEGER,
            total_groups INTEGER
        )
    ''')
    
    output_cursor.execute('''
        CREATE TABLE IF NOT EXISTS processes (
            snapshot_id INTEGER NOT NULL,
            pid INTEGER NOT NULL,
            ppid INTEGER,
            cmdline TEXT,
            exe TEXT,
            cpu_usage REAL,
            memory_rss_mb REAL,
            memory_vms_mb REAL,
            io_read_bytes INTEGER,
            io_write_bytes INTEGER,
            threads INTEGER,
            nice INTEGER,
            ionice_class INTEGER,
            ionice_priority INTEGER,
            cgroup_path TEXT,
            app_group_id TEXT,
            PRIMARY KEY (snapshot_id, pid),
            FOREIGN KEY (snapshot_id) REFERENCES snapshots(snapshot_id)
        )
    ''')
    
    output_cursor.execute('''
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
    ''')
    
    # Copy data from each input database
    for input_file in input_files:
        print(f"Processing: {input_file}")
        input_conn = sqlite3.connect(input_file)
        input_cursor = input_conn.cursor()
        
        # Get max snapshot_id to avoid conflicts
        input_cursor.execute('SELECT MAX(snapshot_id) FROM snapshots')
        max_snapshot_id = input_cursor.fetchone()[0] or 0
        
        # Copy snapshots
        input_cursor.execute('SELECT * FROM snapshots')
        snapshots = input_cursor.fetchall()
        if snapshots:
            output_cursor.executemany('INSERT INTO snapshots VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)', snapshots)
        
        # Copy processes
        input_cursor.execute('SELECT * FROM processes')
        processes = input_cursor.fetchall()
        if processes:
            output_cursor.executemany('INSERT INTO processes VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)', processes)
        
        # Copy app_groups
        input_cursor.execute('SELECT * FROM app_groups')
        app_groups = input_cursor.fetchall()
        if app_groups:
            output_cursor.executemany('INSERT INTO app_groups VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)', app_groups)
        
        input_conn.close()
        print(f"  Copied {len(snapshots)} snapshots, {len(processes)} processes, {len(app_groups)} groups")
    
    # Create indexes
    output_cursor.execute('CREATE INDEX IF NOT EXISTS idx_snapshots_timestamp ON snapshots(timestamp)')
    output_cursor.execute('CREATE INDEX IF NOT EXISTS idx_processes_pid ON processes(pid)')
    output_cursor.execute('CREATE INDEX IF NOT EXISTS idx_processes_app_group ON processes(app_group_id)')
    output_cursor.execute('CREATE INDEX IF NOT EXISTS idx_app_groups_name ON app_groups(app_name)')
    
    output_conn.commit()
    output_conn.close()
    
    print(f"Merged database saved to: {output_file}")

def main():
    print("=== SQLite Database Merger ===")
    
    # Snapshot files to process
    snapshot_files = [
        Path('smoothtask-core/snapshots.20251212_063351.gz'),
        Path('smoothtask-core/snapshots.20251212_070818.gz'),
        Path('smoothtask-core/snapshots.20251212_071113.gz')
    ]
    
    output_file = Path("training_data.sqlite")
    
    if output_file.exists():
        output_file.unlink()
    
    # Extract and merge databases
    with tempfile.TemporaryDirectory() as temp_dir:
        temp_dir_path = Path(temp_dir)
        temp_sqlite_files = []
        
        # Extract gzipped databases
        for i, gz_file in enumerate(snapshot_files):
            if gz_file.exists():
                output_file = temp_dir_path / f"snapshot_{i}.sqlite"
                extracted = extract_gzipped_sqlite(gz_file, output_file)
                temp_sqlite_files.append(extracted)
                print(f"Extracted: {gz_file.name} -> {extracted.name}")
            else:
                print(f"Warning: {gz_file} not found")
        
        if not temp_sqlite_files:
            print("Error: No valid snapshot files found")
            return 1
        
        # Merge databases
        merge_sqlite_databases(temp_sqlite_files, Path("training_data.sqlite"))
        
        # Validate the merged database
        conn = sqlite3.connect("training_data.sqlite")
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
        
        conn.close()
        
        print(f"\nMerged database statistics:")
        print(f"  Snapshots: {snapshot_count}")
        print(f"  Processes: {process_count}")
        print(f"  Groups: {group_count}")
        print(f"  Unique processes: {unique_processes}")
        print(f"  Unique groups: {unique_groups}")
        
        print("\n✅ Database merging completed successfully!")
        return 0

if __name__ == "__main__":
    sys.exit(main())
