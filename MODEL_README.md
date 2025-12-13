# SmoothTask Trained Model

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
