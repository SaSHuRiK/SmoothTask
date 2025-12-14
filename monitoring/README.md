# SmoothTask Monitoring Resources

This directory contains monitoring configuration files, Grafana dashboards, and documentation for SmoothTask performance monitoring.

## Directory Structure

```
monitoring/
├── grafana/
│   └── dashboards/
│       └── smoothtask-overview.json      # Main Grafana dashboard
├── PROMETHEUS_GRAFANA_SETUP.md           # Comprehensive setup guide
└── README.md                             # This file
```

## Quick Start

### 1. Set up Prometheus

1. Install Prometheus 2.40+
2. Configure `/etc/prometheus/prometheus.yml` with SmoothTask scrape target
3. Restart Prometheus

### 2. Set up Grafana

1. Install Grafana 10.0+
2. Add Prometheus data source
3. Import the SmoothTask dashboard

### 3. Import Dashboard

1. Open Grafana web interface (http://localhost:3000)
2. Go to **Dashboards** > **Import**
3. Upload `monitoring/grafana/dashboards/smoothtask-overview.json`
4. Select your Prometheus data source
5. Click **Import**

## Available Dashboards

### SmoothTask Overview Dashboard

**File**: `grafana/dashboards/smoothtask-overview.json`

**Features**:
- **System Overview**: Version, health score, process counts
- **Resource Usage**: CPU, memory, PSI metrics
- **Process Metrics**: CPU share, memory, I/O, network, GPU, energy
- **Process Classification**: Audio, GUI, terminal, SSH processes
- **Application Groups**: Resource aggregation and focus tracking
- **Health Monitoring**: Health score, issues, component status
- **Performance Metrics**: API performance, cache hit rate
- **Daemon Statistics**: Iterations, priority adjustments

**Panels**: 26 comprehensive panels organized in logical sections

## Documentation

### Comprehensive Setup Guide

**File**: `PROMETHEUS_GRAFANA_SETUP.md`

**Contents**:
- Step-by-step installation instructions
- Prometheus configuration examples
- Grafana setup and data source configuration
- Dashboard import instructions
- Complete metrics reference
- Alerting rules and examples
- Troubleshooting guide
- Performance optimization tips
- Security best practices

## Metrics Reference

### Key Metric Categories

| Category | Metrics Count | Description |
|----------|--------------|-------------|
| **System** | 10+ | CPU, memory, PSI, load averages |
| **Processes** | 20+ | CPU, memory, I/O, network, GPU, energy |
| **Application Groups** | 15+ | Aggregated resource usage |
| **Classification** | 8+ | Process types and characteristics |
| **Health** | 10+ | System health monitoring |
| **Daemon** | 12+ | Performance and operation metrics |
| **API** | 8+ | API performance and caching |

### Complete Metrics List

See the [setup guide](PROMETHEUS_GRAFANA_SETUP.md#key-metrics-explained) for a complete list of all available metrics with descriptions and types.

## Alerting

### Pre-configured Alerts

The setup guide includes alert rules for:

- **Health Score**: Critical and degraded states
- **Critical Issues**: Immediate notification
- **Resource Usage**: High memory warnings
- **Daemon Availability**: Downtime detection

### Custom Alert Examples

Additional alert examples for:
- High CPU usage by application groups
- High memory usage by application groups
- High I/O activity detection

## Requirements

### Software

- **Prometheus**: 2.40+ (recommended)
- **Grafana**: 10.0+ (recommended)
- **SmoothTask**: Latest version with API enabled

### Hardware (Minimum)

- **RAM**: 2GB (4GB recommended for production)
- **CPU**: 2 cores
- **Disk**: 10GB+ (depends on retention period)
- **Network**: Connectivity between components

## Configuration Examples

### Prometheus Scrape Configuration

```yaml
scrape_configs:
  - job_name: 'smoothtask'
    scrape_interval: 15s
    metrics_path: '/metrics'
    static_configs:
      - targets: ['localhost:8080']
```

### Grafana Data Source

```json
{
  "name": "SmoothTask-Prometheus",
  "type": "prometheus",
  "url": "http://localhost:9090",
  "access": "proxy",
  "basicAuth": false
}
```

## Troubleshooting

### Common Issues

1. **No metrics in Grafana**: Verify Prometheus can scrape SmoothTask
2. **Dashboard not loading**: Check data source configuration
3. **High cardinality warnings**: Review label usage in metrics
4. **Authentication failures**: Configure proper credentials

### Debugging Commands

```bash
# Test SmoothTask metrics endpoint
curl http://localhost:8080/metrics

# Check Prometheus targets
curl http://localhost:9090/api/v1/targets

# Query specific metric
curl "http://localhost:9090/api/v1/query?query=smoothtask_version"
```

## Performance Optimization

### Prometheus

- **Scrape Interval**: 15-60s based on needs
- **Retention**: 7-30 days for most use cases
- **Recording Rules**: Pre-aggregate metrics to reduce load

### Grafana

- **Dashboard Refresh**: 5-30s based on needs
- **Time Range**: Limit to relevant periods
- **Caching**: Enable for frequently accessed dashboards

## Security

### Recommendations

1. **HTTPS**: Enable for all components
2. **Authentication**: Configure for Prometheus and Grafana
3. **Network Isolation**: Restrict access to monitoring components
4. **RBAC**: Implement role-based access control
5. **Regular Updates**: Keep software up to date

## Support

For issues:

1. Check the [troubleshooting guide](PROMETHEUS_GRAFANA_SETUP.md#troubleshooting)
2. Review component logs
3. Consult official documentation
4. Open an issue on the SmoothTask repository

## Contributing

Contributions to monitoring resources are welcome:

- **New Dashboards**: Submit JSON files with clear documentation
- **Improved Queries**: Optimize existing queries
- **Additional Metrics**: Suggest new metrics to expose
- **Documentation**: Improve setup guides and examples

## License

All monitoring resources are licensed under the same license as SmoothTask.

## Additional Resources

- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)
- [SmoothTask API Documentation](../docs/API.md)
- [PromQL Query Language](https://prometheus.io/docs/prometheus/latest/querying/basics/)

## Changelog

### v1.0 (Current)
- Initial release with comprehensive overview dashboard
- Complete setup guide with best practices
- Alerting rules and troubleshooting guide
- Performance optimization recommendations

## Roadmap

Future enhancements:
- **Per-Process Dashboards**: Detailed process-level monitoring
- **Historical Analysis**: Long-term trend dashboards
- **Alert Templates**: Pre-configured alerting profiles
- **Container Monitoring**: Docker/Kubernetes specific dashboards
- **Multi-System**: Cluster-wide monitoring dashboards
