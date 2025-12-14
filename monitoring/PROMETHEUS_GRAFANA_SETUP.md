# SmoothTask Prometheus & Grafana Monitoring Setup Guide

This guide provides comprehensive instructions for setting up Prometheus and Grafana to monitor SmoothTask performance metrics.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Prometheus Configuration](#prometheus-configuration)
- [Grafana Configuration](#grafana-configuration)
- [Dashboard Import](#dashboard-import)
- [Key Metrics Explained](#key-metrics-explained)
- [Alerting Rules](#alerting-rules)
  - [Custom Alert Examples](#custom-alert-examples)
  - [Comprehensive Alerting Rules](#comprehensive-alerting-rules)
  - [Alert Configuration Best Practices](#alert-configuration-best-practices)
  - [Alert Integration Examples](#alert-integration-examples)
- [Troubleshooting](#troubleshooting)

## Overview

SmoothTask provides a comprehensive Prometheus metrics endpoint (`/metrics`) that exposes detailed information about:

- **System Resources**: CPU, memory, PSI (Pressure Stall Information)
- **Process Metrics**: CPU share, memory usage, I/O operations, network traffic
- **Application Groups**: Resource aggregation by application
- **Process Classification**: Audio clients, GUI windows, terminal sessions, SSH processes
- **Health Monitoring**: System health score, issues, component status
- **Daemon Performance**: Iterations, priority adjustments, API performance

## Prerequisites

### Software Requirements

- **Prometheus** 2.40+ (recommended)
- **Grafana** 10.0+ (recommended)
- **SmoothTask** with API server enabled
- Basic knowledge of YAML configuration

### Hardware Requirements

- Minimum 2GB RAM for Prometheus/Grafana stack
- 10GB+ disk space for metrics storage (depends on retention)
- Network connectivity between components

## Prometheus Configuration

### 1. Install Prometheus

```bash
# For Debian/Ubuntu
sudo apt-get update
sudo apt-get install -y prometheus

# For RHEL/CentOS
sudo yum install -y prometheus2

# For Docker
docker run -d -p 9090:9090 --name prometheus prom/prometheus
```

### 2. Configure Prometheus

Edit the Prometheus configuration file (typically `/etc/prometheus/prometheus.yml`):

```yaml
# Global configuration
global:
  scrape_interval: 15s
  evaluation_interval: 15s
  scrape_timeout: 10s

# Alertmanager configuration
alerting:
  alertmanagers:
    - static_configs:
        - targets: []

# Rule files
rule_files:
  - "alert.rules"

# Scrape configuration
scrape_configs:
  # SmoothTask metrics
  - job_name: 'smoothtask'
    scrape_interval: 15s
    metrics_path: '/metrics'
    static_configs:
      - targets: ['localhost:8080']  # Replace with your SmoothTask API address
    
  # Prometheus self-monitoring
  - job_name: 'prometheus'
    scrape_interval: 15s
    static_configs:
      - targets: ['localhost:9090']
```

### 3. Create Alert Rules (Optional)

Create `/etc/prometheus/alert.rules` with SmoothTask-specific alerts:

```yaml
groups:
- name: smoothtask-alerts
  interval: 30s
  rules:
  
  # Health alerts
  - alert: SmoothTaskHealthCritical
    expr: smoothtask_health_score < 50
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "SmoothTask health score is critical ({{ $value }})"
      description: "System health score has dropped below 50, indicating potential issues"

  - alert: SmoothTaskHealthDegraded
    expr: smoothtask_health_score < 70
    for: 10m
    labels:
      severity: warning
    annotations:
      summary: "SmoothTask health score is degraded ({{ $value }})"
      description: "System health score has dropped below 70"

  # Critical issues alert
  - alert: SmoothTaskCriticalIssues
    expr: smoothtask_health_critical_issues > 0
    for: 1m
    labels:
      severity: critical
    annotations:
      summary: "SmoothTask has critical health issues ({{ $value }})"
      description: "Critical health issues detected in SmoothTask"

  # High memory usage
  - alert: SmoothTaskHighMemoryUsage
    expr: (smoothtask_system_memory_used_kb / smoothtask_system_memory_total_kb) * 100 > 90
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "High memory usage detected ({{ $value }}%)"
      description: "System memory usage exceeds 90%"

  # Daemon not running
  - alert: SmoothTaskDaemonNotRunning
    expr: absent(smoothtask_version)
    for: 2m
    labels:
      severity: critical
    annotations:
      summary: "SmoothTask daemon is not running"
      description: "No metrics received from SmoothTask daemon"
```

### 4. Restart Prometheus

```bash
# For systemd
sudo systemctl restart prometheus

# For Docker
docker restart prometheus
```

### 5. Verify Prometheus Configuration

Check that Prometheus can access SmoothTask metrics:

```bash
# Check targets
curl -s http://localhost:9090/api/v1/targets | jq '.data.activeTargets[] | select(.labels.job == "smoothtask")'

# Check if metrics are being scraped
curl -s http://localhost:9090/api/v1/query?query=smoothtask_version
```

## Grafana Configuration

### 1. Install Grafana

```bash
# For Debian/Ubuntu
sudo apt-get update
sudo apt-get install -y grafana

# For RHEL/CentOS
sudo yum install -y grafana

# For Docker
docker run -d -p 3000:3000 --name grafana grafana/grafana
```

### 2. Start Grafana

```bash
# For systemd
sudo systemctl start grafana-server
sudo systemctl enable grafana-server

# For Docker
docker start grafana
```

### 3. Access Grafana Web Interface

Open your browser and navigate to `http://localhost:3000` (or your server IP).

- Default username: `admin`
- Default password: `admin`

### 4. Add Prometheus Data Source

1. Click the **Configuration** (gear) icon in the left sidebar
2. Select **Data Sources**
3. Click **Add data source**
4. Select **Prometheus**
5. Configure the data source:
   - **Name**: `SmoothTask-Prometheus`
   - **URL**: `http://localhost:9090` (or your Prometheus server)
   - **Access**: `Server` (default)
6. Click **Save & Test**

## Dashboard Import

### 1. Import the SmoothTask Overview Dashboard

1. Click the **+** icon in the left sidebar
2. Select **Import**
3. Click **Upload JSON file** and select the `smoothtask-overview.json` file
4. Select the **SmoothTask-Prometheus** data source
5. Click **Import**

### 2. Dashboard Features

The SmoothTask Overview Dashboard includes:

#### System Overview
- **SmoothTask Version**: Current daemon version
- **System Health Score**: Overall system health (0-100)
- **Total Processes**: Number of monitored processes
- **Application Groups**: Number of application groups
- **Health Issues**: Total health issues detected

#### System Resource Usage
- **Memory Usage %**: System memory utilization
- **CPU Usage %**: System CPU utilization

#### Process Resource Usage
- **CPU Share 1s**: Total CPU share across all processes
- **Memory MB**: Total memory usage by all processes

#### Process Classification
- **Audio Client Processes**: Processes using audio
- **GUI Window Processes**: Processes with graphical interfaces
- **Terminal Processes**: Processes with terminal sessions
- **SSH Processes**: SSH-related processes

#### Application Groups
- **Application Group Resources**: CPU and memory usage by application groups
- **Focused Application Groups**: Groups with focused windows
- **GUI Application Groups**: Groups with graphical interfaces

#### System Health & Performance
- **Health Monitoring**: Health score, critical and warning issues
- **API Performance**: Request rate and cache hit rate

#### Resource Optimization Metrics
- **Energy & GPU Usage**: Total energy consumption and GPU utilization
- **I/O Operations**: Read and write operations
- **Network Traffic**: Received and transmitted bytes
- **Daemon Iterations**: Total daemon iterations
- **Priority Adjustments**: Number of priority adjustments applied

## Key Metrics Explained

### System Metrics

| Metric | Description | Type |
|--------|-------------|------|
| `smoothtask_version` | SmoothTask daemon version | Gauge |
| `smoothtask_system_cpu_usage_percentage` | System CPU usage percentage | Gauge |
| `smoothtask_system_memory_total_kb` | Total system memory in KB | Gauge |
| `smoothtask_system_memory_used_kb` | Used system memory in KB | Gauge |
| `smoothtask_system_memory_available_kb` | Available system memory in KB | Gauge |
| `smoothtask_system_psi_cpu_some_avg10` | CPU PSI (Pressure Stall Information) 10s average | Gauge |
| `smoothtask_system_psi_memory_some_avg10` | Memory PSI 10s average | Gauge |

### Process Metrics

| Metric | Description | Type |
|--------|-------------|------|
| `smoothtask_processes_total` | Total number of processes | Gauge |
| `smoothtask_processes_total_cpu_share_1s` | Total CPU share (1s average) | Gauge |
| `smoothtask_processes_total_cpu_share_10s` | Total CPU share (10s average) | Gauge |
| `smoothtask_processes_total_memory_mb` | Total memory usage (RSS) in MB | Gauge |
| `smoothtask_processes_total_io_read_bytes` | Total read bytes | Counter |
| `smoothtask_processes_total_io_write_bytes` | Total write bytes | Counter |
| `smoothtask_processes_total_network_rx_bytes` | Total received bytes | Counter |
| `smoothtask_processes_total_network_tx_bytes` | Total transmitted bytes | Counter |
| `smoothtask_processes_total_gpu_utilization` | Total GPU utilization | Gauge |
| `smoothtask_processes_total_energy_uj` | Total energy consumption in microjoules | Counter |

### Process Classification Metrics

| Metric | Description | Type |
|--------|-------------|------|
| `smoothtask_processes_audio_client` | Audio client processes | Gauge |
| `smoothtask_processes_gui_window` | Processes with GUI windows | Gauge |
| `smoothtask_processes_terminal` | Processes with terminal sessions | Gauge |
| `smoothtask_processes_ssh` | SSH processes | Gauge |

### Application Group Metrics

| Metric | Description | Type |
|--------|-------------|------|
| `smoothtask_app_groups_total` | Total number of application groups | Gauge |
| `smoothtask_app_groups_total_cpu_share` | Total CPU share for all groups | Gauge |
| `smoothtask_app_groups_total_memory_mb` | Total memory usage for all groups | Gauge |
| `smoothtask_app_groups_total_io_read_bytes` | Total read bytes for all groups | Counter |
| `smoothtask_app_groups_total_io_write_bytes` | Total write bytes for all groups | Counter |
| `smoothtask_app_groups_total_network_rx_bytes` | Total received bytes for all groups | Counter |
| `smoothtask_app_groups_total_network_tx_bytes` | Total transmitted bytes for all groups | Counter |
| `smoothtask_app_groups_total_energy_uj` | Total energy consumption for all groups | Counter |
| `smoothtask_app_groups_focused` | Focused application groups | Gauge |
| `smoothtask_app_groups_with_gui` | Application groups with GUI windows | Gauge |

### Health Monitoring Metrics

| Metric | Description | Type |
|--------|-------------|------|
| `smoothtask_health_score` | System health score (0-100) | Gauge |
| `smoothtask_health_critical_issues` | Number of critical health issues | Gauge |
| `smoothtask_health_warning_issues` | Number of warning health issues | Gauge |
| `smoothtask_health_total_issues` | Total number of health issues | Gauge |

### Daemon Performance Metrics

| Metric | Description | Type |
|--------|-------------|------|
| `smoothtask_daemon_total_iterations` | Total number of daemon iterations | Counter |
| `smoothtask_daemon_successful_iterations` | Successful daemon iterations | Counter |
| `smoothtask_daemon_error_iterations` | Error daemon iterations | Counter |
| `smoothtask_daemon_total_duration_ms` | Total daemon execution time in milliseconds | Counter |
| `smoothtask_daemon_total_applied_adjustments` | Total number of priority adjustments applied | Counter |
| `smoothtask_daemon_total_apply_errors` | Total number of priority adjustment errors | Counter |

### API Performance Metrics

| Metric | Description | Type |
|--------|-------------|------|
| `smoothtask_api_requests_total` | Total number of API requests | Counter |
| `smoothtask_api_cache_hits` | Total number of API cache hits | Counter |
| `smoothtask_api_cache_misses` | Total number of API cache misses | Counter |
| `smoothtask_api_processing_time_us_total` | Total API processing time in microseconds | Counter |

## Example Queries

### System Resource Utilization

```promql
# Memory usage percentage
(smoothtask_system_memory_used_kb / smoothtask_system_memory_total_kb) * 100

# CPU usage rate (5m average)
rate(smoothtask_system_cpu_usage_percentage[5m])

# Memory usage rate of change
rate(smoothtask_system_memory_used_kb[5m])
```

### Process Performance Analysis

```promql
# CPU share per process (requires per-process metrics)
sum by(pid) (smoothtask_process_cpu_share_1s)

# Memory usage per process
sum by(pid) (smoothtask_process_memory_mb)

# Top 10 processes by CPU usage
topk(10, smoothtask_process_cpu_share_1s)
```

### Application Group Analysis

```promql
# CPU share by application group
sum by(app_group) (smoothtask_app_group_cpu_share)

# Memory usage by application group
sum by(app_group) (smoothtask_app_group_memory_mb)

# Network traffic by application group
sum by(app_group) (rate(smoothtask_app_group_network_rx_bytes[5m]) + rate(smoothtask_app_group_network_tx_bytes[5m]))
```

### Health Monitoring

```promql
# Health score trend
smoothtask_health_score

# Critical issues rate
rate(smoothtask_health_critical_issues[5m])

# Health issues by severity
sum by(severity) (smoothtask_health_issues_total)
```

### Performance Optimization Metrics

```promql
# Priority adjustment rate
rate(smoothtask_daemon_total_applied_adjustments[5m])

# API request rate
rate(smoothtask_api_requests_total[5m])

# Cache hit ratio
smoothtask_api_cache_hits / (smoothtask_api_cache_hits + smoothtask_api_cache_misses)
```

## Alerting Rules

The alert rules provided in the Prometheus configuration cover:

1. **Health Score Alerts**: Critical and degraded health states
2. **Critical Issues**: Immediate notification of critical problems
3. **Resource Usage**: High memory usage warnings
4. **Daemon Availability**: Detection of daemon downtime

### Custom Alert Examples

```yaml
# High CPU usage by application group
- alert: HighAppGroupCPUUsage
  expr: smoothtask_app_group_cpu_share > 80
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "High CPU usage by application group ({{ $value }}%)"
    description: "Application group is using more than 80% CPU"

# High memory usage by application group
- alert: HighAppGroupMemoryUsage
  expr: smoothtask_app_group_memory_mb > 2048
  for: 15m
  labels:
    severity: warning
  annotations:
    summary: "High memory usage by application group ({{ $value }} MB)"
    description: "Application group is using more than 2GB memory"

# High I/O activity
- alert: HighIOActivity
  expr: rate(smoothtask_processes_total_io_read_bytes[5m]) + rate(smoothtask_processes_total_io_write_bytes[5m]) > 10485760
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "High I/O activity detected ({{ $value }} bytes/s)"
    description: "System I/O activity exceeds 10MB/s"
```

### Comprehensive Alerting Rules

For a complete monitoring solution, we recommend the following comprehensive alerting rules:

```yaml
# ============================================================================
# SYSTEM HEALTH ALERTS
# ============================================================================

# Critical system health score
- alert: CriticalSystemHealth
  expr: smoothtask_health_score < 30
  for: 5m
  labels:
    severity: critical
  annotations:
    summary: "Critical system health score ({{ $value }})"
    description: "System health score has dropped below 30, indicating critical issues"

# Degraded system health score
- alert: DegradedSystemHealth
  expr: smoothtask_health_score < 70
  for: 15m
  labels:
    severity: warning
  annotations:
    summary: "Degraded system health score ({{ $value }})"
    description: "System health score has dropped below 70, indicating potential issues"

# Critical issues detected
- alert: CriticalIssuesDetected
  expr: smoothtask_health_critical_issues > 0
  for: 1m
  labels:
    severity: critical
  annotations:
    summary: "Critical issues detected ({{ $value }})"
    description: "SmoothTask has detected {{ $value }} critical issues requiring immediate attention"

# ============================================================================
# RESOURCE USAGE ALERTS
# ============================================================================

# High system memory usage
- alert: HighSystemMemoryUsage
  expr: (smoothtask_system_memory_total_mb - smoothtask_system_memory_available_mb) / smoothtask_system_memory_total_mb * 100 > 90
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "High system memory usage ({{ $value }}%)"
    description: "System memory usage exceeds 90%"

# High system CPU usage
- alert: HighSystemCPUUsage
  expr: 100 - (avg by (instance) (rate(smoothtask_system_cpu_idle_seconds_total[1m])) * 100) > 90
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "High system CPU usage ({{ $value }}%)"
    description: "System CPU usage exceeds 90%"

# High PSI memory pressure
- alert: HighMemoryPressure
  expr: smoothtask_system_psi_memory_some_avg10 > 25
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "High memory pressure detected ({{ $value }}%)"
    description: "Memory pressure (some) exceeds 25% over 10-second average"

# ============================================================================
# PROCESS AND APPLICATION ALERTS
# ============================================================================

# High CPU usage by individual process
- alert: HighProcessCPUUsage
  expr: smoothtask_process_cpu_share > 50
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "High CPU usage by process ({{ $value }}%)"
    description: "Process is using more than 50% CPU"

# High memory usage by individual process
- alert: HighProcessMemoryUsage
  expr: smoothtask_process_memory_rss_mb > 1024
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "High memory usage by process ({{ $value }} MB)"
    description: "Process is using more than 1GB memory"

# High I/O usage by process
- alert: HighProcessIOUsage
  expr: rate(smoothtask_process_io_read_bytes[1m]) + rate(smoothtask_process_io_write_bytes[1m]) > 5242880
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "High I/O usage by process ({{ $value }} bytes/s)"
    description: "Process I/O activity exceeds 5MB/s"

# ============================================================================
# DAEMON AND SERVICE ALERTS
# ============================================================================

# SmoothTask daemon not responding
- alert: SmoothTaskDaemonDown
  expr: up{job="smoothtask"} == 0
  for: 2m
  labels:
    severity: critical
  annotations:
    summary: "SmoothTask daemon is down"
    description: "SmoothTask daemon has not responded for 2 minutes"

# High daemon iteration time
- alert: HighDaemonIterationTime
  expr: smoothtask_daemon_iteration_time_seconds > 5
  for: 3m
  labels:
    severity: warning
  annotations:
    summary: "High daemon iteration time ({{ $value }}s)"
    description: "Daemon iteration time exceeds 5 seconds"

# High priority adjustments
- alert: HighPriorityAdjustments
  expr: rate(smoothtask_daemon_priority_adjustments_total[5m]) > 10
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "High priority adjustments ({{ $value }}/s)"
    description: "Daemon is making more than 10 priority adjustments per second"

# ============================================================================
# AUDIO AND MULTIMEDIA ALERTS
# ============================================================================

# Audio XRUN detection
- alert: AudioXRUNDetected
  expr: increase(smoothtask_audio_xruns_total[1m]) > 0
  for: 1m
  labels:
    severity: warning
  annotations:
    summary: "Audio XRUN detected ({{ $value }})"
    description: "Audio underruns detected, indicating potential audio glitches"

# High audio latency
- alert: HighAudioLatency
  expr: smoothtask_audio_latency_ms > 50
  for: 2m
  labels:
    severity: warning
  annotations:
    summary: "High audio latency ({{ $value }}ms)"
    description: "Audio latency exceeds 50ms"

# ============================================================================
# NETWORK ALERTS
# ============================================================================

# High network traffic
- alert: HighNetworkTraffic
  expr: rate(smoothtask_network_receive_bytes_total[1m]) + rate(smoothtask_network_transmit_bytes_total[1m]) > 10485760
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "High network traffic ({{ $value }} bytes/s)"
    description: "Network traffic exceeds 10MB/s"

# High network errors
- alert: HighNetworkErrors
  expr: rate(smoothtask_network_receive_errors_total[1m]) + rate(smoothtask_network_transmit_errors_total[1m]) > 10
  for: 2m
  labels:
    severity: warning
  annotations:
    summary: "High network errors ({{ $value }} errors/s)"
    description: "Network errors exceed 10 per second"
```

### Alert Configuration Best Practices

1. **Alert Thresholds**: Adjust thresholds based on your system's normal operating conditions
2. **Notification Routing**: Route critical alerts to appropriate teams/channels
3. **Alert Grouping**: Group related alerts to reduce notification noise
4. **Silencing**: Implement alert silencing during maintenance windows
5. **Escalation**: Set up escalation policies for unacknowledged alerts

### Alert Integration Examples

#### Email Notifications

```yaml
# prometheus.yml alertmanager configuration
alertmanager:
  static_configs:
    - targets: ['alertmanager:9093']

# alertmanager.yml configuration
route:
  group_by: ['alertname', 'severity']
  group_wait: 30s
  group_interval: 5m
  repeat_interval: 3h
  receiver: 'email-notifications'

receivers:
  - name: 'email-notifications'
    email_configs:
      - to: 'admin@example.com'
        from: 'alertmanager@example.com'
        smarthost: 'smtp.example.com:587'
        auth_username: 'alertmanager@example.com'
        auth_password: 'password'
```

#### Slack Notifications

```yaml
receivers:
  - name: 'slack-notifications'
    slack_configs:
      - api_url: 'https://hooks.slack.com/services/XXX/YYY/ZZZ'
        channel: '#alerts'
        title: '{{ template "slack.title" . }}'
        text: '{{ template "slack.text" . }}'
```

#### PagerDuty Integration

```yaml
receivers:
  - name: 'pagerduty-critical'
    pagerduty_configs:
      - routing_key: 'your-pagerduty-routing-key'
        severity: 'critical'
        client: 'SmoothTask'
        client_url: 'https://your-smoothtask-dashboard'
```

## Troubleshooting

### Common Issues and Solutions

#### Prometheus Cannot Scrape SmoothTask Metrics

**Symptoms**:
- Target shows as `DOWN` in Prometheus
- No metrics appear in Grafana

**Solutions**:
1. Verify SmoothTask API is running: `curl http://localhost:8080/metrics`
2. Check network connectivity between Prometheus and SmoothTask
3. Verify firewall rules allow traffic on port 8080
4. Check SmoothTask logs for errors

#### Grafana Dashboard Shows No Data

**Symptoms**:
- Dashboard panels are empty
- "No data" messages in Grafana

**Solutions**:
1. Verify Prometheus data source is configured correctly
2. Check that metrics exist in Prometheus: `curl http://localhost:9090/api/v1/query?query=smoothtask_version`
3. Verify time range in Grafana includes current data
4. Check that the dashboard is using the correct data source

#### High Cardinality Issues

**Symptoms**:
- Prometheus uses excessive memory
- Slow query performance

**Solutions**:
1. Limit label cardinality in SmoothTask configuration
2. Use recording rules to pre-aggregate metrics
3. Adjust Prometheus retention settings
4. Consider using Prometheus federation for large deployments

#### Authentication Issues

**Symptoms**:
- 401/403 errors when scraping metrics
- Authentication failures

**Solutions**:
1. Configure basic auth in Prometheus scrape config:
   ```yaml
   basic_auth:
     username: "your_username"
     password: "your_password"
   ```
2. Ensure SmoothTask API has proper authentication configured
3. Check that credentials are correct

### Debugging Commands

```bash
# Check SmoothTask metrics endpoint
curl -v http://localhost:8080/metrics

# Check Prometheus targets
curl -s http://localhost:9090/api/v1/targets | jq '.'

# Query specific metric
curl -s "http://localhost:9090/api/v1/query?query=smoothtask_version"

# Check Prometheus logs
journalctl -u prometheus -f

# Check Grafana logs
journalctl -u grafana-server -f

# Test Prometheus configuration
promtool check config /etc/prometheus/prometheus.yml
```

## Advanced Configuration

### Prometheus Recording Rules

Add recording rules to pre-aggregate metrics and reduce load:

```yaml
rule_files:
  - "recording.rules"
```

Example recording rules (`recording.rules`):

```yaml
groups:
- name: smoothtask-recording-rules
  interval: 30s
  rules:
  
  # 5-minute averages
  - record: smoothtask:process_cpu_5m
    expr: avg_over_time(smoothtask_processes_total_cpu_share_1s[5m])
  
  - record: smoothtask:process_memory_5m
    expr: avg_over_time(smoothtask_processes_total_memory_mb[5m])
  
  # 1-hour averages
  - record: smoothtask:process_cpu_1h
    expr: avg_over_time(smoothtask_processes_total_cpu_share_1s[1h])
  
  - record: smoothtask:health_score_1h
    expr: avg_over_time(smoothtask_health_score[1h])
```

### Prometheus Federation

For large deployments, consider Prometheus federation:

```yaml
# In your central Prometheus config
scrape_configs:
  - job_name: 'federate'
    scrape_interval: 15s
    honor_labels: true
    metrics_path: '/federate'
    params:
      'match[]':
        - '{__name__=~"smoothtask:.*"}'
        - '{__name__=~"smoothtask_health.*"}'
        - '{__name__=~"smoothtask_processes.*"}'
    static_configs:
      - targets: ['prometheus-server-1:9090']
      - targets: ['prometheus-server-2:9090']
```

### Long-term Storage

Configure Prometheus for long-term storage:

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s
  # Keep metrics for 30 days
  storage.tsdb.retention.time: 30d
  storage.tsdb.retention.size: 50GB
```

## Security Considerations

### Secure SmoothTask API

1. **Enable HTTPS**: Configure SmoothTask API with TLS
2. **Authentication**: Enable basic auth or token authentication
3. **Firewall**: Restrict access to the metrics endpoint
4. **Rate Limiting**: Configure rate limiting for the API

### Secure Prometheus

1. **Authentication**: Enable basic auth for Prometheus
2. **TLS**: Configure HTTPS for Prometheus
3. **Network Isolation**: Run Prometheus in a secure network segment
4. **RBAC**: Configure role-based access control

### Secure Grafana

1. **Change Default Password**: Immediately change the admin password
2. **Enable HTTPS**: Configure Grafana with TLS
3. **Authentication**: Enable OAuth or LDAP integration
4. **RBAC**: Configure proper user roles and permissions
5. **Network Security**: Restrict Grafana access to authorized networks

## Performance Optimization

### Prometheus Performance

1. **Scrape Interval**: Adjust based on your needs (15s-60s)
2. **Retention**: Set appropriate retention periods
3. **Compaction**: Schedule regular compaction
4. **Sharding**: Consider sharding for large deployments

### Grafana Performance

1. **Dashboard Optimization**: Limit time ranges and panel density
2. **Caching**: Enable Grafana caching
3. **Data Source Limits**: Configure query timeouts
4. **Dashboard Cleanup**: Remove unused dashboards

## Upgrading

### Upgrading Prometheus

1. Backup configuration and data
2. Stop Prometheus service
3. Install new version
4. Restore configuration
5. Start Prometheus
6. Verify metrics continuity

### Upgrading Grafana

1. Backup Grafana database
2. Stop Grafana service
3. Install new version
4. Restore database
5. Start Grafana
6. Verify dashboards and data sources

## Backup and Recovery

### Prometheus Backup

```bash
# Backup Prometheus data
sudo systemctl stop prometheus
sudo tar -czvf prometheus_backup_$(date +%Y%m%d).tar.gz /var/lib/prometheus/
sudo systemctl start prometheus

# Restore Prometheus data
sudo systemctl stop prometheus
sudo rm -rf /var/lib/prometheus/*
sudo tar -xzvf prometheus_backup.tar.gz -C /
sudo systemctl start prometheus
```

### Grafana Backup

```bash
# Backup Grafana database (SQLite)
sudo systemctl stop grafana-server
sudo cp /var/lib/grafana/grafana.db /backup/grafana_db_backup_$(date +%Y%m%d).db
sudo systemctl start grafana-server

# For MySQL/PostgreSQL, use appropriate database backup tools
```

## Monitoring Best Practices

### Dashboard Organization

1. **Use Folders**: Organize dashboards by category
2. **Consistent Naming**: Use clear, consistent naming conventions
3. **Documentation**: Add descriptions to dashboards and panels
4. **Tags**: Use tags for easy filtering

### Alert Management

1. **Alert Fatigue**: Avoid excessive alerts
2. **Severity Levels**: Use appropriate severity levels
3. **Alert Routing**: Route alerts to appropriate teams
4. **Alert Documentation**: Document alert resolution procedures

### Metrics Retention

1. **Hot Data**: Keep recent data (1-7 days) for detailed analysis
2. **Warm Data**: Keep medium-term data (7-30 days) for trends
3. **Cold Data**: Archive long-term data (>30 days) for compliance

## Conclusion

This comprehensive monitoring setup provides:

- **Real-time Visibility**: Into SmoothTask performance and system health
- **Historical Analysis**: For trend analysis and capacity planning
- **Proactive Alerting**: For early issue detection
- **Performance Optimization**: Through detailed metrics analysis

The SmoothTask Overview Dashboard gives you a complete view of your system's performance, process behavior, and resource optimization metrics, enabling you to make data-driven decisions for optimal system performance.

## Additional Resources

- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)
- [SmoothTask API Documentation](https://github.com/your-repo/smoothtask/docs/API.md)
- [PromQL Query Language](https://prometheus.io/docs/prometheus/latest/querying/basics/)

## Support

For issues with SmoothTask monitoring:

1. Check SmoothTask logs for errors
2. Verify Prometheus and Grafana configurations
3. Consult the troubleshooting section
4. Open an issue on the SmoothTask GitHub repository

For Prometheus/Grafana issues:

1. Consult official documentation
2. Check community forums
3. Review Stack Overflow for common issues
