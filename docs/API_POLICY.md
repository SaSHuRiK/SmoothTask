# SmoothTask Policy API Documentation

This document provides comprehensive API documentation for the SmoothTask policy module, covering priority classes, QoS management, and policy engine functionality.

## Table of Contents

- [Priority Classes API](#priority-classes-api)
  - [PriorityClass Enum](#priorityclass-enum)
  - [Priority Parameters](#priority-parameters)
  - [Usage Examples](#usage-examples)
- [Policy Engine API](#policy-engine-api)
  - [PolicyEngine Structure](#policyengine-structure)
  - [Key Functions](#key-functions)
  - [Integration Examples](#integration-examples)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Priority Classes API

The priority classes module defines Quality of Service (QoS) classes and their mapping to system parameters.

### PriorityClass Enum

Defines the priority classes used for process classification and resource allocation.

```rust
pub enum PriorityClass {
    /// Critically interactive processes (focus + audio/games)
    CritInteractive,
    /// Normal interactive processes (UI/CLI)
    Interactive,
    /// Default priority
    Normal,
    /// Background processes (batch/maintenance)
    Background,
    /// Processes that can run on "leftover" resources
    Idle,
}
```

### Priority Parameters

#### `PriorityParams`

Complete priority parameters for a priority class.

```rust
pub struct PriorityParams {
    pub nice: NiceParams,           // Nice priority parameters
    pub latency_nice: LatencyNiceParams, // Latency nice parameters
    pub ionice: IoNiceParams,       // I/O nice parameters
    pub cgroup: CgroupParams,       // Cgroup v2 parameters
}
```

#### `NiceParams`

Nice priority parameters.

```rust
pub struct NiceParams {
    pub nice: i32,                  // Nice value (-20 to +19, typically -8 to +10)
}
```

#### `LatencyNiceParams`

Latency nice parameters for scheduling latency control.

```rust
pub struct LatencyNiceParams {
    pub latency_nice: i32,          // Latency nice value (-20 to +19)
}
```

#### `IoNiceParams`

I/O nice parameters for disk I/O prioritization.

```rust
pub struct IoNiceParams {
    pub class: i32,                 // I/O class (1=realtime, 2=best-effort, 3=idle)
    pub level: i32,                 // Priority level within class (0-7 for best-effort)
}
```

#### `CgroupParams`

Cgroup v2 parameters for CPU resource control.

```rust
pub struct CgroupParams {
    pub cpu_weight: u32,            // CPU weight (1-10000, typically 25-200)
}
```

### PriorityClass Methods

#### `PriorityClass::params(&self) -> PriorityParams`

Gets the complete priority parameters for a priority class.

**Returns:**
- `PriorityParams`: Complete parameters for the priority class

**Example:**
```rust
use smoothtask_core::policy::classes::PriorityClass;

let params = PriorityClass::CritInteractive.params();
println!("CritInteractive nice: {}", params.nice.nice);
```

#### `PriorityClass::nice(&self) -> i32`

Gets the nice value for a priority class.

**Returns:**
- `i32`: Nice value

**Example:**
```rust
let nice = PriorityClass::Interactive.nice();
println!("Interactive nice: {}", nice);
```

#### `PriorityClass::latency_nice(&self) -> i32`

Gets the latency nice value for a priority class.

**Returns:**
- `i32`: Latency nice value

**Example:**
```rust
let latency_nice = PriorityClass::Normal.latency_nice();
println!("Normal latency_nice: {}", latency_nice);
```

#### `PriorityClass::ionice(&self) -> IoNiceParams`

Gets the I/O nice parameters for a priority class.

**Returns:**
- `IoNiceParams`: I/O nice parameters

**Example:**
```rust
let ionice = PriorityClass::Background.ionice();
println!("Background I/O class: {}", ionice.class);
```

#### `PriorityClass::cpu_weight(&self) -> u32`

Gets the CPU weight for a priority class.

**Returns:**
- `u32`: CPU weight value

**Example:**
```rust
let cpu_weight = PriorityClass::Idle.cpu_weight();
println!("Idle CPU weight: {}", cpu_weight);
```

#### `PriorityClass::as_str(&self) -> &'static str`

Gets the string representation of a priority class.

**Returns:**
- `&'static str`: String representation

**Example:**
```rust
let class_str = PriorityClass::CritInteractive.as_str();
println!("String representation: {}", class_str);
```

#### `PriorityClass::from_str(s: &str) -> Option<Self>`

Parses a priority class from a string.

**Parameters:**
- `s`: String to parse

**Returns:**
- `Option<PriorityClass>`: Parsed priority class or `None` if invalid

**Example:**
```rust
let priority_class = PriorityClass::from_str("CRIT_INTERACTIVE").unwrap();
println!("Parsed class: {:?}", priority_class);
```

### Priority Class Parameters Reference

| Class | Nice | Latency Nice | I/O Class | I/O Level | CPU Weight |
|-------|------|--------------|-----------|-----------|------------|
| CritInteractive | -8 | -15 | 2 | 0 | 200 |
| Interactive | -4 | -10 | 2 | 2 | 150 |
| Normal | 0 | 0 | 2 | 4 | 100 |
| Background | 5 | 10 | 2 | 6 | 50 |
| Idle | 10 | 15 | 3 | 0 | 25 |

### Usage Examples

#### Basic Priority Class Usage

```rust
use smoothtask_core::policy::classes::PriorityClass;

fn main() {
    // Get priority class parameters
    let crit_params = PriorityClass::CritInteractive.params();
    println!("CritInteractive parameters:");
    println!("  nice: {}", crit_params.nice.nice);
    println!("  latency_nice: {}", crit_params.latency_nice.latency_nice);
    println!("  ionice: class={}, level={}", 
             crit_params.ionice.class, crit_params.ionice.level);
    println!("  cpu_weight: {}", crit_params.cgroup.cpu_weight);
    
    // Compare priority classes
    assert!(PriorityClass::CritInteractive > PriorityClass::Interactive);
    assert!(PriorityClass::Normal < PriorityClass::Interactive);
}
```

#### Priority Class Conversion

```rust
use smoothtask_core::policy::classes::PriorityClass;

fn main() {
    // Convert to string
    let class_str = PriorityClass::Background.as_str();
    println!("Background as string: {}", class_str);
    
    // Parse from string
    let parsed_class = PriorityClass::from_str("INTERACTIVE").unwrap();
    println!("Parsed class: {:?}", parsed_class);
    
    // Get individual parameters
    let nice = PriorityClass::Idle.nice();
    let latency_nice = PriorityClass::CritInteractive.latency_nice();
    let ionice = PriorityClass::Normal.ionice();
    let cpu_weight = PriorityClass::Background.cpu_weight();
    
    println!("Idle nice: {}", nice);
    println!("CritInteractive latency_nice: {}", latency_nice);
    println!("Normal ionice: class={}, level={}", ionice.class, ionice.level);
    println!("Background cpu_weight: {}", cpu_weight);
}
```

#### Priority Class Comparison

```rust
use smoothtask_core::policy::classes::PriorityClass;

fn main() {
    // Priority classes can be compared using standard comparison operators
    let classes = vec![
        PriorityClass::CritInteractive,
        PriorityClass::Interactive,
        PriorityClass::Normal,
        PriorityClass::Background,
        PriorityClass::Idle,
    ];
    
    // Sort by priority (highest first)
    let mut sorted_classes = classes.clone();
    sorted_classes.sort_by(|a, b| b.cmp(a));
    
    println!("Classes sorted by priority:");
    for class in sorted_classes {
        println!("- {}: nice={}, cpu_weight={}", 
                 class.as_str(), 
                 class.nice(), 
                 class.cpu_weight());
    }
}
```

## Policy Engine API

The policy engine applies priority rules to processes based on their classification and system state.

### PolicyEngine Structure

```rust
pub struct PolicyEngine {
    // Internal state
}
```

### Key Functions

#### `PolicyEngine::new() -> Self`

Creates a new policy engine instance.

**Returns:**
- `PolicyEngine`: New policy engine instance

**Example:**
```rust
use smoothtask_core::policy::engine::PolicyEngine;

let engine = PolicyEngine::new();
```

#### `PolicyEngine::apply_policy(&self, process: &mut ProcessRecord, system_state: &SystemState)`

Applies policy rules to a process based on its classification and system state.

**Parameters:**
- `process`: Mutable reference to the process record
- `system_state`: Current system state

**Example:**
```rust
use smoothtask_core::policy::engine::PolicyEngine;
use smoothtask_core::logging::snapshots::ProcessRecord;

let mut engine = PolicyEngine::new();
let mut process = ProcessRecord {
    pid: 1234,
    exe: Some("firefox".to_string()),
    process_type: Some("browser".to_string()),
    priority_class: PriorityClass::Interactive,
    // ... other fields
};

let system_state = SystemState::default();
engine.apply_policy(&mut process, &system_state);
```

#### `PolicyEngine::determine_priority_class(&self, process: &ProcessRecord) -> PriorityClass`

Determines the appropriate priority class for a process based on its classification.

**Parameters:**
- `process`: Reference to the process record

**Returns:**
- `PriorityClass`: Determined priority class

**Example:**
```rust
let priority_class = engine.determine_priority_class(&process);
println!("Determined priority: {:?}", priority_class);
```

#### `PolicyEngine::apply_system_constraints(&self, priority_class: PriorityClass, system_state: &SystemState) -> PriorityClass`

Applies system constraints to adjust priority class based on current system state.

**Parameters:**
- `priority_class`: Original priority class
- `system_state`: Current system state

**Returns:**
- `PriorityClass`: Adjusted priority class

**Example:**
```rust
let adjusted_class = engine.apply_system_constraints(PriorityClass::CritInteractive, &system_state);
println!("Adjusted priority: {:?}", adjusted_class);
```

### Integration Examples

#### Basic Policy Application

```rust
use smoothtask_core::policy::engine::PolicyEngine;
use smoothtask_core::logging::snapshots::ProcessRecord;
use smoothtask_core::policy::classes::PriorityClass;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = PolicyEngine::new();
    
    // Create a process record
    let mut process = ProcessRecord {
        pid: 1234,
        exe: Some("firefox".to_string()),
        process_type: Some("browser".to_string()),
        priority_class: PriorityClass::Normal, // Initial classification
        // ... other fields
    };
    
    // Determine appropriate priority class
    let determined_class = engine.determine_priority_class(&process);
    println!("Determined priority class: {:?}", determined_class);
    
    // Apply policy to the process
    let system_state = SystemState::default();
    engine.apply_policy(&mut process, &system_state);
    
    println!("Final priority class: {:?}", process.priority_class);
    
    Ok(())
}
```

#### Priority Class Determination

```rust
use smoothtask_core::policy::engine::PolicyEngine;
use smoothtask_core::logging::snapshots::ProcessRecord;

fn determine_process_priority(process: &ProcessRecord) -> PriorityClass {
    let engine = PolicyEngine::new();
    engine.determine_priority_class(process)
}

fn main() {
    let process = ProcessRecord {
        pid: 5678,
        exe: Some("batch_job".to_string()),
        process_type: Some("batch".to_string()),
        // ... other fields
    };
    
    let priority = determine_process_priority(&process);
    println!("Batch job priority: {:?}", priority);
}
```

#### System Constraints Application

```rust
use smoothtask_core::policy::engine::PolicyEngine;
use smoothtask_core::policy::classes::PriorityClass;

fn apply_constraints(priority: PriorityClass, system_state: &SystemState) -> PriorityClass {
    let engine = PolicyEngine::new();
    engine.apply_system_constraints(priority, system_state)
}

fn main() {
    let system_state = SystemState {
        cpu_pressure: 0.8, // High CPU pressure
        memory_pressure: 0.6,
        // ... other fields
    };
    
    let original_priority = PriorityClass::CritInteractive;
    let adjusted_priority = apply_constraints(original_priority, &system_state);
    
    println!("Original: {:?}, Adjusted: {:?}", original_priority, adjusted_priority);
}
```

## Best Practices

1. **Use Appropriate Priority Classes**: Choose priority classes based on process type and user expectations.
2. **Handle System Constraints**: Always apply system constraints to prevent resource starvation.
3. **Cache Policy Decisions**: Cache priority decisions for frequently accessed processes.
4. **Monitor Priority Changes**: Track priority changes over time for debugging and optimization.
5. **Use Graceful Degradation**: Provide fallback behavior when policy application fails.

## Troubleshooting

### Common Issues

1. **Permission Errors**: Ensure the application has appropriate permissions to modify process priorities.
2. **Cgroup v2 Requirements**: Cgroup v2 is required for CPU weight management.
3. **System Constraints**: High system load may cause priority adjustments.
4. **Priority Inversion**: Be aware of potential priority inversion scenarios.

### Debugging Tips

1. Check current process priorities: `ps -eo pid,nice,cls,pri,cmd`
2. Verify cgroup v2 availability: `mount | grep cgroup2`
3. Monitor system pressure: `cat /proc/pressure/*`
4. Check process classification: Review process tags and types
5. Enable debug logging: Set appropriate log level for policy decisions

### Performance Considerations

1. **Policy Evaluation Overhead**: Complex policy rules can impact performance.
2. **Priority Change Frequency**: Limit frequency of priority changes to reduce overhead.
3. **System State Monitoring**: Cache system state information to avoid repeated queries.
4. **Batch Processing**: Apply policies to multiple processes in batches where possible.
