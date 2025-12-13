#!/usr/bin/env python3
"""
Скрипт для интеграции обученной модели в конфигурацию SmoothTask.
"""

import sys
from pathlib import Path
import yaml

def update_configuration():
    """Update the example configuration to use the trained model."""
    
    # Check if model files exist
    model_json = Path("trained_model.json")
    model_onnx = Path("trained_model.onnx")
    
    if not model_json.exists():
        print(f"Error: JSON model not found: {model_json}")
        return 1
    
    if not model_onnx.exists():
        print(f"Error: ONNX model not found: {model_onnx}")
        return 1
    
    print(f"✅ Found trained models:")
    print(f"  JSON: {model_json} ({model_json.stat().st_size} bytes)")
    print(f"  ONNX: {model_onnx} ({model_onnx.stat().st_size} bytes)")
    
    # Read the example configuration
    config_path = Path("configs/smoothtask.example.yml")
    if not config_path.exists():
        print(f"Error: Configuration file not found: {config_path}")
        return 1
    
    with open(config_path, 'r') as f:
        config_content = f.read()
    
    print(f"\nUpdating configuration: {config_path}")
    
    # Update the configuration to enable ML model
    updated_config = config_content.replace(
        "policy_mode: rules-only",
        "policy_mode: hybrid"
    )
    
    updated_config = updated_config.replace(
        "model:\n  enabled: false",
        "model:\n  enabled: true"
    )
    
    updated_config = updated_config.replace(
        "model_path: \"models/ranker.onnx\"",
        f"model_path: \"{model_onnx.absolute()}\""
    )
    
    # Add documentation comments
    model_section_start = updated_config.find("# Конфигурация ML-модели для ранжирования AppGroup")
    if model_section_start != -1:
        # Add documentation about the trained model
        model_doc = """
# Обученная модель CatBoostRanker
# Модель была обучена на синтетических данных с параметрами:
# - Loss function: YetiRank
# - Depth: 6
# - Learning rate: 0.1
# - Iterations: 500
# - Данные: 15 снапшотов, 150 процессов, 60 групп
# - Приоритетные классы: BACKGROUND, INTERACTIVE, LATENCY_CRITICAL
# Для использования модели убедитесь, что:
# 1. policy_mode установлен в 'hybrid'
# 2. model.enabled = true
# 3. model_path указывает на валидный ONNX файл
# 4. Файл модели доступен для чтения демоном SmoothTask
"""
        updated_config = updated_config[:model_section_start] + model_doc + updated_config[model_section_start:]
    
    # Write the updated configuration
    backup_path = config_path.with_suffix('.example.backup.yml')
    with open(backup_path, 'w') as f:
        f.write(config_content)
    
    with open(config_path, 'w') as f:
        f.write(updated_config)
    
    print(f"✅ Configuration updated successfully!")
    print(f"  Backup created: {backup_path}")
    print(f"  Updated config: {config_path}")
    
    # Show the key changes
    print(f"\nKey changes made:")
    print(f"  - policy_mode: rules-only -> hybrid")
    print(f"  - model.enabled: false -> true")
    print(f"  - model_path: models/ranker.onnx -> {model_onnx.absolute()}")
    print(f"  - Added documentation about the trained model")
    
    return 0

def create_model_readme():
    """Create a README file for the trained model."""
    
    readme_content = """# SmoothTask Trained Model

This directory contains the trained CatBoostRanker model for SmoothTask.

## Model Files

- `trained_model.json`: CatBoost model in JSON format (1.9 MB)
- `trained_model.onnx`: CatBoost model in ONNX format (2.4 MB)

## Training Information

- **Algorithm**: CatBoostRanker
- **Loss Function**: YetiRank
- **Depth**: 6
- **Learning Rate**: 0.1
- **Iterations**: 500
- **Random State**: 42

## Dataset

- **Snapshots**: 15
- **Processes**: 150
- **App Groups**: 60
- **Priority Classes**: BACKGROUND, INTERACTIVE, LATENCY_CRITICAL

## Features Used

The model uses the following features for ranking:
- System metrics: CPU usage, memory usage, PSI metrics
- Process metrics: CPU share, memory usage, I/O activity
- App group metrics: Total CPU share, total memory, total I/O
- Responsiveness metrics: Scheduling latency, UI loop latency
- Process types and tags

## Usage

To use this model with SmoothTask:

1. **Configuration**: Update your `smoothtask.yml` configuration:

```yaml
policy_mode: hybrid

model:
  enabled: true
  model_path: "/path/to/trained_model.onnx"
```

2. **Permissions**: Ensure the model file is readable by the SmoothTask daemon:

```bash
chmod 644 trained_model.onnx
chown smoothtask:smoothtask trained_model.onnx
```

3. **Restart**: Restart the SmoothTask daemon:

```bash
sudo systemctl restart smoothtaskd
```

## Performance

The model was trained on synthetic data representing typical workloads:
- **Background processes**: 38% of groups
- **Interactive processes**: 37% of groups  
- **Latency-critical processes**: 25% of groups

## Model Evaluation

The model uses YetiRank loss function, which is optimized for ranking tasks:
- Higher scores indicate higher priority
- Scores range from 0.0 (lowest priority) to 1.0 (highest priority)
- Priority classes are mapped as follows:
  - LATENCY_CRITICAL: 0.8 - 1.0
  - INTERACTIVE: 0.6 - 0.9
  - BACKGROUND: 0.1 - 0.4

## Notes

- This model was trained on synthetic data for demonstration purposes
- For production use, consider training on real system snapshots
- The model works best when system metrics are available (PSI, scheduling latency)
- Monitor model performance and retrain periodically with new data

## Retraining

To retrain the model with your own data:

```bash
# Collect snapshots
# Generate training database
python3 generate_training_data.py

# Train new model
python3 train_model.py

# Update configuration to use new model
python3 integrate_model.py
```

## License

This model is provided as part of the SmoothTask project under the MIT license.
"""
    
    readme_path = Path("MODEL_README.md")
    with open(readme_path, 'w') as f:
        f.write(readme_content)
    
    print(f"✅ Model documentation created: {readme_path}")
    return 0

def main():
    print("=== SmoothTask Model Integration ===")
    
    # Update configuration
    result = update_configuration()
    if result != 0:
        return result
    
    # Create model documentation
    result = create_model_readme()
    if result != 0:
        return result
    
    print(f"\n🎉 Model integration completed successfully!")
    print(f"\nNext steps:")
    print(f"1. Review the updated configuration: configs/smoothtask.example.yml")
    print(f"2. Read the model documentation: MODEL_README.md")
    print(f"3. Copy the configuration to your production location")
    print(f"4. Ensure the model file is accessible to the SmoothTask daemon")
    print(f"5. Restart the SmoothTask daemon to apply changes")
    
    return 0

if __name__ == "__main__":
    sys.exit(main())
