#!/usr/bin/env python3
"""
Скрипт для сбора данных из существующих снапшотов и обучения модели.
"""

import tempfile
import gzip
import shutil
from pathlib import Path
import sqlite3
import sys
import os

# Add the trainer to path
sys.path.insert(0, 'smoothtask-trainer')

from smoothtask_trainer.collect_data import collect_data_from_snapshots, validate_dataset
from smoothtask_trainer.train_pipeline import TrainingPipeline

def extract_gzipped_sqlite(gz_file: Path, output_file: Path):
    """Extract gzipped SQLite database to a temporary file."""
    with gzip.open(gz_file, 'rb') as f_in:
        with open(output_file, 'wb') as f_out:
            shutil.copyfileobj(f_in, f_out)
    return output_file

def main():
    print("=== SmoothTask ML Training Pipeline ===")
    
    # Step 1: Collect available snapshot files
    snapshot_files = [
        Path('smoothtask-core/snapshots.20251212_063351.gz'),
        Path('smoothtask-core/snapshots.20251212_070818.gz'),
        Path('smoothtask-core/snapshots.20251212_071113.gz')
    ]
    
    print(f"Found {len(snapshot_files)} snapshot files")
    
    # Step 2: Extract and prepare temporary SQLite files
    temp_sqlite_files = []
    with tempfile.TemporaryDirectory() as temp_dir:
        temp_dir_path = Path(temp_dir)
        
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
        
        # Step 3: Create training database
        training_db = Path("training_data.sqlite")
        if training_db.exists():
            training_db.unlink()
        
        print(f"\nCreating training database: {training_db}")
        
        try:
            # Collect data directly using the function
            from smoothtask_trainer.collect_data import collect_data_from_snapshots
            
            db_path = collect_data_from_snapshots(
                snapshot_files=temp_sqlite_files,
                output_db=training_db,
                use_temp_db=False
            )
            print(f"Data collected to: {db_path}")
            
            # Validate dataset
            stats = validate_dataset(
                db_path=db_path,
                min_snapshots=1,
                min_processes=5,
                min_groups=1
            )
            print(f"\nDataset validation:")
            print(f"  Snapshots: {stats['snapshot_count']}")
            print(f"  Processes: {stats['process_count']}")
            print(f"  Groups: {stats['group_count']}")
            print(f"  Unique processes: {stats['unique_processes']}")
            print(f"  Unique groups: {stats['unique_groups']}")
            
            # Check if we have enough data
            if stats['snapshot_count'] < 3:
                print("Warning: Low number of snapshots for training")
            if stats['process_count'] < 10:
                print("Warning: Low number of processes for training")
            if stats['group_count'] < 3:
                print("Warning: Low number of groups for training")
            
            # Copy to final location
            if db_path != training_db:
                shutil.copy2(db_path, training_db)
                print(f"Training database saved to: {training_db}")
            
            print("\n✅ Data collection completed successfully!")
            return 0
            
        except Exception as e:
            print(f"Error during data collection: {e}")
            import traceback
            traceback.print_exc()
            return 1

if __name__ == "__main__":
    sys.exit(main())
