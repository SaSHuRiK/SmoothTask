#!/usr/bin/env python3
"""
Test configuration validation using JSON Schema.

This test module validates SmoothTask configuration files against the JSON Schema
to ensure they conform to the expected structure and constraints.
"""

import json
import yaml
import jsonschema
from pathlib import Path

# Paths to schema and config files
SCHEMA_PATH = Path(__file__).parent.parent.parent / "configs" / "api-config-schema.json"
EXAMPLE_CONFIG_PATH = Path(__file__).parent.parent.parent / "configs" / "smoothtask.example.yml"


def load_schema():
    """Load JSON Schema from file."""
    with open(SCHEMA_PATH, 'r', encoding='utf-8') as f:
        return json.load(f)


def load_config_yaml(path):
    """Load YAML configuration file."""
    with open(path, 'r', encoding='utf-8') as f:
        return yaml.safe_load(f)


def test_schema_exists():
    """Test that the JSON Schema file exists."""
    assert SCHEMA_PATH.exists(), f"Schema file not found at {SCHEMA_PATH}"
    assert SCHEMA_PATH.is_file(), f"{SCHEMA_PATH} is not a file"


def test_example_config_exists():
    """Test that the example configuration file exists."""
    assert EXAMPLE_CONFIG_PATH.exists(), f"Example config file not found at {EXAMPLE_CONFIG_PATH}"
    assert EXAMPLE_CONFIG_PATH.is_file(), f"{EXAMPLE_CONFIG_PATH} is not a file"


def test_schema_valid():
    """Test that the JSON Schema itself is valid."""
    schema = load_schema()
    assert isinstance(schema, dict), "Schema should be a dictionary"
    assert "$schema" in schema, "Schema should have $schema field"
    assert "properties" in schema, "Schema should have properties field"


def test_example_config_valid():
    """Test that the example configuration is valid according to the schema."""
    schema = load_schema()
    config = load_config_yaml(EXAMPLE_CONFIG_PATH)
    
    # Validate the configuration against the schema
    try:
        jsonschema.validate(instance=config, schema=schema)
        print("✓ Example configuration is valid according to the schema")
    except jsonschema.ValidationError as e:
        raise AssertionError(f"Example configuration validation failed: {e.message}")


def test_config_structure():
    """Test that the example configuration has the expected structure."""
    config = load_config_yaml(EXAMPLE_CONFIG_PATH)
    
    # Check top-level fields
    assert "polling_interval_ms" in config
    assert "max_candidates" in config
    assert "dry_run_default" in config
    assert "policy_mode" in config
    assert "enable_snapshot_logging" in config
    assert "thresholds" in config
    assert "paths" in config
    assert "notifications" in config
    assert "model" in config
    
    # Check thresholds structure
    thresholds = config["thresholds"]
    assert "psi_cpu_some_high" in thresholds
    assert "psi_io_some_high" in thresholds
    assert "user_idle_timeout_sec" in thresholds
    assert "interactive_build_grace_sec" in thresholds
    assert "noisy_neighbour_cpu_share" in thresholds
    assert "crit_interactive_percentile" in thresholds
    assert "interactive_percentile" in thresholds
    assert "normal_percentile" in thresholds
    assert "background_percentile" in thresholds
    assert "sched_latency_p99_threshold_ms" in thresholds
    assert "ui_loop_p95_threshold_ms" in thresholds
    
    # Check paths structure
    paths = config["paths"]
    assert "snapshot_db_path" in paths
    assert "patterns_dir" in paths
    
    # Check notifications structure
    notifications = config["notifications"]
    assert "enabled" in notifications
    assert "backend" in notifications
    assert "app_name" in notifications
    assert "min_level" in notifications
    
    # Check model structure
    model = config["model"]
    assert "model_path" in model
    assert "enabled" in model


def test_config_types():
    """Test that the example configuration has correct data types."""
    config = load_config_yaml(EXAMPLE_CONFIG_PATH)
    
    # Check top-level types
    assert isinstance(config["polling_interval_ms"], int)
    assert isinstance(config["max_candidates"], int)
    assert isinstance(config["dry_run_default"], bool)
    assert isinstance(config["policy_mode"], str)
    assert isinstance(config["enable_snapshot_logging"], bool)
    assert isinstance(config["thresholds"], dict)
    assert isinstance(config["paths"], dict)
    assert isinstance(config["notifications"], dict)
    assert isinstance(config["model"], dict)
    
    # Check threshold types
    thresholds = config["thresholds"]
    assert isinstance(thresholds["psi_cpu_some_high"], (int, float))
    assert isinstance(thresholds["psi_io_some_high"], (int, float))
    assert isinstance(thresholds["user_idle_timeout_sec"], int)
    assert isinstance(thresholds["interactive_build_grace_sec"], int)
    assert isinstance(thresholds["noisy_neighbour_cpu_share"], (int, float))
    assert isinstance(thresholds["crit_interactive_percentile"], (int, float))
    assert isinstance(thresholds["interactive_percentile"], (int, float))
    assert isinstance(thresholds["normal_percentile"], (int, float))
    assert isinstance(thresholds["background_percentile"], (int, float))
    assert isinstance(thresholds["sched_latency_p99_threshold_ms"], (int, float))
    assert isinstance(thresholds["ui_loop_p95_threshold_ms"], (int, float))
    
    # Check path types
    paths = config["paths"]
    assert isinstance(paths["snapshot_db_path"], str)
    assert isinstance(paths["patterns_dir"], str)
    
    # Check notification types
    notifications = config["notifications"]
    assert isinstance(notifications["enabled"], bool)
    assert isinstance(notifications["backend"], str)
    assert isinstance(notifications["app_name"], str)
    assert isinstance(notifications["min_level"], str)
    
    # Check model types
    model = config["model"]
    assert isinstance(model["model_path"], str)
    assert isinstance(model["enabled"], bool)


def test_invalid_config():
    """Test that invalid configurations are properly rejected."""
    schema = load_schema()
    
    # Test with missing required field
    invalid_config = {
        "max_candidates": 150,
        "dry_run_default": False,
        "policy_mode": "rules-only",
        "enable_snapshot_logging": True,
        "thresholds": {
            "psi_cpu_some_high": 0.6,
            "psi_io_some_high": 0.4,
            "user_idle_timeout_sec": 120,
            "interactive_build_grace_sec": 10,
            "noisy_neighbour_cpu_share": 0.7,
            "crit_interactive_percentile": 0.9,
            "interactive_percentile": 0.6,
            "normal_percentile": 0.3,
            "background_percentile": 0.1,
            "sched_latency_p99_threshold_ms": 20.0,
            "ui_loop_p95_threshold_ms": 16.67
        },
        "paths": {
            "snapshot_db_path": "/var/lib/smoothtask/snapshots.db",
            "patterns_dir": "/etc/smoothtask/patterns"
        },
        "notifications": {
            "enabled": False,
            "backend": "stub",
            "app_name": "SmoothTask",
            "min_level": "warning"
        },
        "model": {
            "model_path": "models/ranker.onnx",
            "enabled": False
        }
    }
    
    # This should fail because polling_interval_ms is missing
    try:
        jsonschema.validate(instance=invalid_config, schema=schema)
        raise AssertionError("Expected ValidationError but validation passed")
    except jsonschema.ValidationError:
        # Expected exception
        pass


def test_config_ranges():
    """Test that configuration values are within expected ranges."""
    config = load_config_yaml(EXAMPLE_CONFIG_PATH)
    
    # Check polling_interval_ms range
    assert 100 <= config["polling_interval_ms"] <= 10000, \
        f"polling_interval_ms {config['polling_interval_ms']} is out of range [100, 10000]"
    
    # Check max_candidates range
    assert 10 <= config["max_candidates"] <= 1000, \
        f"max_candidates {config['max_candidates']} is out of range [10, 1000]"
    
    # Check threshold ranges
    thresholds = config["thresholds"]
    assert 0 <= thresholds["psi_cpu_some_high"] <= 1, \
        f"psi_cpu_some_high {thresholds['psi_cpu_some_high']} is out of range [0, 1]"
    assert 0 <= thresholds["psi_io_some_high"] <= 1, \
        f"psi_io_some_high {thresholds['psi_io_some_high']} is out of range [0, 1]"
    assert 30 <= thresholds["user_idle_timeout_sec"] <= 600, \
        f"user_idle_timeout_sec {thresholds['user_idle_timeout_sec']} is out of range [30, 600]"
    assert 5 <= thresholds["interactive_build_grace_sec"] <= 60, \
        f"interactive_build_grace_sec {thresholds['interactive_build_grace_sec']} is out of range [5, 60]"
    assert 0.1 <= thresholds["noisy_neighbour_cpu_share"] <= 1, \
        f"noisy_neighbour_cpu_share {thresholds['noisy_neighbour_cpu_share']} is out of range [0.1, 1]"


if __name__ == "__main__":
    # Run tests
    print("Running configuration validation tests...")
    
    print("\n1. Testing schema existence...")
    test_schema_exists()
    print("✓ Schema file exists")
    
    print("\n2. Testing example config existence...")
    test_example_config_exists()
    print("✓ Example config file exists")
    
    print("\n3. Testing schema validity...")
    test_schema_valid()
    print("✓ Schema is valid")
    
    print("\n4. Testing example config validity...")
    test_example_config_valid()
    print("✓ Example config is valid")
    
    print("\n5. Testing config structure...")
    test_config_structure()
    print("✓ Config structure is correct")
    
    print("\n6. Testing config types...")
    test_config_types()
    print("✓ Config types are correct")
    
    print("\n7. Testing config ranges...")
    test_config_ranges()
    print("✓ Config values are within expected ranges")
    
    print("\n8. Testing invalid config...")
    test_invalid_config()
    print("✓ Invalid config is properly rejected")
    
    print("\n" + "="*50)
    print("All configuration validation tests passed! ✓")
    print("="*50)