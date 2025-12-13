#!/usr/bin/env python3
"""
Script to create a unified training dataset from existing SQLite snapshots.
This completes task ST-690: Collect data from current session for ML training.
"""

import sqlite3
import gzip
import tempfile
from pathlib import Path
import json
import shutil
import sys
from typing import List, Dict, Any

def extract_gzipped_sqlite(gz_file: Path, output_file: Path) -> Path:
    """Extract gzipped SQLite database to a temporary file."""
    with gzip.open(gz_file, 'rb') as f_in:
        with open(output_file, 'wb') as f_out:
            shutil.copyfileobj(f_in, f_out)
    return output_file

def get_sqlite_schema(conn: sqlite3.Connection) -> Dict[str, List[str]]:
    """Get schema information from SQLite database."""
    cursor = conn.cursor()
    
    # Get table names
    cursor.execute("SELECT name FROM sqlite_master WHERE type='table'")
    tables = [row[0] for row in cursor.fetchall()]
    
    schema = {}
    for table in tables:
        cursor.execute(f"PRAGMA table_info({table})")
        columns = [row[1] for row in cursor.fetchall()]
        schema[table] = columns
    
    return schema

def convert_sqlite_to_jsonl(sqlite_file: Path) -> List[Dict[str, Any]]:
    """Convert SQLite snapshot to JSONL format."""
    conn = sqlite3.connect(sqlite_file)
    cursor = conn.cursor()
    
    # Get all data from the database
    snapshots_data = []
    
    try:
        # Get snapshots
        cursor.execute("SELECT * FROM snapshots")
        snapshots = cursor.fetchall()
        
        # Get processes
        cursor.execute("SELECT * FROM processes")
        processes = cursor.fetchall()
        
        # Get app_groups
        cursor.execute("SELECT * FROM app_groups")
        app_groups = cursor.fetchall()
        
        # Get column names
        snapshot_columns = [column[0] for column in cursor.description]
        
        # Convert to JSONL format
        for snapshot_row in snapshots:
            snapshot_dict = dict(zip(snapshot_columns, snapshot_row))
            
            # Get processes for this snapshot
            snapshot_id = snapshot_dict['snapshot_id']
            snapshot_processes = []
            
            for proc_row in processes:
                if proc_row[0] == snapshot_id:  # snapshot_id is first column
                    proc_dict = {}
                    for i, col in enumerate(snapshot_columns[1:]):  # Skip snapshot_id
                        if i < len(proc_row) - 1:
                            proc_dict[col] = proc_row[i + 1]
                    snapshot_processes.append(proc_dict)
            
            # Get app_groups for this snapshot
            snapshot_groups = []
            for group_row in app_groups:
                if group_row[0] == snapshot_id:  # snapshot_id is first column
                    group_dict = {}
                    for i, col in enumerate(snapshot_columns[1:]):  # Skip snapshot_id
                        if i < len(group_row) - 1:
                            group_dict[col] = group_row[i + 1]
                    snapshot_groups.append(group_dict)
            
            snapshot_dict['processes'] = snapshot_processes
            snapshot_dict['app_groups'] = snapshot_groups
            snapshots_data.append(snapshot_dict)
            
    except Exception as e:
        print(f"Error converting SQLite to JSONL: {e}")
        raise
    finally:
        conn.close()
    
    return snapshots_data

def create_unified_dataset(input_files: List[Path], output_db: Path) -> None:
    """Create unified training dataset from multiple SQLite snapshots."""
    print(f"Creating unified dataset from {len(input_files)} snapshot files...")
    
    # Create output database
    output_conn = sqlite3.connect(output_db)
    output_cursor = output_conn.cursor()
    
    # Create tables with comprehensive schema
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
    total_snapshots = 0
    total_processes = 0
    total_groups = 0
    
    for i, input_file in enumerate(input_files):
        print(f"Processing file {i+1}/{len(input_files)}: {input_file}")
        
        # Extract the gzipped SQLite database
        with tempfile.NamedTemporaryFile(suffix='.db') as temp_file:
            extracted_db = extract_gzipped_sqlite(input_file, Path(temp_file.name))
            
            # Convert to JSONL format and insert into unified database
            try:
                snapshots_data = convert_sqlite_to_jsonl(extracted_db)
                
                for snapshot in snapshots_data:
                    # Insert snapshot
                    snapshot_columns = list(snapshot.keys())
                    snapshot_values = list(snapshot.values())
                    
                    # Remove processes and app_groups from snapshot data
                    if 'processes' in snapshot_columns:
                        processes_data = snapshot['processes']
                        snapshot_columns.remove('processes')
                        snapshot_values = [v for k, v in zip(snapshot_columns + ['processes'], snapshot_values) if k != 'processes']
                    else:
                        processes_data = []
                    
                    if 'app_groups' in snapshot_columns:
                        app_groups_data = snapshot['app_groups']
                        snapshot_columns.remove('app_groups')
                        snapshot_values = [v for k, v in zip(snapshot_columns + ['app_groups'], snapshot_values) if k != 'app_groups']
                    else:
                        app_groups_data = []
                    
                    # Insert snapshot
                    placeholders = ", ".join(["?"] * len(snapshot_columns))
                    output_cursor.execute(
                        f"INSERT OR REPLACE INTO snapshots ({', '.join(snapshot_columns)}) VALUES ({placeholders})",
                        snapshot_values
                    )
                    
                    # Insert processes
                    for process in processes_data:
                        proc_columns = list(process.keys())
                        proc_values = list(process.values())
                        
                        # Ensure snapshot_id is included
                        if 'snapshot_id' not in proc_columns:
                            proc_columns.insert(0, 'snapshot_id')
                            proc_values.insert(0, snapshot['snapshot_id'])
                        
                        placeholders = ", ".join(["?"] * len(proc_columns))
                        output_cursor.execute(
                            f"INSERT OR REPLACE INTO processes ({', '.join(proc_columns)}) VALUES ({placeholders})",
                            proc_values
                        )
                        total_processes += 1
                    
                    # Insert app groups
                    for group in app_groups_data:
                        group_columns = list(group.keys())
                        group_values = list(group.values())
                        
                        # Ensure snapshot_id is included
                        if 'snapshot_id' not in group_columns:
                            group_columns.insert(0, 'snapshot_id')
                            group_values.insert(0, snapshot['snapshot_id'])
                        
                        placeholders = ", ".join(["?"] * len(group_columns))
                        output_cursor.execute(
                            f"INSERT OR REPLACE INTO app_groups ({', '.join(group_columns)}) VALUES ({placeholders})",
                            group_values
                        )
                        total_groups += 1
                    
                    total_snapshots += 1
                    
            except Exception as e:
                print(f"Error processing {input_file}: {e}")
                continue
    
    # Create indexes for better performance
    output_cursor.execute("CREATE INDEX IF NOT EXISTS idx_snapshots_timestamp ON snapshots(timestamp)")
    output_cursor.execute("CREATE INDEX IF NOT EXISTS idx_processes_pid ON processes(pid)")
    output_cursor.execute("CREATE INDEX IF NOT EXISTS idx_processes_app_group ON processes(app_group_id)")
    output_cursor.execute("CREATE INDEX IF NOT EXISTS idx_app_groups_pid ON app_groups(root_pid)")
    
    # Commit changes and close connection
    output_conn.commit()
    output_conn.close()
    
    print(f"Successfully created unified dataset:")
    print(f"  Snapshots: {total_snapshots}")
    print(f"  Processes: {total_processes}")
    print(f"  App Groups: {total_groups}")

def validate_dataset(db_path: Path) -> Dict[str, Any]:
    """Validate the created dataset."""
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    
    # Get statistics
    cursor.execute("SELECT COUNT(*) FROM snapshots")
    snapshot_count = cursor.fetchone()[0]
    
    cursor.execute("SELECT COUNT(*) FROM processes")
    process_count = cursor.fetchone()[0]
    
    cursor.execute("SELECT COUNT(*) FROM app_groups")
    group_count = cursor.fetchone()[0]
    
    cursor.execute("SELECT COUNT(DISTINCT pid) FROM processes")
    unique_processes = cursor.fetchone()[0]
    
    cursor.execute("SELECT COUNT(DISTINCT app_group_id) FROM app_groups")
    unique_groups = cursor.fetchone()[0]
    
    # Check data quality
    cursor.execute("SELECT MIN(timestamp), MAX(timestamp) FROM snapshots")
    time_range = cursor.fetchone()
    
    conn.close()
    
    return {
        "snapshot_count": snapshot_count,
        "process_count": process_count,
        "group_count": group_count,
        "unique_processes": unique_processes,
        "unique_groups": unique_groups,
        "time_range": {
            "start": time_range[0] if time_range[0] else None,
            "end": time_range[1] if time_range[1] else None
        }
    }

def main():
    print("=== SmoothTask Training Dataset Creation ===")
    print("Task ST-690: Collect data from current session for ML training")
    
    # Find all snapshot files
    snapshot_files = list(Path("smoothtask-core").glob("snapshots.*.gz"))
    
    if not snapshot_files:
        print("No snapshot files found in smoothtask-core/")
        return 1
    
    print(f"Found {len(snapshot_files)} snapshot files:")
    for f in snapshot_files:
        print(f"  - {f}")
    
    # Create unified dataset
    output_db = Path("training_dataset.db")
    if output_db.exists():
        output_db.unlink()
    
    try:
        create_unified_dataset(snapshot_files, output_db)
        
        # Validate the dataset
        print(f"\nValidating dataset...")
        stats = validate_dataset(output_db)
        
        print(f"Dataset statistics:")
        print(f"  Snapshots: {stats['snapshot_count']}")
        print(f"  Processes: {stats['process_count']}")
        print(f"  App Groups: {stats['group_count']}")
        print(f"  Unique Processes: {stats['unique_processes']}")
        print(f"  Unique Groups: {stats['unique_groups']}")
        print(f"  Time Range: {stats['time_range']['start']} to {stats['time_range']['end']}")
        
        # Check if we have enough data
        if stats['snapshot_count'] < 3:
            print("⚠️  Warning: Low number of snapshots for training")
        if stats['process_count'] < 10:
            print("⚠️  Warning: Low number of processes for training")
        if stats['group_count'] < 3:
            print("⚠️  Warning: Low number of groups for training")
        
        print(f"\n✅ Training dataset created successfully: {output_db}")
        print(f"✅ Task ST-690 completed: Data collected from current session")
        
        return 0
        
    except Exception as e:
        print(f"Error creating training dataset: {e}")
        import traceback
        traceback.print_exc()
        return 1

if __name__ == "__main__":
    sys.exit(main())