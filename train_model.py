#!/usr/bin/env python3
"""
Скрипт для обучения модели на собранных данных.
"""

import sys
from pathlib import Path

# Add the trainer to path
sys.path.insert(0, 'smoothtask-trainer')

from smoothtask_trainer.train_pipeline import TrainingPipeline
from smoothtask_trainer.train_ranker import train_ranker
from smoothtask_trainer.export_model import export_model

def main():
    print("=== SmoothTask Model Training ===")
    
    # Check if training data exists
    training_db = Path("training_data.sqlite")
    if not training_db.exists():
        print(f"Error: Training database not found: {training_db}")
        return 1
    
    print(f"Using training database: {training_db}")
    
    # Validate the database first
    try:
        from smoothtask_trainer.collect_data import validate_dataset
        
        stats = validate_dataset(
            db_path=training_db,
            min_snapshots=5,
            min_processes=50,
            min_groups=10
        )
        
        print(f"\nDataset validation:")
        print(f"  Snapshots: {stats['snapshot_count']}")
        print(f"  Processes: {stats['process_count']}")
        print(f"  Groups: {stats['group_count']}")
        print(f"  Unique processes: {stats['unique_processes']}")
        print(f"  Unique groups: {stats['unique_groups']}")
        
        # Check if we have enough data for meaningful training
        if stats['snapshot_count'] < 5:
            print("Warning: Low number of snapshots for training")
        if stats['process_count'] < 50:
            print("Warning: Low number of processes for training")
        if stats['group_count'] < 10:
            print("Warning: Low number of groups for training")
        
    except Exception as e:
        print(f"Error validating dataset: {e}")
        return 1
    
    # Train the model
    try:
        print(f"\nTraining model...")
        
        # Use the training pipeline
        pipeline = TrainingPipeline(
            db_path=training_db,
            use_temp_db=False,
            min_snapshots=1,
            min_processes=10,
            min_groups=1
        )
        
        # Collect data first
        db_path = pipeline.collect_data()
        print(f"Data collected from: {db_path}")
        
        # Train model and save in both formats
        model_path_json = Path("trained_model.json")
        model_path_onnx = Path("trained_model.onnx")
        
        model = pipeline.train_model(
            model_path=model_path_json,
            onnx_path=model_path_onnx
        )
        
        print(f"✅ Model training completed!")
        print(f"  JSON model saved to: {model_path_json}")
        print(f"  ONNX model saved to: {model_path_onnx}")
        
        # Get model info
        if hasattr(model, 'get_params'):
            params = model.get_params()
            print(f"\nModel parameters:")
            print(f"  Loss function: {params.get('loss_function', 'Unknown')}")
            print(f"  Depth: {params.get('depth', 'Unknown')}")
            print(f"  Learning rate: {params.get('learning_rate', 'Unknown')}")
            print(f"  Iterations: {params.get('iterations', 'Unknown')}")
        
        # Check model files
        if model_path_json.exists():
            print(f"\n✅ JSON model file created: {model_path_json.stat().st_size} bytes")
        else:
            print(f"\n❌ JSON model file not created")
            
        if model_path_onnx.exists():
            print(f"✅ ONNX model file created: {model_path_onnx.stat().st_size} bytes")
        else:
            print(f"❌ ONNX model file not created")
        
        return 0
        
    except Exception as e:
        print(f"Error during model training: {e}")
        import traceback
        traceback.print_exc()
        return 1

if __name__ == "__main__":
    sys.exit(main())
