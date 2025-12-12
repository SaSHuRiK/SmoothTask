# SmoothTask Classify API Documentation

This document provides comprehensive API documentation for the SmoothTask classify module, covering process classification, pattern matching, and ML integration.

## Table of Contents

- [Pattern Database API](#pattern-database-api)
  - [Structures](#structures)
  - [Functions](#functions)
  - [Usage Examples](#usage-examples)
- [Classification API](#classification-api)
  - [Key Functions](#key-functions)
  - [Integration Examples](#integration-examples)
- [ML Classifier API](#ml-classifier-api)
  - [MLClassifier Trait](#mlclassifier-trait)
  - [Implementation Examples](#implementation-examples)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Pattern Database API

The pattern database module provides functionality for loading, managing, and matching process patterns.

### Structures

#### `PatternCategory`

Represents a category of patterns (e.g., browsers, IDEs, terminals).

```rust
pub struct PatternCategory(pub String);
```

#### `AppPattern`

Defines a pattern for matching a specific application.

```rust
pub struct AppPattern {
    pub name: String,                     // Unique application name
    pub label: String,                    // Human-readable label
    pub exe_patterns: Vec<String>,        // Executable name patterns
    pub desktop_patterns: Vec<String>,    // Desktop file patterns
    pub cgroup_patterns: Vec<String>,     // Cgroup path patterns
    pub tags: Vec<String>,                // Tags to apply when matched
}
```

#### `PatternFile`

Represents a YAML file containing patterns for a specific category.

```rust
pub struct PatternFile {
    pub category: PatternCategory,        // Pattern category
    pub apps: Vec<AppPattern>,            // Application patterns
}
```

#### `PatternDatabase`

Manages the collection of all loaded patterns.

```rust
pub struct PatternDatabase {
    // Internal pattern storage
}
```

#### `PatternUpdateResult`

Contains statistics about pattern database updates.

```rust
pub struct PatternUpdateResult {
    pub total_files: usize,              // Total files processed
    pub total_patterns: usize,            // Total patterns loaded
    pub new_patterns: usize,              // New patterns added
    pub updated_patterns: usize,          // Existing patterns updated
    pub removed_patterns: usize,          // Patterns removed
}
```

### Functions

#### `PatternDatabase::load(patterns_dir: impl AsRef<Path>) -> Result<Self>`

Loads patterns from a directory containing YAML files.

**Parameters:**
- `patterns_dir`: Directory path containing pattern YAML files

**Returns:**
- `Result<PatternDatabase>`: Loaded pattern database or error

**Example:**
```rust
use smoothtask_core::classify::rules::PatternDatabase;
use std::path::Path;

let pattern_db = PatternDatabase::load(Path::new("configs/patterns"))?;
println!("Loaded {} patterns", pattern_db.total_patterns());
```

#### `PatternDatabase::patterns_for_category(&self, category: &PatternCategory) -> &[AppPattern]`

Gets patterns for a specific category.

**Parameters:**
- `category`: Pattern category to filter by

**Returns:**
- `&[AppPattern]`: Array of patterns in the specified category

**Example:**
```rust
let browser_patterns = pattern_db.patterns_for_category(&PatternCategory("browser".to_string()));
println!("Found {} browser patterns", browser_patterns.len());
```

#### `PatternDatabase::all_patterns(&self) -> &[(PatternCategory, AppPattern)]`

Gets all patterns in the database.

**Returns:**
- `&[(PatternCategory, AppPattern)]`: Array of all patterns with their categories

**Example:**
```rust
let all_patterns = pattern_db.all_patterns();
println!("Total patterns: {}", all_patterns.len());
```

#### `PatternDatabase::reload(&mut self, patterns_dir: impl AsRef<Path>) -> Result<PatternUpdateResult>`

Reloads patterns from disk, detecting changes and updating the database.

**Parameters:**
- `patterns_dir`: Directory path containing pattern YAML files

**Returns:**
- `Result<PatternUpdateResult>`: Update statistics or error

**Example:**
```rust
let update_result = pattern_db.reload(Path::new("configs/patterns"))?;
println!("Update result: {} new, {} updated, {} removed", 
         update_result.new_patterns, 
         update_result.updated_patterns, 
         update_result.removed_patterns);
```

#### `PatternDatabase::has_changes(&self, patterns_dir: impl AsRef<Path>) -> Result<bool>`

Checks if pattern files have changed since last load.

**Parameters:**
- `patterns_dir`: Directory path containing pattern YAML files

**Returns:**
- `Result<bool>`: `true` if changes detected, `false` otherwise

**Example:**
```rust
if pattern_db.has_changes(Path::new("configs/patterns"))? {
    println!("Patterns have changed, reloading...");
    pattern_db.reload(Path::new("configs/patterns"))?;
}
```

#### `PatternDatabase::match_process(&self, process: &ProcessRecord) -> Option<&AppPattern>`

Attempts to match a process against loaded patterns.

**Parameters:**
- `process`: Process record to match

**Returns:**
- `Option<&AppPattern>`: Matching pattern or `None` if no match

**Example:**
```rust
use smoothtask_core::logging::snapshots::ProcessRecord;

let process = ProcessRecord {
    pid: 1234,
    exe: Some("firefox".to_string()),
    // ... other fields
};

if let Some(pattern) = pattern_db.match_process(&process) {
    println!("Matched pattern: {} ({})", pattern.name, pattern.label);
}
```

### Usage Examples

#### Loading and Using Pattern Database

```rust
use smoothtask_core::classify::rules::PatternDatabase;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load pattern database
    let pattern_db = PatternDatabase::load(Path::new("configs/patterns"))?;
    
    println!("Pattern database loaded successfully");
    println!("Total patterns: {}", pattern_db.total_patterns());
    
    // Get patterns by category
    let browser_patterns = pattern_db.patterns_for_category(&PatternCategory("browser".to_string()));
    println!("Browser patterns: {}", browser_patterns.len());
    
    // List all patterns
    for (category, pattern) in pattern_db.all_patterns() {
        println!("- {}: {}", category.0, pattern.name);
    }
    
    Ok(())
}
```

#### Pattern Matching

```rust
use smoothtask_core::classify::rules::PatternDatabase;
use smoothtask_core::logging::snapshots::ProcessRecord;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pattern_db = PatternDatabase::load(Path::new("configs/patterns"))?;
    
    // Create test processes
    let processes = vec![
        ProcessRecord {
            pid: 1000,
            exe: Some("firefox".to_string()),
            cmdline: Some("firefox --new-window".to_string()),
            // ... other fields
        },
        ProcessRecord {
            pid: 1001,
            exe: Some("code".to_string()),
            cmdline: Some("/usr/bin/code --user-data-dir".to_string()),
            // ... other fields
        },
    ];
    
    // Match processes against patterns
    for process in processes {
        if let Some(pattern) = pattern_db.match_process(&process) {
            println!("PID {} matched: {} ({})", 
                     process.pid, pattern.name, pattern.label);
            println!("  Tags: {:?}", pattern.tags);
        } else {
            println!("PID {} not matched", process.pid);
        }
    }
    
    Ok(())
}
```

#### Pattern Database Reloading

```rust
use smoothtask_core::classify::rules::PatternDatabase;
use std::path::Path;
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut pattern_db = PatternDatabase::load(Path::new("configs/patterns"))?;
    
    // Monitoring loop with automatic reloading
    loop {
        // Check for pattern changes
        if pattern_db.has_changes(Path::new("configs/patterns"))? {
            println!("Pattern changes detected, reloading...");
            let update_result = pattern_db.reload(Path::new("configs/patterns"))?;
            
            if update_result.has_changes() {
                println!("Pattern update summary:");
                println!("  New patterns: {}", update_result.new_patterns);
                println!("  Updated patterns: {}", update_result.updated_patterns);
                println!("  Removed patterns: {}", update_result.removed_patterns);
            }
        }
        
        // Sleep before next check
        thread::sleep(Duration::from_secs(60));
    }
}
```

## Classification API

The classification API provides functions for classifying processes using patterns and ML models.

### Key Functions

#### `classify_process(process: &mut ProcessRecord, pattern_db: &PatternDatabase, ml_classifier: Option<&dyn MLClassifier>, system_state: Option<&SystemState>)`

Classifies a process using pattern matching and optionally ML classification.

**Parameters:**
- `process`: Mutable reference to the process record (will be updated with classification results)
- `pattern_db`: Pattern database for pattern matching
- `ml_classifier`: Optional ML classifier for advanced classification
- `system_state`: Optional system state for context-aware classification

**Example:**
```rust
use smoothtask_core::classify::rules::classify_process;
use smoothtask_core::classify::ml_classifier::StubMLClassifier;

let mut process = ProcessRecord {
    pid: 1234,
    exe: Some("firefox".to_string()),
    has_gui_window: true,
    cpu_share_10s: Some(0.5),
    process_type: None,
    tags: Vec::new(),
    // ... other fields
};

let pattern_db = PatternDatabase::load(Path::new("configs/patterns"))?;
let ml_classifier = StubMLClassifier::new();

classify_process(&mut process, &pattern_db, Some(&ml_classifier), None);

println!("Process type: {:?}", process.process_type);
println!("Tags: {:?}", process.tags);
```

#### `classify_app_group(app_group: &mut AppGroupRecord, pattern_db: &PatternDatabase, ml_classifier: Option<&dyn MLClassifier>, system_state: Option<&SystemState>)`

Classifies an application group using pattern matching and optionally ML classification.

**Parameters:**
- `app_group`: Mutable reference to the app group record (will be updated with classification results)
- `pattern_db`: Pattern database for pattern matching
- `ml_classifier`: Optional ML classifier for advanced classification
- `system_state`: Optional system state for context-aware classification

**Example:**
```rust
use smoothtask_core::classify::rules::classify_app_group;
use smoothtask_core::logging::snapshots::AppGroupRecord;

let mut app_group = AppGroupRecord {
    name: "firefox".to_string(),
    processes: vec![1234, 1235, 1236],
    process_type: None,
    tags: Vec::new(),
    // ... other fields
};

classify_app_group(&mut app_group, &pattern_db, Some(&ml_classifier), None);

println!("App group type: {:?}", app_group.process_type);
println!("App group tags: {:?}", app_group.tags);
```

#### `classify_all(processes: &mut [ProcessRecord], pattern_db: &PatternDatabase, ml_classifier: Option<&dyn MLClassifier>, system_state: Option<&SystemState>)`

Classifies multiple processes efficiently.

**Parameters:**
- `processes`: Mutable slice of process records to classify
- `pattern_db`: Pattern database for pattern matching
- `ml_classifier`: Optional ML classifier for advanced classification
- `system_state`: Optional system state for context-aware classification

**Example:**
```rust
let mut processes = vec![
    ProcessRecord {
        pid: 1000,
        exe: Some("firefox".to_string()),
        // ... other fields
    },
    ProcessRecord {
        pid: 1001,
        exe: Some("code".to_string()),
        // ... other fields
    },
];

classify_all(&mut processes, &pattern_db, Some(&ml_classifier), None);

for process in processes {
    println!("PID {}: type={:?}, tags={:?}", 
             process.pid, process.process_type, process.tags);
}
```

### Integration Examples

#### Basic Process Classification

```rust
use smoothtask_core::classify::rules::{PatternDatabase, classify_process};
use smoothtask_core::logging::snapshots::ProcessRecord;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load pattern database
    let pattern_db = PatternDatabase::load(Path::new("configs/patterns"))?;
    
    // Create a process record
    let mut process = ProcessRecord {
        pid: 1234,
        exe: Some("firefox".to_string()),
        cmdline: Some("firefox --new-window https://example.com".to_string()),
        has_gui_window: true,
        cpu_share_10s: Some(0.3),
        mem_rss_kb: Some(150_000),
        process_type: None,
        tags: Vec::new(),
        // ... other fields
    };
    
    // Classify the process (pattern matching only)
    classify_process(&mut process, &pattern_db, None, None);
    
    println!("Classification results:");
    println!("  Process type: {:?}", process.process_type);
    println!("  Tags: {:?}", process.tags);
    
    Ok(())
}
```

#### ML-Enhanced Classification

```rust
use smoothtask_core::classify::rules::classify_process;
use smoothtask_core::classify::ml_classifier::StubMLClassifier;
use smoothtask_core::classify::rules::PatternDatabase;
use smoothtask_core::logging::snapshots::ProcessRecord;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load pattern database
    let pattern_db = PatternDatabase::load(Path::new("configs/patterns"))?;
    
    // Create ML classifier (stub for testing)
    let ml_classifier = StubMLClassifier::new();
    
    // Create a process record
    let mut process = ProcessRecord {
        pid: 5678,
        exe: Some("custom_app".to_string()),
        cmdline: Some("/usr/bin/custom_app --mode=interactive".to_string()),
        has_gui_window: true,
        cpu_share_10s: Some(0.8),
        mem_rss_kb: Some(200_000),
        io_read_bytes: Some(10_000),
        io_write_bytes: Some(5_000),
        process_type: None,
        tags: Vec::new(),
        // ... other fields
    };
    
    // Classify with ML enhancement
    classify_process(&mut process, &pattern_db, Some(&ml_classifier), None);
    
    println!("ML-enhanced classification results:");
    println!("  Process type: {:?}", process.process_type);
    println!("  Tags: {:?}", process.tags);
    println!("  Priority class: {:?}", process.priority_class);
    
    Ok(())
}
```

#### Batch Classification

```rust
use smoothtask_core::classify::rules::{PatternDatabase, classify_all};
use smoothtask_core::classify::ml_classifier::StubMLClassifier;
use smoothtask_core::logging::snapshots::ProcessRecord;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load pattern database
    let pattern_db = PatternDatabase::load(Path::new("configs/patterns"))?;
    
    // Create ML classifier
    let ml_classifier = StubMLClassifier::new();
    
    // Create multiple process records
    let mut processes = vec![
        ProcessRecord {
            pid: 1000,
            exe: Some("firefox".to_string()),
            // ... other fields
        },
        ProcessRecord {
            pid: 1001,
            exe: Some("code".to_string()),
            // ... other fields
        },
        ProcessRecord {
            pid: 1002,
            exe: Some("batch_job".to_string()),
            // ... other fields
        },
    ];
    
    // Classify all processes
    classify_all(&mut processes, &pattern_db, Some(&ml_classifier), None);
    
    // Process classification results
    for process in processes {
        println!("PID {}: type={:?}, tags={:?}", 
                 process.pid, process.process_type, process.tags);
    }
    
    Ok(())
}
```

## ML Classifier API

The ML classifier API provides integration with machine learning models for advanced process classification.

### MLClassifier Trait

```rust
pub trait MLClassifier {
    /// Classifies a process using ML model
    fn classify(&self, process: &ProcessRecord) -> Result<MLClassificationResult>;
    
    /// Gets the confidence threshold for ML classification
    fn confidence_threshold(&self) -> f32;
    
    /// Sets the confidence threshold for ML classification
    fn set_confidence_threshold(&mut self, threshold: f32);
    
    /// Gets classifier metadata
    fn metadata(&self) -> MLClassifierMetadata;
}
```

### MLClassificationResult

```rust
pub struct MLClassificationResult {
    pub process_type: Option<String>,   // Predicted process type
    pub tags: Vec<String>,              // Predicted tags
    pub confidence: f32,                // Classification confidence (0.0-1.0)
    pub model_version: String,          // Model version used
}
```

### MLClassifierMetadata

```rust
pub struct MLClassifierMetadata {
    pub name: String,                   // Classifier name
    pub version: String,                // Classifier version
    pub model_type: String,             // Model type (e.g., "catboost", "onnx")
    pub features_used: Vec<String>,     // Features used by the model
}
```

### Implementation Examples

#### Using StubMLClassifier (for testing)

```rust
use smoothtask_core::classify::ml_classifier::StubMLClassifier;
use smoothtask_core::logging::snapshots::ProcessRecord;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create stub classifier
    let mut classifier = StubMLClassifier::new();
    
    // Configure confidence threshold
    classifier.set_confidence_threshold(0.7);
    
    // Create a process record
    let process = ProcessRecord {
        pid: 1234,
        exe: Some("test_app".to_string()),
        has_gui_window: true,
        cpu_share_10s: Some(0.5),
        // ... other fields
    };
    
    // Classify the process
    let result = classifier.classify(&process)?;
    
    println!("ML Classification Result:");
    println!("  Process type: {:?}", result.process_type);
    println!("  Tags: {:?}", result.tags);
    println!("  Confidence: {:.2}", result.confidence);
    println!("  Model version: {}", result.model_version);
    
    // Get classifier metadata
    let metadata = classifier.metadata();
    println!("Classifier metadata:");
    println!("  Name: {}", metadata.name);
    println!("  Version: {}", metadata.version);
    println!("  Model type: {}", metadata.model_type);
    
    Ok(())
}
```

#### Using CatBoostClassifier

```rust
#[cfg(feature = "catboost")]
use smoothtask_core::classify::ml_classifier::CatBoostClassifier;

#[cfg(feature = "catboost")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load CatBoost model
    let classifier = CatBoostClassifier::load("models/process_classifier.cbm")?;
    
    // Create a process record
    let process = ProcessRecord {
        pid: 5678,
        exe: Some("ml_test_app".to_string()),
        has_gui_window: true,
        cpu_share_10s: Some(0.8),
        mem_rss_kb: Some(250_000),
        io_read_bytes: Some(15_000),
        // ... other fields
    };
    
    // Classify using CatBoost model
    let result = classifier.classify(&process)?;
    
    println!("CatBoost Classification:");
    println!("  Type: {:?} (confidence: {:.2})", 
             result.process_type, result.confidence);
    println!("  Tags: {:?}", result.tags);
    
    Ok(())
}
```

#### Using ONNX Classifier

```rust
#[cfg(feature = "onnx")]
use smoothtask_core::classify::ml_classifier::ONNXClassifier;

#[cfg(feature = "onnx")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load ONNX model
    let classifier = ONNXClassifier::load("models/process_classifier.onnx")?;
    
    // Create a process record
    let process = ProcessRecord {
        pid: 9012,
        exe: Some("onnx_test_app".to_string()),
        has_gui_window: false,
        cpu_share_10s: Some(0.2),
        mem_rss_kb: Some(50_000),
        // ... other fields
    };
    
    // Classify using ONNX model
    let result = classifier.classify(&process)?;
    
    println!("ONNX Classification:");
    println!("  Type: {:?} (confidence: {:.2})", 
             result.process_type, result.confidence);
    println!("  Tags: {:?}", result.tags);
    println!("  Model version: {}", result.model_version);
    
    Ok(())
}
```

## Best Practices

1. **Pattern Organization**: Organize patterns by category for easier management.
2. **Pattern Specificity**: Use specific patterns before general ones to avoid misclassification.
3. **ML Fallback**: Use ML classification as a fallback when pattern matching fails.
4. **Confidence Thresholds**: Set appropriate confidence thresholds for ML classification.
5. **Performance Monitoring**: Monitor classification performance and adjust as needed.
6. **Pattern Validation**: Validate patterns against real process data.
7. **Cache Results**: Cache classification results for frequently seen processes.

## Troubleshooting

### Common Issues

1. **Pattern Matching Failures**: Ensure patterns are specific enough and cover common variations.
2. **ML Classification Errors**: Verify model files are accessible and in the correct format.
3. **Performance Issues**: Optimize pattern matching and consider caching for frequently accessed processes.
4. **Memory Usage**: Large pattern databases can consume significant memory.
5. **File Access Errors**: Ensure the application has read access to pattern directories.

### Debugging Tips

1. Enable debug logging for classification: `RUST_LOG=debug`
2. Test pattern matching with specific processes: Use `match_process` for debugging
3. Validate YAML files: Ensure pattern files are valid YAML
4. Check file permissions: Verify read access to pattern directories
5. Monitor classification performance: Track classification time and success rates

### Performance Considerations

1. **Pattern Matching**: Complex patterns can impact performance - use simple patterns where possible.
2. **ML Classification**: ML classification is more expensive than pattern matching - use judiciously.
3. **Caching**: Cache classification results for processes that don't change frequently.
4. **Batch Processing**: Classify multiple processes in batches to reduce overhead.
5. **Memory Usage**: Large pattern databases consume memory - consider lazy loading for rarely used patterns.
