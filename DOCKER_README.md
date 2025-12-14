# SmoothTask Docker Support

This directory contains Dockerfiles and configuration for running SmoothTask in containerized environments.

## Available Dockerfiles

### 1. `Dockerfile` - Production Image

A production-ready Docker image with:
- Multi-stage build for minimal final image
- All necessary runtime dependencies
- Proper cgroup v2 support
- Non-root user for security
- Entrypoint script for container-specific setup

**Build:**
```bash
docker build -t smoothtask:latest .
```

**Run:**
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

### 2. `Dockerfile.dev` - Development Image

A development environment with:
- Full build toolchain
- Python virtual environment for trainer
- All development dependencies
- Source code mounted for easy testing

**Build:**
```bash
docker build -t smoothtask-dev -f Dockerfile.dev .
```

**Run:**
```bash
docker run -it \
  --name smoothtask-dev \
  --privileged \
  --volume $(pwd):/app \
  --volume /sys/fs/cgroup:/sys/fs/cgroup:ro \
  -p 8080:8080 \
  smoothtask-dev bash
```

### 3. `Dockerfile.minimal` - Minimal Production Image

A minimal production image with:
- Only essential runtime dependencies
- No optional features
- Smallest possible footprint
- Basic functionality only

**Build:**
```bash
docker build -t smoothtask-minimal -f Dockerfile.minimal .
```

## Container-Specific Configuration

### Cgroup v2 Support

SmoothTask automatically detects cgroup v2 environments and adapts its metrics collection accordingly. The container metrics module supports:

- **cgroup v1 paths**: `/sys/fs/cgroup/memory{}/memory.limit_in_bytes`, `/sys/fs/cgroup/cpu{}/cpu.shares`
- **cgroup v2 paths**: `/sys/fs/cgroup{}/memory.max`, `/sys/fs/cgroup{}/cpu.weight`, `/sys/fs/cgroup{}/cpu.max`

### Container Detection

SmoothTask detects container environments using:
- Environment variables (`CONTAINER_TYPE`, `DOCKER_CONTAINER`, `PODMAN_CONTAINER`)
- Container-specific files (`/.dockerenv`, `/.containerenv`)
- Cgroup information (`/proc/1/cgroup`)

### Runtime Detection

Supported container runtimes:
- Docker
- Podman
- Containerd
- LXC

## Running in Containers

### Required Privileges

For full functionality, SmoothTask requires:
- `--privileged` mode or specific capabilities
- Access to `/sys/fs/cgroup` (read-only is sufficient for monitoring)
- Access to `/proc` and `/sys` for system metrics
- Network access for API endpoints

### Volume Mounts

Recommended volume mounts:
- `/etc/smoothtask` - Configuration directory
- `/var/log/smoothtask` - Log directory
- `/var/lib/smoothtask` - Data directory
- `/sys/fs/cgroup:/sys/fs/cgroup:ro` - Cgroup access (read-only)

### Environment Variables

- `RUST_LOG` - Log level (default: `info`)
- `SMOOTHTASK_CONFIG` - Path to configuration file (default: `/etc/smoothtask/config.yml`)

## Container-Specific Configuration Examples

### Example `config.yml` for Containers

```yaml
# Container-specific configuration
container:
  # Enable container-aware mode
  enabled: true
  
  # Container metrics collection
  metrics:
    enabled: true
    interval_sec: 60

# Adjust resource monitoring for container environments
metrics:
  system:
    # Reduce frequency in containers
    interval_sec: 10
  
  process:
    # Container-optimized process monitoring
    interval_sec: 5

# Policy adjustments for containers
policy:
  # More conservative resource allocation in containers
  cpu_weight_range: [10, 500]
  memory_protection: true
```

## Building and Testing

### Build all images
```bash
docker build -t smoothtask:latest .
docker build -t smoothtask-dev -f Dockerfile.dev .
docker build -t smoothtask-minimal -f Dockerfile.minimal .
```

### Test container detection
```bash
docker run --rm smoothtask:latest /usr/local/bin/smoothtaskd --version
docker run --rm -e CONTAINER_TYPE=docker smoothtask:latest /usr/local/bin/smoothtaskd --container-info
```

## Troubleshooting

### Cgroup Permissions

If you see cgroup permission errors:
- Ensure `/sys/fs/cgroup` is mounted with proper permissions
- Use `--privileged` flag or specific capabilities
- Check that the container has access to the host's cgroup filesystem

### Container Detection Issues

If container detection fails:
- Check that environment variables are properly set
- Verify that container-specific files exist
- Ensure the container runtime is supported

### Performance in Containers

For better performance in containers:
- Use host networking (`--network host`) for lower latency
- Mount `/sys` and `/proc` for accurate system metrics
- Adjust monitoring intervals based on container resources

## Security Considerations

- Run as non-root user (already configured in Dockerfiles)
- Use read-only filesystems where possible
- Limit capabilities to only what's needed
- Use secrets for sensitive configuration
- Regularly update base images

## Container Integration Testing

The container support includes comprehensive integration tests that verify:
- Container detection in various environments
- Cgroup v1 and v2 metrics collection
- Container-specific adaptations
- Error handling and graceful degradation

Run container integration tests with:
```bash
cargo test --package smoothtask-core --test container_integration_test
```