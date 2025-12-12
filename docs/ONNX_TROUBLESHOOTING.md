# Troubleshooting ONNX Integration

## Common Issues and Solutions

### 1. Model Loading Errors

#### Error: `File not found: models/ranker.onnx`

**Cause:** The ONNX model file is missing or the path is incorrect.

**Solution:**
```bash
# Check if the file exists
ls -la models/ranker.onnx

# If missing, train and export the model
cd smoothtask-trainer
python -m smoothtask_trainer.train_ranker \
    --db snapshots.db \
    --model-json models/ranker.json \
    --model-onnx models/ranker.onnx

# Verify the file was created
ls -la models/ranker.onnx
```

#### Error: `Invalid ONNX model format`

**Cause:** The ONNX file is corrupted or has an incompatible format.

**Solution:**
```bash
# Check the ONNX file structure
python -c "
import onnx
model = onnx.load('models/ranker.onnx')
print('Inputs:', [input.name for input in model.graph.input])
print('Outputs:', [output.name for output in model.graph.output])
print('IR version:', model.ir_version)
"

# Retrain and re-export the model
python -m smoothtask_trainer.train_ranker \
    --db snapshots.db \
    --model-json models/ranker_new.json \
    --model-onnx models/ranker_new.onnx

# Replace the old model
mv models/ranker_new.onnx models/ranker.onnx
```

### 2. Runtime Errors

#### Error: `Shape mismatch in ONNX model`

**Cause:** The number of features in the input data doesn't match the model's expected input size.

**Solution:**
```bash
# Check the model's expected input shape
python -c "
import onnx
model = onnx.load('models/ranker.onnx')
input_shape = model.graph.input[0].type.tensor_type.shape
print('Expected input shape:', input_shape)
"

# Check the actual features being generated
# Add debug logging to see the feature vector size

# Retrain the model with the correct feature set
```

#### Error: `ONNX Runtime error during inference`

**Cause:** ONNX Runtime version mismatch or system compatibility issues.

**Solution:**
```bash
# Check ONNX Runtime version
pip show onnxruntime

# Update ONNX Runtime
pip install --upgrade onnxruntime

# For Rust, check the ort crate version
cargo tree | grep ort

# Update Rust dependencies
cargo update -p ort
```

### 3. Performance Issues

#### Issue: Slow ONNX model execution

**Diagnosis:**
```bash
# Check CPU usage during inference
top -p $(pgrep smoothtaskd)

# Check model complexity
python -c "
import onnx
model = onnx.load('models/ranker.onnx')
print('Model size:', len(model.SerializeToString()), 'bytes')
"
```

**Solutions:**

1. **Reduce model complexity:**
   ```bash
   # Retrain with smaller parameters
   python -m smoothtask_trainer.train_ranker \
       --db snapshots.db \
       --model-json models/ranker_small.json \
       --model-onnx models/ranker_small.onnx
   ```

2. **Use quantization:**
   ```python
   from onnxruntime.quantization import quantize_dynamic, QuantType
   
   # Load the model
   model_path = "models/ranker.onnx"
   quantized_path = "models/ranker_quantized.onnx"
   
   # Quantize the model
   quantize_dynamic(model_path, quantized_path, weight_type=QuantType.QUInt8)
   ```

3. **Use GPU acceleration (if available):**
   ```rust
   // In Rust code, use GPU providers
   let session = Session::builder()?
       .with_execution_providers(["CUDAExecutionProvider".to_string()])?
       .commit_from_file(model_path)?;
   ```

### 4. Integration Issues

#### Issue: ONNX ranker not being used

**Diagnosis:**
```bash
# Check if ONNX ranker is enabled in config
grep -A 5 "model:" configs/smoothtask.yml

# Check logs for ONNX loading
journalctl -u smoothtaskd -f | grep -i onnx
```

**Solution:**
```yaml
# Ensure proper configuration
model:
  model_path: "models/ranker.onnx"
  enabled: true

policy:
  mode: "hybrid"  # or "ml_only"
```

#### Issue: Fallback to stub ranker

**Diagnosis:**
```bash
# Check for fallback messages in logs
journalctl -u smoothtaskd -f | grep -i "stub\|fallback"

# Verify model path is correct
realpath models/ranker.onnx
```

**Solution:**
```bash
# Fix file permissions
chmod 644 models/ranker.onnx

# Ensure correct path in config
# Use absolute paths if relative paths don't work
```

### 5. Training Issues

#### Error: `Insufficient data for training`

**Cause:** Not enough snapshots collected for meaningful training.

**Solution:**
```bash
# Check snapshot count
sqlite3 snapshots.db "SELECT COUNT(*) FROM snapshots;"

# Collect more data
cargo run --bin smoothtaskd -- --config configs/smoothtask.example.yml

# Wait for sufficient data (recommended: 1000+ snapshots)
```

#### Error: `Feature mismatch during training`

**Cause:** Inconsistent feature sets between training and inference.

**Solution:**
```bash
# Ensure consistent feature extraction
# Check the feature building code in both trainer and core

# Retrain with consistent features
python -m smoothtask_trainer.train_ranker \
    --db snapshots.db \
    --model-json models/ranker.json \
    --model-onnx models/ranker.onnx
```

### 6. Debugging Tools

#### ONNX Model Visualization

```bash
# Install Netron for model visualization
pip install netron

# Visualize the model
netron models/ranker.onnx
```

#### Feature Debugging

```rust
// Add debug logging to see feature vectors
let features = build_features(&app_group, &snapshot);
tracing::debug!("Features: {:?}", features);
```

#### Performance Profiling

```bash
# Profile ONNX inference
perf stat -e cache-misses,cpu-clock cargo run --bin smoothtaskd

# Check ONNX Runtime logs
export ORT_LOGGING_LEVEL=2
cargo run --bin smoothtaskd
```

### 7. Common Configuration Examples

#### Example 1: Basic ONNX Configuration

```yaml
model:
  model_path: "models/ranker.onnx"
  enabled: true

policy:
  mode: "hybrid"
```

#### Example 2: ML-Only Mode

```yaml
model:
  model_path: "models/ranker.onnx"
  enabled: true

policy:
  mode: "ml_only"
```

#### Example 3: Debug Configuration

```yaml
model:
  model_path: "models/ranker.onnx"
  enabled: true

logging:
  level: "debug"
  onnx_logging: true
```

### 8. Version Compatibility

#### ONNX Runtime Versions

| SmoothTask Version | ONNX Runtime Version |
|-------------------|---------------------|
| 0.1.x             | 1.14.x - 1.16.x      |
| 0.2.x             | 1.16.x - 1.18.x      |

#### Checking Versions

```bash
# Python ONNX Runtime
pip show onnxruntime

# Rust ONNX Runtime (ort crate)
cargo tree | grep ort

# ONNX model IR version
python -c "
import onnx
model = onnx.load('models/ranker.onnx')
print('IR version:', model.ir_version)
"
```

### 9. Performance Optimization

#### Model Optimization Techniques

1. **Quantization:** Reduces model size and improves inference speed
2. **Pruning:** Removes unnecessary weights
3. **Fusion:** Combines operations for better performance
4. **Caching:** Cache inference results for similar inputs

#### Implementation Example

```python
# Quantize ONNX model
from onnxruntime.quantization import quantize_dynamic, QuantType

quantize_dynamic(
    "models/ranker.onnx",
    "models/ranker_quantized.onnx", 
    weight_type=QuantType.QUInt8
)
```

### 10. Monitoring and Logging

#### Enabling Debug Logs

```yaml
logging:
  level: "debug"
  onnx_logging: true
```

#### Checking Logs

```bash
# View ONNX-related logs
journalctl -u smoothtaskd -f | grep -E "ONNX|model|ranker"

# Check for errors
journalctl -u smoothtaskd -f | grep -i "error"
```

## Additional Resources

- [ONNX Official Documentation](https://onnx.ai/)
- [ONNX Runtime GitHub](https://github.com/microsoft/onnxruntime)
- [CatBoost ONNX Export](https://catboost.readthedocs.io/en/latest/onnx.html)
- [Netron Model Viewer](https://github.com/lutzroeder/netron)
- [SmoothTask ONNX Integration Guide](ONNX_INTEGRATION.md)