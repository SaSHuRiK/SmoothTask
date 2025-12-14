# SmoothTask Container Support

This document describes SmoothTask's container detection, monitoring, and integration capabilities.

## Overview

SmoothTask provides comprehensive support for running in containerized environments including Docker, Podman, Containerd, and LXC. The system automatically detects when it's running in a container and adapts its behavior accordingly.

## Container Detection

### Detection Methods

SmoothTask uses multiple methods to detect container environments:

1. **Environment Variables**
   - `CONTAINER_TYPE` (docker, podman, etc.)
   - `DOCKER_CONTAINER` (Docker-specific)
   - `PODMAN_CONTAINER` (Podman-specific)

2. **Container-Specific Files**
   - `/.dockerenv` (Docker)
   - `/.containerenv` (Podman)

3. **Cgroup Information**
   - Parses `/proc/1/cgroup` for container-specific cgroup paths
   - Detects Docker, Podman, Containerd, and LXC patterns

### Supported Runtimes

- **Docker**: Full support with automatic detection
- **Podman**: Full support with automatic detection  
- **Containerd**: Detected via cgroup patterns
- **LXC**: Detected via cgroup patterns
- **Unknown**: Custom runtimes can be detected and labeled

### API Functions

```rust
use smoothtask_core::utils::container::*;

// Detect container runtime
let runtime = detect_container_runtime();

// Check if running in container
let containerized = is_containerized();

// Get detailed container information
let info = get_container_info();

// Adapt configuration for container environment
let adapted = adapt_for_container();
```

## Container Metrics

### Metrics Collection

SmoothTask collects container-specific metrics including:

- **Memory**: Limit and current usage
- **CPU**: Shares/weight and quota/period constraints
- **Network**: Container-specific network interfaces
- **Runtime**: Container runtime type and ID
- **Cgroup**: Container cgroup path

### Cgroup v1 vs v2 Support

The system automatically detects and supports both cgroup versions:

**cgroup v1 paths:**
- Memory: `/sys/fs/cgroup/memory{}/memory.limit_in_bytes`, `/sys/fs/cgroup/memory{}/memory.usage_in_bytes`
- CPU: `/sys/fs/cgroup/cpu{}/cpu.shares`, `/sys/fs/cgroup/cpu{}/cpu.cfs_quota_us`, `/sys/fs/cgroup/cpu{}/cpu.cfs_period_us`

**cgroup v2 paths:**
- Memory: `/sys/fs/cgroup{}/memory.max`, `/sys/fs/cgroup{}/memory.current`
- CPU: `/sys/fs/cgroup{}/cpu.weight`, `/sys/fs/cgroup{}/cpu.max`

### Metrics Structure

```rust
pub struct ContainerMetrics {
    pub runtime: ContainerRuntime,
    pub container_id: Option<String>,
    pub memory_limit_bytes: Option<u64>,
    pub memory_usage_bytes: Option<u64>,
    pub cpu_shares: Option<u64>,
    pub cpu_quota: Option<i64>,
    pub cpu_period: Option<u64>,
    pub network_interfaces: Vec<String>,
}
```

### Usage Example

```rust
use smoothtask_core::utils::container::collect_container_metrics;

let metrics = collect_container_metrics();

if let Some(limit) = metrics.memory_limit_bytes {
    println!("Container memory limit: {} bytes", limit);
}

if let Some(shares) = metrics.cpu_shares {
    println!("Container CPU shares/weight: {}", shares);
}
```

## Container Integration

### Configuration Adaptation

When running in containers, SmoothTask automatically:
- Adjusts monitoring intervals
- Modifies resource allocation strategies
- Adapts to container constraints
- Logs container-specific information

### Policy Adjustments

Container-aware policy engine:
- More conservative resource allocation
- Container constraint awareness
- Graceful degradation when limits are hit
- Container-specific priority management

## Docker Support

### Dockerfiles

SmoothTask provides several Dockerfile configurations:

1. **Production Dockerfile** (`Dockerfile`)
   - Multi-stage build for minimal image
   - All runtime dependencies
   - Proper cgroup v2 support
   - Non-root user for security

2. **Development Dockerfile** (`Dockerfile.dev`)
   - Full build toolchain
   - Python environment for trainer
   - Source code mounted for testing

3. **Minimal Dockerfile** (`Dockerfile.minimal`)
   - Essential dependencies only
   - Smallest footprint
   - Basic functionality

### Building Images

```bash
# Production image
docker build -t smoothtask:latest .

# Development image
docker build -t smoothtask-dev -f Dockerfile.dev .

# Minimal image
docker build -t smoothtask-minimal -f Dockerfile.minimal .
```

### Running Containers

```bash
docker run -d \
  --name smoothtask \
  --privileged \
  --volume /sys/fs/cgroup:/sys/fs/cgroup:ro \
  --volume /etc/smoothtask:/etc/smoothtask \
  --volume /var/log/smoothtask:/var/log/smoothtask \
  -p 8080:8080 \
  smoothtask:latest
```

## Container-Specific Configuration

### Configuration Example

```yaml
# Container-specific configuration
container:
  enabled: true
  
  metrics:
    enabled: true
    interval_sec: 60

# Adjust resource monitoring for container environments
metrics:
  system:
    interval_sec: 10
  
  process:
    interval_sec: 5

# Policy adjustments for containers
policy:
  cpu_weight_range: [10, 500]
  memory_protection: true
```

### Environment Variables

- `CONTAINER_TYPE`: Set container runtime type
- `DOCKER_CONTAINER`: Force Docker detection
- `PODMAN_CONTAINER`: Force Podman detection
- `RUST_LOG`: Set log level (default: `info`)
- `SMOOTHTASK_CONFIG`: Configuration file path

## Container Monitoring

### Metrics Collection Process

1. **Detection**: Identify container runtime and environment
2. **Cgroup Analysis**: Parse cgroup information for constraints
3. **Resource Monitoring**: Collect memory, CPU, and network metrics
4. **Adaptation**: Adjust SmoothTask behavior for container constraints
5. **Logging**: Record container-specific information

### Monitoring Intervals

- Container metrics: Configurable (default: 60 seconds)
- System metrics: Adjusted for container environments
- Process metrics: Container-optimized intervals

## Container Health Monitoring

### Health Indicators

- Container memory usage vs limits
- CPU constraint utilization
- Network interface status
- Cgroup constraint compliance

### Alerting

Container-specific alerts for:
- Approaching memory limits
- CPU constraint violations
- Network interface issues
- Cgroup permission problems

## Container-Specific Features

### Automatic Detection

- No manual configuration required
- Works with all major container runtimes
- Graceful degradation in unsupported environments

### Resource Awareness

- Respects container memory limits
- Adapts to CPU constraints
- Monitors network interfaces
- Container-constrained resource allocation

### Security

- Non-root user by default
- Read-only cgroup access
- Minimal privilege requirements
- Secure configuration handling

## Container Integration Testing

### Test Coverage

Comprehensive integration tests verify:
- Container detection in various environments
- Cgroup v1 and v2 metrics collection
- Container-specific adaptations
- Error handling and graceful degradation

### Running Tests

```bash
# Run container integration tests
cargo test --package smoothtask-core --test container_integration_test

# Run in container for full testing
docker build -t smoothtask-test .
docker run --rm smoothtask-test /usr/local/bin/smoothtaskd --container-info
```

## Container-Specific Error Handling

### Common Issues and Solutions

**Cgroup Permission Errors**
- Ensure `/sys/fs/cgroup` is mounted with proper permissions
- Use `--privileged` flag or specific capabilities
- Check container has access to host's cgroup filesystem

**Container Detection Failures**
- Verify environment variables are set correctly
- Check container-specific files exist
- Ensure supported container runtime

**Performance Issues**
- Use host networking for lower latency
- Mount `/sys` and `/proc` for accurate metrics
- Adjust monitoring intervals based on resources

## Container Best Practices

### Configuration

- Use container-optimized configuration
- Adjust monitoring intervals for container resources
- Enable container-specific metrics
- Set appropriate log levels

### Deployment

- Use minimal production images
- Run as non-root user
- Mount necessary volumes
- Set proper resource limits

### Monitoring

- Monitor container-specific metrics
- Set up container health alerts
- Track resource constraint utilization
- Log container events

### Security

- Use read-only filesystems where possible
- Limit capabilities to essentials
- Use secrets for sensitive configuration
- Regularly update base images

## Container-Specific API

### Container Info Endpoint

```bash
# Get container information
curl http://localhost:8080/api/container/info

# Get container metrics
curl http://localhost:8080/api/container/metrics
```

### Response Format

```json
{
  "runtime": "docker",
  "container_id": "abc123",
  "is_containerized": true,
  "cgroup_path": "/docker/abc123",
  "metrics": {
    "memory_limit_bytes": 2147483648,
    "memory_usage_bytes": 1073741824,
    "cpu_shares": 1024,
    "cpu_quota": 100000,
    "cpu_period": 100000,
    "network_interfaces": ["eth0", "veth1"]
  }
}
```

## Container-Specific Logging

### Log Format

Container-related log entries include:
- Container runtime information
- Container ID and cgroup path
- Container metrics and constraints
- Container-specific events

### Log Example

```
INFO  container: Detected container environment: Docker
INFO  container: Container ID: abc123def456
INFO  container: Container cgroup path: /docker/abc123def456
INFO  container: Container memory limit: 2147483648 bytes
INFO  container: Container CPU shares: 1024
```

## Container-Specific Troubleshooting

### Debugging Container Detection

```bash
# Check container detection
smoothtaskd --container-info

# Check environment variables
env | grep CONTAINER

# Check container files
ls -la /.dockerenv /.containerenv

# Check cgroup information
cat /proc/1/cgroup
```

### Debugging Metrics Collection

```bash
# Check cgroup v2 availability
smoothtaskd --cgroup-info

# Check memory limits
cat /sys/fs/cgroup/memory/memory.limit_in_bytes
cat /sys/fs/cgroup/memory.max

# Check CPU constraints
cat /sys/fs/cgroup/cpu/cpu.shares
cat /sys/fs/cgroup/cpu.weight
```

## Container-Specific Performance

### Performance Considerations

- Container overhead is minimal
- Metrics collection is optimized for containers
- Resource monitoring respects container constraints
- Adaptive algorithms work within container limits

### Performance Tuning

```yaml
# Container performance configuration
container:
  metrics_interval_sec: 30
  adaptation_interval_sec: 60
  
metrics:
  system_interval_sec: 5
  process_interval_sec: 3
```

## Container-Specific Future Development

### Planned Enhancements

- Kubernetes integration
- Container orchestration support
- Enhanced container health monitoring
- Container-specific policy profiles
- Improved container network monitoring

### Roadmap

1. **Kubernetes Support**: Automatic detection and integration
2. **Orchestration APIs**: Container lifecycle management
3. **Enhanced Metrics**: More detailed container monitoring
4. **Policy Profiles**: Container-type specific policies
5. **Network Monitoring**: Container network performance tracking

## Container-Specific References

### Related Documentation

- [Caching System](CACHING_SYSTEM.md)
- [Hysteresis Mechanism](HYSTERESIS_MECHANISM.md)
- [API Documentation](API.md)
- [Configuration Schema](API_CONFIG_SCHEMA.md)

### External Resources

- Docker Documentation: https://docs.docker.com/
- Podman Documentation: https://podman.io/docs
- cgroups v2 Documentation: https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html
- Container Runtimes: https://containerd.io/ https://linuxcontainers.org/

## Container-Specific Support

For container-specific issues and questions:
- Check container detection and metrics
- Verify cgroup permissions and access
- Review container configuration
- Consult container runtime documentation
- Check SmoothTask logs for container events