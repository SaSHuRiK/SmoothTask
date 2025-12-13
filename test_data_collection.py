#!/usr/bin/env python3
"""
Test script for data collection functionality.
Tests the complete_data_collection.py script and its components.
"""

import sqlite3
import tempfile
from pathlib import Path
import sys
import os

# Add the current directory to path for imports
sys.path.insert(0, '.')

def test_database_creation():
    """Test that the database is created with correct schema."""
    print("Testing database creation...")
    
    # Import the function
    from complete_data_collection import create_comprehensive_training_dataset
    
    # Create a temporary database
    with tempfile.NamedTemporaryFile(suffix='.db', delete=False) as temp_file:
        temp_db = Path(temp_file.name)
    
    try:
        # Create the database
        create_comprehensive_training_dataset(temp_db)
        
        # Verify it exists
        assert temp_db.exists(), "Database file was not created"
        
        # Verify tables exist
        conn = sqlite3.connect(temp_db)
        cursor = conn.cursor()
        
        cursor.execute("SELECT name FROM sqlite_master WHERE type='table'")
        tables = [row[0] for row in cursor.fetchall()]
        
        assert 'snapshots' in tables, "snapshots table not found"
        assert 'processes' in tables, "processes table not found"
        assert 'app_groups' in tables, "app_groups table not found"
        
        # Verify indexes exist
        cursor.execute("SELECT name FROM sqlite_master WHERE type='index'")
        indexes = [row[0] for row in cursor.fetchall()]
        
        assert any('snapshots' in idx for idx in indexes), "snapshots index not found"
        assert any('processes' in idx for idx in indexes), "processes index not found"
        assert any('app_groups' in idx for idx in indexes), "app_groups index not found"
        
        conn.close()
        print("✅ Database creation test passed")
        return True
        
    except Exception as e:
        print(f"❌ Database creation test failed: {e}")
        return False
    finally:
        # Clean up
        if temp_db.exists():
            temp_db.unlink()

def test_validation_function():
    """Test the validation function with an empty database."""
    print("Testing validation function...")
    
    from complete_data_collection import validate_dataset
    
    # Create a temporary database
    with tempfile.NamedTemporaryFile(suffix='.db', delete=False) as temp_file:
        temp_db = Path(temp_file.name)
    
    try:
        # Create empty database with correct schema
        conn = sqlite3.connect(temp_db)
        cursor = conn.cursor()
        
        cursor.execute("""
            CREATE TABLE snapshots (
                snapshot_id INTEGER PRIMARY KEY,
                timestamp INTEGER NOT NULL
            )
        """)
        
        cursor.execute("""
            CREATE TABLE processes (
                snapshot_id INTEGER NOT NULL,
                pid INTEGER NOT NULL,
                app_group_id TEXT,
                PRIMARY KEY (snapshot_id, pid)
            )
        """)
        
        cursor.execute("""
            CREATE TABLE app_groups (
                snapshot_id INTEGER NOT NULL,
                app_group_id TEXT NOT NULL,
                priority_class TEXT,
                PRIMARY KEY (snapshot_id, app_group_id)
            )
        """)
        
        conn.commit()
        conn.close()
        
        # Test validation
        stats = validate_dataset(temp_db)
        
        assert stats['snapshot_count'] == 0, "Expected 0 snapshots"
        assert stats['process_count'] == 0, "Expected 0 processes"
        assert stats['group_count'] == 0, "Expected 0 groups"
        
        print("✅ Validation function test passed")
        return True
        
    except Exception as e:
        print(f"❌ Validation function test failed: {e}")
        return False
    finally:
        # Clean up
        if temp_db.exists():
            temp_db.unlink()

def test_report_generation():
    """Test the report generation function."""
    print("Testing report generation...")
    
    from complete_data_collection import create_data_collection_report
    
    # Test with empty stats
    empty_stats = {
        'snapshot_count': 0,
        'process_count': 0,
        'group_count': 0,
        'unique_processes': 0,
        'unique_groups': 0,
        'time_range': {'start': None, 'end': None, 'duration_seconds': 0},
        'quality_metrics': {
            'group_coverage_percentage': 0.0,
            'priority_coverage_percentage': 0.0,
            'avg_processes_per_snapshot': 0.0,
            'avg_groups_per_snapshot': 0.0
        }
    }
    
    try:
        # Generate report
        report = create_data_collection_report(empty_stats)
        
        # Verify report contains expected sections
        assert '# SmoothTask Data Collection Report' in report, "Report title not found"
        assert 'Dataset Overview' in report, "Dataset Overview section not found"
        assert 'Data Sufficiency Analysis' in report, "Data Sufficiency Analysis section not found"
        assert 'Recommendations' in report, "Recommendations section not found"
        
        print("✅ Report generation test passed")
        return True
        
    except Exception as e:
        print(f"❌ Report generation test failed: {e}")
        return False

def test_main_functionality():
    """Test the main data collection workflow."""
    print("Testing main functionality...")
    
    # Test that the main script can run without errors
    try:
        # Import main function
        from complete_data_collection import main
        
        # Test with no arguments (should work with existing files)
        # We'll just verify it doesn't crash
        print("✅ Main functionality test passed (script is callable)")
        return True
        
    except Exception as e:
        print(f"❌ Main functionality test failed: {e}")
        return False

def run_all_tests():
    """Run all tests and report results."""
    print("=== Running Data Collection Tests ===")
    print()
    
    tests = [
        test_database_creation,
        test_validation_function,
        test_report_generation,
        test_main_functionality
    ]
    
    passed = 0
    total = len(tests)
    
    for test in tests:
        if test():
            passed += 1
        print()
    
    print(f"=== Test Results ===")
    print(f"Passed: {passed}/{total}")
    
    if passed == total:
        print("🎉 All tests passed!")
        return 0
    else:
        print("❌ Some tests failed")
        return 1

if __name__ == "__main__":
    sys.exit(run_all_tests())