//! Comprehensive Network Monitoring Module
//!
//! This module provides advanced network monitoring capabilities including:
//! - Detailed network interface monitoring
//! - Network traffic analysis
//! - Protocol-level monitoring
//! - Port-based monitoring
//! - Connection tracking and analysis
//! - Network quality metrics

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::{Duration, SystemTime};

/// Comprehensive network interface statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkInterfaceStats {
    /// Interface name (e.g., "eth0", "wlan0")
    pub name: String,
    /// Interface type (ethernet, wifi, loopback, etc.)
    pub interface_type: NetworkInterfaceType,
    /// MAC address (if available)
    pub mac_address: Option<String>,
    /// IP addresses assigned to this interface
    pub ip_addresses: Vec<IpAddr>,
    /// Interface speed in Mbps (if available)
    pub speed_mbps: Option<u64>,
    /// Interface state (up/down)
    pub is_up: bool,
    /// Bytes received
    pub rx_bytes: u64,
    /// Bytes transmitted
    pub tx_bytes: u64,
    /// Packets received
    pub rx_packets: u64,
    /// Packets transmitted
    pub tx_packets: u64,
    /// Receive errors
    pub rx_errors: u64,
    /// Transmit errors
    pub tx_errors: u64,
    /// Receive dropped packets
    pub rx_dropped: u64,
    /// Transmit dropped packets
    pub tx_dropped: u64,
    /// Receive overruns
    pub rx_overruns: u64,
    /// Transmit overruns
    pub tx_overruns: u64,
    /// Interface flags
    pub flags: u32,
    /// Timestamp of last update
    pub timestamp: SystemTime,
}

/// Network interface type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub enum NetworkInterfaceType {
    Ethernet,
    Wifi,
    Loopback,
    Virtual,
    Tunnel,
    Bridge,
    #[default]
    Unknown,
}

impl Default for NetworkInterfaceStats {
    fn default() -> Self {
        Self {
            name: String::new(),
            interface_type: NetworkInterfaceType::default(),
            mac_address: None,
            ip_addresses: Vec::new(),
            speed_mbps: None,
            is_up: false,
            rx_bytes: 0,
            tx_bytes: 0,
            rx_packets: 0,
            tx_packets: 0,
            rx_errors: 0,
            tx_errors: 0,
            rx_dropped: 0,
            tx_dropped: 0,
            rx_overruns: 0,
            tx_overruns: 0,
            flags: 0,
            timestamp: SystemTime::UNIX_EPOCH,
        }
    }
}

impl Default for NetworkConnectionStats {
    fn default() -> Self {
        Self {
            src_ip: "0.0.0.0".parse().unwrap(),
            dst_ip: "0.0.0.0".parse().unwrap(),
            src_port: 0,
            dst_port: 0,
            protocol: String::new(),
            state: String::new(),
            pid: None,
            process_name: None,
            bytes_transmitted: 0,
            bytes_received: 0,
            packets_transmitted: 0,
            packets_received: 0,
            start_time: SystemTime::UNIX_EPOCH,
            last_activity: SystemTime::UNIX_EPOCH,
            duration: Duration::from_secs(0),
        }
    }
}

impl Default for ComprehensiveNetworkStats {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::UNIX_EPOCH,
            interfaces: Vec::new(),
            protocols: NetworkProtocolStats::default(),
            port_usage: Vec::new(),
            active_connections: Vec::new(),
            quality: NetworkQualityMetrics::default(),
            total_rx_bytes: 0,
            total_tx_bytes: 0,
            total_rx_packets: 0,
            total_tx_packets: 0,
        }
    }
}

/// Network protocol statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NetworkProtocolStats {
    /// TCP connections count
    pub tcp_connections: u64,
    /// UDP connections count
    pub udp_connections: u64,
    /// ICMP packets
    pub icmp_packets: u64,
    /// TCP retransmissions
    pub tcp_retransmissions: u64,
    /// TCP errors
    pub tcp_errors: u64,
    /// UDP errors
    pub udp_errors: u64,
}

/// Port usage statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PortUsageStats {
    /// Port number
    pub port: u16,
    /// Protocol (TCP/UDP)
    pub protocol: String,
    /// Number of connections using this port
    pub connection_count: u64,
    /// Total bytes transmitted through this port
    pub bytes_transmitted: u64,
    /// Total bytes received through this port
    pub bytes_received: u64,
    /// Processes using this port
    pub processes: Vec<u32>,
}

/// Network connection statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkConnectionStats {
    /// Source IP address
    pub src_ip: IpAddr,
    /// Destination IP address
    pub dst_ip: IpAddr,
    /// Source port
    pub src_port: u16,
    /// Destination port
    pub dst_port: u16,
    /// Protocol (TCP/UDP)
    pub protocol: String,
    /// Connection state
    pub state: String,
    /// Process ID (if available)
    pub pid: Option<u32>,
    /// Process name (if available)
    pub process_name: Option<String>,
    /// Bytes transmitted
    pub bytes_transmitted: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Packets transmitted
    pub packets_transmitted: u64,
    /// Packets received
    pub packets_received: u64,
    /// Connection start time
    pub start_time: SystemTime,
    /// Last activity time
    pub last_activity: SystemTime,
    /// Connection duration
    pub duration: Duration,
}

/// Network quality metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NetworkQualityMetrics {
    /// Packet loss percentage
    pub packet_loss: f64,
    /// Latency in milliseconds
    pub latency_ms: f64,
    /// Jitter in milliseconds
    pub jitter_ms: f64,
    /// Bandwidth utilization percentage
    pub bandwidth_utilization: f64,
    /// Connection stability score (0.0 to 1.0)
    pub stability_score: f64,
}

/// Comprehensive network statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComprehensiveNetworkStats {
    /// Timestamp of collection
    pub timestamp: SystemTime,
    /// Network interfaces statistics
    pub interfaces: Vec<NetworkInterfaceStats>,
    /// Protocol statistics
    pub protocols: NetworkProtocolStats,
    /// Port usage statistics
    pub port_usage: Vec<PortUsageStats>,
    /// Active connections
    pub active_connections: Vec<NetworkConnectionStats>,
    /// Network quality metrics
    pub quality: NetworkQualityMetrics,
    /// Total bytes received
    pub total_rx_bytes: u64,
    /// Total bytes transmitted
    pub total_tx_bytes: u64,
    /// Total packets received
    pub total_rx_packets: u64,
    /// Total packets transmitted
    pub total_tx_packets: u64,
}

/// Network monitoring configuration
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct NetworkMonitorConfig {
    /// Enable detailed interface monitoring
    pub enable_detailed_interfaces: bool,
    /// Enable protocol-level monitoring
    pub enable_protocol_monitoring: bool,
    /// Enable port-level monitoring
    pub enable_port_monitoring: bool,
    /// Enable connection tracking
    pub enable_connection_tracking: bool,
    /// Enable network quality monitoring
    pub enable_quality_monitoring: bool,
    /// Maximum number of connections to track
    pub max_connections: usize,
    /// Update interval in seconds
    pub update_interval_secs: u64,
    /// Ports to monitor specifically
    pub monitored_ports: Vec<u16>,
    /// Protocols to monitor specifically
    pub monitored_protocols: Vec<String>,
}

impl Default for NetworkMonitorConfig {
    fn default() -> Self {
        Self {
            enable_detailed_interfaces: true,
            enable_protocol_monitoring: true,
            enable_port_monitoring: true,
            enable_connection_tracking: true,
            enable_quality_monitoring: true,
            max_connections: 1024,
            update_interval_secs: 60,
            monitored_ports: vec![80, 443, 22, 53, 8080],
            monitored_protocols: vec!["TCP".to_string(), "UDP".to_string()],
        }
    }
}

/// Network monitor main structure
pub struct NetworkMonitor {
    config: NetworkMonitorConfig,
    previous_stats: Option<ComprehensiveNetworkStats>,
    interface_cache: HashMap<String, NetworkInterfaceStats>,
    cache_ttl: Duration,
    last_cache_update: SystemTime,
}

impl Default for NetworkMonitor {
    fn default() -> Self {
        Self {
            config: NetworkMonitorConfig::default(),
            previous_stats: None,
            interface_cache: HashMap::new(),
            cache_ttl: Duration::from_secs(5),
            last_cache_update: SystemTime::UNIX_EPOCH,
        }
    }
}

impl NetworkMonitor {
    /// Create a new NetworkMonitor with default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new NetworkMonitor with custom configuration
    pub fn with_config(config: NetworkMonitorConfig) -> Self {
        Self {
            config,
            previous_stats: None,
            interface_cache: HashMap::new(),
            cache_ttl: Duration::from_secs(5),
            last_cache_update: SystemTime::UNIX_EPOCH,
        }
    }

    /// Create a new NetworkMonitor with custom configuration and cache TTL
    pub fn with_config_and_cache(config: NetworkMonitorConfig, cache_ttl_secs: u64) -> Self {
        Self {
            config,
            previous_stats: None,
            interface_cache: HashMap::new(),
            cache_ttl: Duration::from_secs(cache_ttl_secs),
            last_cache_update: SystemTime::UNIX_EPOCH,
        }
    }

    /// Clear the interface cache
    pub fn clear_interface_cache(&mut self) {
        self.interface_cache.clear();
        self.last_cache_update = SystemTime::UNIX_EPOCH;
    }

    /// Check if interface cache is valid
    fn is_interface_cache_valid(&self) -> bool {
        if self.interface_cache.is_empty() {
            return false;
        }
        
        match self.last_cache_update.elapsed() {
            Ok(elapsed) => elapsed < self.cache_ttl,
            Err(_) => false, // Cache is too old or system time issue
        }
    }

    /// Collect comprehensive network statistics
    pub fn collect_network_stats(&mut self) -> Result<ComprehensiveNetworkStats> {
        let mut stats = ComprehensiveNetworkStats {
            timestamp: SystemTime::now(),
            ..Default::default()
        };

        // Collect interface statistics with caching
        if self.config.enable_detailed_interfaces {
            stats.interfaces = self.collect_interface_stats_optimized()?;
        }

        // Collect protocol statistics
        if self.config.enable_protocol_monitoring {
            stats.protocols = self.collect_protocol_stats()?;
        }

        // Collect port usage statistics
        if self.config.enable_port_monitoring {
            stats.port_usage = self.collect_port_usage_stats()?;
        }

        // Collect connection statistics
        if self.config.enable_connection_tracking {
            stats.active_connections = self.collect_connection_stats()?;
        }

        // Collect network quality metrics
        if self.config.enable_quality_monitoring {
            stats.quality = self.collect_network_quality_metrics()?;
        }

        // Calculate totals
        stats.total_rx_bytes = stats.interfaces.iter().map(|iface| iface.rx_bytes).sum();
        stats.total_tx_bytes = stats.interfaces.iter().map(|iface| iface.tx_bytes).sum();
        stats.total_rx_packets = stats.interfaces.iter().map(|iface| iface.rx_packets).sum();
        stats.total_tx_packets = stats.interfaces.iter().map(|iface| iface.tx_packets).sum();

        // Store current stats for next collection
        self.previous_stats = Some(stats.clone());

        Ok(stats)
    }

    /// Optimized interface statistics collection with caching
    fn collect_interface_stats_optimized(&mut self) -> Result<Vec<NetworkInterfaceStats>> {
        // Check if we can use cached interface data
        if self.is_interface_cache_valid() {
            tracing::debug!("Using cached interface data (cache TTL: {}s)", self.cache_ttl.as_secs());
            return Ok(self.interface_cache.values().cloned().collect());
        }

        // Cache is invalid or empty, collect fresh data
        let interfaces = self.collect_interface_stats()?;
        
        // Update cache with fresh data
        self.interface_cache.clear();
        for iface in &interfaces {
            self.interface_cache.insert(iface.name.clone(), iface.clone());
        }
        self.last_cache_update = SystemTime::now();
        
        tracing::debug!("Updated interface cache with {} interfaces", interfaces.len());
        
        Ok(interfaces)
    }

    /// Collect interface statistics from /proc/net/dev and /sys/class/net
    fn collect_interface_stats(&self) -> Result<Vec<NetworkInterfaceStats>> {
        let mut interfaces = Vec::new();

        // Read from /proc/net/dev with better error handling
        let proc_net_dev = fs::read_to_string("/proc/net/dev")
            .context("Failed to read /proc/net/dev")?;

        // Pre-allocate capacity based on typical number of interfaces
        interfaces.reserve(8); // Most systems have 2-8 interfaces

        // Parse interface statistics with validation using optimized parsing
        for (line_num, line) in proc_net_dev.lines().skip(2).enumerate() { // Skip header lines
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 17 {
                let name = parts[0].trim_end_matches(':').to_string();
                
                // Validate interface name
                if name.is_empty() {
                    tracing::warn!("Empty interface name found at line {}", line_num + 2);
                    continue;
                }

                // Use optimized methods for interface info collection
                let interface_type = self.detect_interface_type(&name);
                let is_up = self.check_interface_up_optimized(&name);
                let mac_address = self.get_mac_address_optimized(&name);
                let ip_addresses = self.get_ip_addresses_optimized(&name);
                let speed_mbps = self.get_interface_speed_optimized(&name);

                // Parse numeric values with error handling using fast parsing
                let rx_bytes = self.parse_u64_fast(parts[1], &name, "rx_bytes", line_num + 2)?;
                let tx_bytes = self.parse_u64_fast(parts[9], &name, "tx_bytes", line_num + 2)?;
                let rx_packets = self.parse_u64_fast(parts[2], &name, "rx_packets", line_num + 2)?;
                let tx_packets = self.parse_u64_fast(parts[10], &name, "tx_packets", line_num + 2)?;
                let rx_errors = self.parse_u64_fast(parts[3], &name, "rx_errors", line_num + 2)?;
                let tx_errors = self.parse_u64_fast(parts[11], &name, "tx_errors", line_num + 2)?;
                let rx_dropped = self.parse_u64_fast(parts[4], &name, "rx_dropped", line_num + 2)?;
                let tx_dropped = self.parse_u64_fast(parts[12], &name, "tx_dropped", line_num + 2)?;
                let rx_overruns = self.parse_u64_fast(parts[5], &name, "rx_overruns", line_num + 2)?;
                let tx_overruns = self.parse_u64_fast(parts[13], &name, "tx_overruns", line_num + 2)?;

                let mut iface = NetworkInterfaceStats {
                    name: name.clone(),
                    interface_type,
                    mac_address,
                    ip_addresses,
                    speed_mbps,
                    is_up,
                    rx_bytes,
                    tx_bytes,
                    rx_packets,
                    tx_packets,
                    rx_errors,
                    tx_errors,
                    rx_dropped,
                    tx_dropped,
                    rx_overruns,
                    tx_overruns,
                    flags: 0, // Will be populated from sysfs
                    timestamp: SystemTime::now(),
                };

                // Get additional info from sysfs with error handling
                match self.get_interface_flags_optimized(&name) {
                    Ok(flags) => {
                        iface.flags = flags;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to get interface flags for {}: {}", name, e);
                        // Continue with default flags
                    }
                }

                interfaces.push(iface);
            } else {
                tracing::debug!("Skipping line {}: insufficient data (expected >= 17 fields, got {})", line_num + 2, parts.len());
            }
        }

        if interfaces.is_empty() {
            tracing::warn!("No network interfaces found in /proc/net/dev");
        }

        Ok(interfaces)
    }

    /// Fast u64 parsing with error handling
    fn parse_u64_fast(&self, s: &str, interface_name: &str, field_name: &str, line_num: usize) -> Result<u64> {
        s.parse::<u64>()
            .with_context(|| format!("Failed to parse {} for interface {} at line {}", field_name, interface_name, line_num))
    }

    /// Optimized interface up check with reduced filesystem operations
    fn check_interface_up_optimized(&self, name: &str) -> bool {
        let operstate_path = format!("/sys/class/net/{}/operstate", name);
        match fs::read_to_string(operstate_path) {
            Ok(s) => s.trim() == "up",
            Err(e) => {
                tracing::debug!("Failed to read operstate for {}: {}", name, e);
                false
            }
        }
    }

    /// Optimized MAC address retrieval
    fn get_mac_address_optimized(&self, name: &str) -> Option<String> {
        let address_path = format!("/sys/class/net/{}/address", name);
        match fs::read_to_string(address_path) {
            Ok(s) => {
                let mac = s.trim().to_string();
                if mac.is_empty() || mac == "00:00:00:00:00:00" {
                    None
                } else {
                    Some(mac)
                }
            }
            Err(e) => {
                tracing::debug!("Failed to read MAC address for {}: {}", name, e);
                None
            }
        }
    }

    /// Optimized IP address retrieval
    fn get_ip_addresses_optimized(&self, name: &str) -> Vec<IpAddr> {
        let mut addresses = Vec::with_capacity(4); // Most interfaces have 1-4 addresses

        // Try IPv4 with error handling
        let ipv4_path = format!("/sys/class/net/{}/inet4", name);
        if let Ok(ipv4_dir) = fs::read_dir(ipv4_path) {
            for entry in ipv4_dir {
                match entry {
                    Ok(entry) => {
                        if let Some(ip_str) = entry.file_name().to_str() {
                            if let Ok(ip) = ip_str.parse::<Ipv4Addr>() {
                                addresses.push(IpAddr::V4(ip));
                            } else {
                                tracing::debug!("Failed to parse IPv4 address '{}' for interface {}", ip_str, name);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::debug!("Error reading IPv4 directory entry for {}: {}", name, e);
                    }
                }
            }
        }

        // Try IPv6 with error handling
        let ipv6_path = format!("/sys/class/net/{}/inet6", name);
        if let Ok(ipv6_dir) = fs::read_dir(ipv6_path) {
            for entry in ipv6_dir {
                match entry {
                    Ok(entry) => {
                        if let Some(ip_str) = entry.file_name().to_str() {
                            if let Ok(ip) = ip_str.parse::<Ipv6Addr>() {
                                addresses.push(IpAddr::V6(ip));
                            } else {
                                tracing::debug!("Failed to parse IPv6 address '{}' for interface {}", ip_str, name);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::debug!("Error reading IPv6 directory entry for {}: {}", name, e);
                    }
                }
            }
        }

        addresses
    }

    /// Optimized interface speed retrieval
    fn get_interface_speed_optimized(&self, name: &str) -> Option<u64> {
        let speed_path = format!("/sys/class/net/{}/speed", name);
        match fs::read_to_string(speed_path) {
            Ok(s) => {
                match s.trim().parse::<u64>() {
                    Ok(speed) => Some(speed),
                    Err(e) => {
                        tracing::debug!("Failed to parse interface speed for {}: {}", name, e);
                        None
                    }
                }
            }
            Err(e) => {
                tracing::debug!("Failed to read interface speed for {}: {}", name, e);
                None
            }
        }
    }

    /// Optimized interface flags retrieval
    fn get_interface_flags_optimized(&self, name: &str) -> Result<u32> {
        let flags_path = format!("/sys/class/net/{}/flags", name);
        let flags_str = fs::read_to_string(flags_path)
            .with_context(|| format!("Failed to read flags for interface {}", name))?;
        let flags = flags_str.trim().parse::<u32>()
            .with_context(|| format!("Failed to parse flags for interface {}", name))?;
        Ok(flags)
    }

    /// Detect interface type based on name
    fn detect_interface_type(&self, name: &str) -> NetworkInterfaceType {
        if name.starts_with("lo") {
            NetworkInterfaceType::Loopback
        } else if name.starts_with("eth") || name.starts_with("en") {
            NetworkInterfaceType::Ethernet
        } else if name.starts_with("wlan") || name.starts_with("wl") {
            NetworkInterfaceType::Wifi
        } else if name.starts_with("vir") || name.starts_with("veth") {
            NetworkInterfaceType::Virtual
        } else if name.starts_with("tun") || name.starts_with("tap") {
            NetworkInterfaceType::Tunnel
        } else if name.starts_with("br") {
            NetworkInterfaceType::Bridge
        } else {
            NetworkInterfaceType::Unknown
        }
    }



    /// Collect protocol statistics from /proc/net/snmp
    fn collect_protocol_stats(&self) -> Result<NetworkProtocolStats> {
        let mut stats = NetworkProtocolStats::default();

        // Read TCP statistics with error handling
        let tcp_stats = fs::read_to_string("/proc/net/snmp")
            .context("Failed to read /proc/net/snmp")?;

        for (line_num, line) in tcp_stats.lines().enumerate() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let protocol = parts[0].trim_end_matches(':');
                match protocol {
                    "Tcp" => {
                        if parts.len() >= 16 {
                            stats.tcp_connections = parts[15].parse::<u64>()
                                .with_context(|| format!("Failed to parse TCP connections at line {}", line_num))?;
                            stats.tcp_retransmissions = parts[12].parse::<u64>()
                                .with_context(|| format!("Failed to parse TCP retransmissions at line {}", line_num))?;
                        } else {
                            tracing::debug!("Insufficient TCP data at line {}: expected >= 16 fields, got {}", line_num, parts.len());
                        }
                    }
                    "Udp" => {
                        if parts.len() >= 4 {
                            stats.udp_connections = parts[3].parse::<u64>()
                                .with_context(|| format!("Failed to parse UDP connections at line {}", line_num))?;
                        } else {
                            tracing::debug!("Insufficient UDP data at line {}: expected >= 4 fields, got {}", line_num, parts.len());
                        }
                    }
                    "Icmp" => {
                        if parts.len() >= 3 {
                            stats.icmp_packets = parts[2].parse::<u64>()
                                .with_context(|| format!("Failed to parse ICMP packets at line {}", line_num))?;
                        } else {
                            tracing::debug!("Insufficient ICMP data at line {}: expected >= 3 fields, got {}", line_num, parts.len());
                        }
                    }
                    _ => {
                        tracing::debug!("Unknown protocol '{}' at line {}", protocol, line_num);
                    }
                }
            }
        }

        tracing::debug!("Collected protocol stats: TCP={} connections, UDP={} connections, ICMP={} packets",
                      stats.tcp_connections, stats.udp_connections, stats.icmp_packets);

        Ok(stats)
    }

    /// Collect port usage statistics
    fn collect_port_usage_stats(&self) -> Result<Vec<PortUsageStats>> {
        let mut port_stats = Vec::new();

        // This would typically be enhanced with eBPF or netstat-like functionality
        // For now, we'll provide a basic implementation
        
        // Add common ports that we're monitoring
        for &port in &self.config.monitored_ports {
            port_stats.push(PortUsageStats {
                port,
                protocol: "TCP".to_string(),
                connection_count: 0, // Would be populated with real data
                bytes_transmitted: 0,
                bytes_received: 0,
                processes: Vec::new(),
            });
        }

        Ok(port_stats)
    }

    /// Collect connection statistics
    fn collect_connection_stats(&self) -> Result<Vec<NetworkConnectionStats>> {
        let mut connections = Vec::new();

        // This would typically use eBPF or /proc/net/tcp, /proc/net/udp
        // For now, we'll provide a basic implementation with better error handling
        
        // Try to read TCP connections with error handling
        match fs::read_to_string("/proc/net/tcp") {
            Ok(tcp_connections) => {
                for (line_num, line) in tcp_connections.lines().skip(1).enumerate() { // Skip header
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 10 {
                        // Parse connection info - this is simplified
                        // In a real implementation, we'd parse the hex addresses/ports
                        connections.push(NetworkConnectionStats {
                            src_ip: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                            dst_ip: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                            src_port: 0,
                            dst_port: 0,
                            protocol: "TCP".to_string(),
                            state: parts[3].to_string(), // Connection state
                            pid: None,
                            process_name: None,
                            bytes_transmitted: 0,
                            bytes_received: 0,
                            packets_transmitted: 0,
                            packets_received: 0,
                            start_time: SystemTime::now(),
                            last_activity: SystemTime::now(),
                            duration: Duration::from_secs(0),
                        });
                    } else {
                        tracing::debug!("Skipping TCP connection line {}: insufficient data (expected >= 10 fields, got {})", line_num + 1, parts.len());
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to read /proc/net/tcp: {}", e);
                // Continue with empty connections - graceful degradation
            }
        }

        // Limit connections to configured maximum
        if connections.len() > self.config.max_connections {
            tracing::info!("Truncating connections list from {} to {} (max_connections limit)", 
                          connections.len(), self.config.max_connections);
            connections.truncate(self.config.max_connections);
        }

        tracing::debug!("Collected {} network connections", connections.len());

        Ok(connections)
    }

    /// Collect network quality metrics
    fn collect_network_quality_metrics(&self) -> Result<NetworkQualityMetrics> {
        // This would typically require ping or other network testing
        // For now, we'll return default values
        Ok(NetworkQualityMetrics::default())
    }

    /// Benchmark network monitoring performance
    pub fn benchmark_network_monitoring(&mut self, iterations: usize) -> Result<NetworkBenchmarkResults> {
        use std::time::Instant;
        
        let mut results = NetworkBenchmarkResults {
            iterations,
            ..Default::default()
        };
        
        // Clear cache for fair benchmarking
        self.clear_interface_cache();
        
        // Benchmark interface collection
        let interface_start = Instant::now();
        for _ in 0..iterations {
            let _ = self.collect_interface_stats()?;
        }
        results.interface_collection_time = interface_start.elapsed();
        
        // Benchmark protocol collection
        let protocol_start = Instant::now();
        for _ in 0..iterations {
            let _ = self.collect_protocol_stats()?;
        }
        results.protocol_collection_time = protocol_start.elapsed();
        
        // Benchmark connection collection
        let connection_start = Instant::now();
        for _ in 0..iterations {
            let _ = self.collect_connection_stats()?;
        }
        results.connection_collection_time = connection_start.elapsed();
        
        // Benchmark full collection with caching
        let full_start = Instant::now();
        for _ in 0..iterations {
            let _ = self.collect_network_stats()?;
        }
        results.full_collection_time = full_start.elapsed();
        
        // Calculate averages
        if iterations > 0 {
            results.avg_interface_time = results.interface_collection_time / iterations as u32;
            results.avg_protocol_time = results.protocol_collection_time / iterations as u32;
            results.avg_connection_time = results.connection_collection_time / iterations as u32;
            results.avg_full_time = results.full_collection_time / iterations as u32;
        }
        
        Ok(results)
    }

    /// Calculate network traffic deltas between current and previous collection
    pub fn calculate_traffic_deltas(
        &self,
        current: &ComprehensiveNetworkStats,
        previous: &ComprehensiveNetworkStats,
    ) -> NetworkTrafficDeltas {
        let mut deltas = NetworkTrafficDeltas::default();

        // Calculate interface deltas
        for current_iface in &current.interfaces {
            if let Some(prev_iface) = previous.interfaces.iter().find(|i| i.name == current_iface.name) {


                let rx_bytes_delta = current_iface.rx_bytes.saturating_sub(prev_iface.rx_bytes);
                let tx_bytes_delta = current_iface.tx_bytes.saturating_sub(prev_iface.tx_bytes);
                let rx_packets_delta = current_iface.rx_packets.saturating_sub(prev_iface.rx_packets);
                let tx_packets_delta = current_iface.tx_packets.saturating_sub(prev_iface.tx_packets);

                deltas.interface_deltas.push(NetworkInterfaceDelta {
                    name: current_iface.name.clone(),
                    rx_bytes_delta,
                    tx_bytes_delta,
                    rx_packets_delta,
                    tx_packets_delta,
                });
            }
        }

        // Calculate total deltas
        deltas.total_rx_bytes_delta = current.total_rx_bytes.saturating_sub(previous.total_rx_bytes);
        deltas.total_tx_bytes_delta = current.total_tx_bytes.saturating_sub(previous.total_tx_bytes);
        deltas.total_rx_packets_delta = current.total_rx_packets.saturating_sub(previous.total_rx_packets);
        deltas.total_tx_packets_delta = current.total_tx_packets.saturating_sub(previous.total_tx_packets);

        deltas
    }
}

/// Network benchmark results
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NetworkBenchmarkResults {
    /// Number of benchmark iterations
    pub iterations: usize,
    /// Total time spent collecting interface statistics
    pub interface_collection_time: Duration,
    /// Total time spent collecting protocol statistics
    pub protocol_collection_time: Duration,
    /// Total time spent collecting connection statistics
    pub connection_collection_time: Duration,
    /// Total time spent collecting full network statistics
    pub full_collection_time: Duration,
    /// Average time per interface collection
    pub avg_interface_time: Duration,
    /// Average time per protocol collection
    pub avg_protocol_time: Duration,
    /// Average time per connection collection
    pub avg_connection_time: Duration,
    /// Average time per full collection
    pub avg_full_time: Duration,
}

/// Network traffic deltas between collections
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NetworkTrafficDeltas {
    /// Interface-specific deltas
    pub interface_deltas: Vec<NetworkInterfaceDelta>,
    /// Total bytes received delta
    pub total_rx_bytes_delta: u64,
    /// Total bytes transmitted delta
    pub total_tx_bytes_delta: u64,
    /// Total packets received delta
    pub total_rx_packets_delta: u64,
    /// Total packets transmitted delta
    pub total_tx_packets_delta: u64,
}

/// Interface-specific traffic delta
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NetworkInterfaceDelta {
    /// Interface name
    pub name: String,
    /// Bytes received delta
    pub rx_bytes_delta: u64,
    /// Bytes transmitted delta
    pub tx_bytes_delta: u64,
    /// Packets received delta
    pub rx_packets_delta: u64,
    /// Packets transmitted delta
    pub tx_packets_delta: u64,
}

/// Helper function to convert IP address from u32 to IpAddr
pub fn u32_to_ipaddr(ip: u32) -> IpAddr {
    let octets = [
        ((ip >> 24) & 0xFF) as u8,
        ((ip >> 16) & 0xFF) as u8,
        ((ip >> 8) & 0xFF) as u8,
        (ip & 0xFF) as u8,
    ];
    IpAddr::V4(Ipv4Addr::from(octets))
}

/// Helper function to format IP address for display
pub fn format_ip_addr(ip: IpAddr) -> String {
    ip.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn test_network_interface_stats_default() {
        let stats = NetworkInterfaceStats::default();
        assert_eq!(stats.name, String::new());
        assert_eq!(stats.rx_bytes, 0);
        assert_eq!(stats.tx_bytes, 0);
        assert!(matches!(stats.interface_type, NetworkInterfaceType::Unknown));
    }

    #[test]
    fn test_network_protocol_stats_default() {
        let stats = NetworkProtocolStats::default();
        assert_eq!(stats.tcp_connections, 0);
        assert_eq!(stats.udp_connections, 0);
        assert_eq!(stats.icmp_packets, 0);
    }

    #[test]
    fn test_comprehensive_network_stats_default() {
        let stats = ComprehensiveNetworkStats::default();
        assert_eq!(stats.interfaces.len(), 0);
        assert_eq!(stats.total_rx_bytes, 0);
        assert_eq!(stats.total_tx_bytes, 0);
    }

    #[test]
    fn test_network_monitor_creation() {
        let monitor = NetworkMonitor::new();
        assert!(matches!(monitor.config.enable_detailed_interfaces, true));
        assert!(matches!(monitor.config.enable_protocol_monitoring, true));
    }

    #[test]
    fn test_network_monitor_with_config() {
        let config = NetworkMonitorConfig {
            enable_detailed_interfaces: false,
            ..Default::default()
        };
        let monitor = NetworkMonitor::with_config(config);
        assert!(matches!(monitor.config.enable_detailed_interfaces, false));
    }

    #[test]
    fn test_interface_type_detection() {
        let monitor = NetworkMonitor::new();
        assert!(matches!(monitor.detect_interface_type("lo"), NetworkInterfaceType::Loopback));
        assert!(matches!(monitor.detect_interface_type("eth0"), NetworkInterfaceType::Ethernet));
        assert!(matches!(monitor.detect_interface_type("wlan0"), NetworkInterfaceType::Wifi));
        assert!(matches!(monitor.detect_interface_type("virbr0"), NetworkInterfaceType::Virtual));
        assert!(matches!(monitor.detect_interface_type("tun0"), NetworkInterfaceType::Tunnel));
        assert!(matches!(monitor.detect_interface_type("br0"), NetworkInterfaceType::Bridge));
        assert!(matches!(monitor.detect_interface_type("unknown0"), NetworkInterfaceType::Unknown));
    }

    #[test]
    fn test_ip_conversion() {
        let ip = u32_to_ipaddr(0x01020304); // 1.2.3.4
        assert!(matches!(ip, IpAddr::V4(_)));
        if let IpAddr::V4(ipv4) = ip {
            assert_eq!(ipv4, Ipv4Addr::new(1, 2, 3, 4));
        }
    }

    #[test]
    fn test_network_traffic_deltas() {
        let mut current = ComprehensiveNetworkStats::default();
        current.total_rx_bytes = 1000;
        current.total_tx_bytes = 2000;

        let mut previous = ComprehensiveNetworkStats::default();
        previous.total_rx_bytes = 500;
        previous.total_tx_bytes = 1000;

        let monitor = NetworkMonitor::new();
        let deltas = monitor.calculate_traffic_deltas(&current, &previous);

        assert_eq!(deltas.total_rx_bytes_delta, 500);
        assert_eq!(deltas.total_tx_bytes_delta, 1000);
    }

    #[test]
    fn test_network_config_serialization() {
        let config = NetworkMonitorConfig::default();
        let json = serde_json::to_string(&config).expect("Serialization should work");
        let deserialized: NetworkMonitorConfig = serde_json::from_str(&json).expect("Deserialization should work");
        assert_eq!(deserialized.enable_detailed_interfaces, config.enable_detailed_interfaces);
        assert_eq!(deserialized.max_connections, config.max_connections);
    }

    #[test]
    fn test_network_traffic_deltas_with_interfaces() {
        let mut current = ComprehensiveNetworkStats::default();
        current.total_rx_bytes = 1000;
        current.total_tx_bytes = 2000;
        
        // Add interface data
        current.interfaces.push(NetworkInterfaceStats {
            name: "eth0".to_string(),
            rx_bytes: 500,
            tx_bytes: 1000,
            ..Default::default()
        });

        let mut previous = ComprehensiveNetworkStats::default();
        previous.total_rx_bytes = 500;
        previous.total_tx_bytes = 1000;
        
        // Add previous interface data
        previous.interfaces.push(NetworkInterfaceStats {
            name: "eth0".to_string(),
            rx_bytes: 250,
            tx_bytes: 500,
            ..Default::default()
        });

        let monitor = NetworkMonitor::new();
        let deltas = monitor.calculate_traffic_deltas(&current, &previous);

        assert_eq!(deltas.total_rx_bytes_delta, 500);
        assert_eq!(deltas.total_tx_bytes_delta, 1000);
        assert_eq!(deltas.interface_deltas.len(), 1);
        assert_eq!(deltas.interface_deltas[0].name, "eth0");
        assert_eq!(deltas.interface_deltas[0].rx_bytes_delta, 250);
        assert_eq!(deltas.interface_deltas[0].tx_bytes_delta, 500);
    }

    #[test]
    fn test_network_traffic_deltas_no_previous_data() {
        let mut current = ComprehensiveNetworkStats::default();
        current.total_rx_bytes = 1000;
        current.total_tx_bytes = 2000;
        
        let previous = ComprehensiveNetworkStats::default();

        let monitor = NetworkMonitor::new();
        let deltas = monitor.calculate_traffic_deltas(&current, &previous);

        assert_eq!(deltas.total_rx_bytes_delta, 1000);
        assert_eq!(deltas.total_tx_bytes_delta, 2000);
        assert_eq!(deltas.interface_deltas.len(), 0);
    }

    #[test]
    fn test_network_traffic_deltas_multiple_interfaces() {
        let mut current = ComprehensiveNetworkStats::default();
        current.total_rx_bytes = 1500;
        current.total_tx_bytes = 3000;
        
        // Add multiple interfaces
        current.interfaces.push(NetworkInterfaceStats {
            name: "eth0".to_string(),
            rx_bytes: 500,
            tx_bytes: 1000,
            ..Default::default()
        });
        current.interfaces.push(NetworkInterfaceStats {
            name: "wlan0".to_string(),
            rx_bytes: 1000,
            tx_bytes: 2000,
            ..Default::default()
        });

        let mut previous = ComprehensiveNetworkStats::default();
        previous.total_rx_bytes = 750;
        previous.total_tx_bytes = 1500;
        
        // Add previous interface data
        previous.interfaces.push(NetworkInterfaceStats {
            name: "eth0".to_string(),
            rx_bytes: 250,
            tx_bytes: 500,
            ..Default::default()
        });
        previous.interfaces.push(NetworkInterfaceStats {
            name: "wlan0".to_string(),
            rx_bytes: 500,
            tx_bytes: 1000,
            ..Default::default()
        });

        let monitor = NetworkMonitor::new();
        let deltas = monitor.calculate_traffic_deltas(&current, &previous);

        assert_eq!(deltas.total_rx_bytes_delta, 750);
        assert_eq!(deltas.total_tx_bytes_delta, 1500);
        assert_eq!(deltas.interface_deltas.len(), 2);
        
        // Check eth0 deltas
        let eth0_delta = deltas.interface_deltas.iter().find(|d| d.name == "eth0").unwrap();
        assert_eq!(eth0_delta.rx_bytes_delta, 250);
        assert_eq!(eth0_delta.tx_bytes_delta, 500);
        
        // Check wlan0 deltas
        let wlan0_delta = deltas.interface_deltas.iter().find(|d| d.name == "wlan0").unwrap();
        assert_eq!(wlan0_delta.rx_bytes_delta, 500);
        assert_eq!(wlan0_delta.tx_bytes_delta, 1000);
    }

    #[test]
    fn test_network_interface_stats_with_data() {
        let mut stats = NetworkInterfaceStats::default();
        stats.name = "eth0".to_string();
        stats.interface_type = NetworkInterfaceType::Ethernet;
        stats.mac_address = Some("00:11:22:33:44:55".to_string());
        stats.ip_addresses = vec![IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))];
        stats.speed_mbps = Some(1000);
        stats.is_up = true;
        stats.rx_bytes = 1024;
        stats.tx_bytes = 2048;
        stats.rx_packets = 100;
        stats.tx_packets = 200;
        
        assert_eq!(stats.name, "eth0");
        assert!(matches!(stats.interface_type, NetworkInterfaceType::Ethernet));
        assert_eq!(stats.mac_address, Some("00:11:22:33:44:55".to_string()));
        assert_eq!(stats.ip_addresses.len(), 1);
        assert_eq!(stats.speed_mbps, Some(1000));
        assert!(stats.is_up);
        assert_eq!(stats.rx_bytes, 1024);
        assert_eq!(stats.tx_bytes, 2048);
    }

    #[test]
    fn test_network_connection_stats_with_data() {
        let mut stats = NetworkConnectionStats::default();
        stats.src_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        stats.dst_ip = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
        stats.src_port = 12345;
        stats.dst_port = 80;
        stats.protocol = "TCP".to_string();
        stats.state = "ESTABLISHED".to_string();
        stats.pid = Some(1234);
        stats.process_name = Some("test_process".to_string());
        stats.bytes_transmitted = 1024;
        stats.bytes_received = 2048;
        
        assert!(matches!(stats.src_ip, IpAddr::V4(_)));
        assert!(matches!(stats.dst_ip, IpAddr::V4(_)));
        assert_eq!(stats.src_port, 12345);
        assert_eq!(stats.dst_port, 80);
        assert_eq!(stats.protocol, "TCP");
        assert_eq!(stats.state, "ESTABLISHED");
        assert_eq!(stats.pid, Some(1234));
        assert_eq!(stats.process_name, Some("test_process".to_string()));
    }

    #[test]
    fn test_network_quality_metrics_with_data() {
        let mut metrics = NetworkQualityMetrics::default();
        metrics.packet_loss = 0.1;
        metrics.latency_ms = 50.5;
        metrics.jitter_ms = 5.2;
        metrics.bandwidth_utilization = 0.75;
        metrics.stability_score = 0.95;
        
        assert_eq!(metrics.packet_loss, 0.1);
        assert_eq!(metrics.latency_ms, 50.5);
        assert_eq!(metrics.jitter_ms, 5.2);
        assert_eq!(metrics.bandwidth_utilization, 0.75);
        assert_eq!(metrics.stability_score, 0.95);
    }

    #[test]
    fn test_port_usage_stats_with_data() {
        let mut stats = PortUsageStats::default();
        stats.port = 8080;
        stats.protocol = "TCP".to_string();
        stats.connection_count = 5;
        stats.bytes_transmitted = 10240;
        stats.bytes_received = 20480;
        stats.processes = vec![1234, 5678];
        
        assert_eq!(stats.port, 8080);
        assert_eq!(stats.protocol, "TCP");
        assert_eq!(stats.connection_count, 5);
        assert_eq!(stats.bytes_transmitted, 10240);
        assert_eq!(stats.bytes_received, 20480);
        assert_eq!(stats.processes.len(), 2);
    }

    #[test]
    fn test_network_monitor_config_custom() {
        let config = NetworkMonitorConfig {
            enable_detailed_interfaces: false,
            enable_protocol_monitoring: false,
            enable_port_monitoring: true,
            enable_connection_tracking: true,
            enable_quality_monitoring: false,
            max_connections: 2048,
            update_interval_secs: 30,
            monitored_ports: vec![80, 443, 8080],
            monitored_protocols: vec!["TCP".to_string(), "UDP".to_string(), "ICMP".to_string()],
        };
        
        let monitor = NetworkMonitor::with_config(config);
        assert!(!monitor.config.enable_detailed_interfaces);
        assert!(!monitor.config.enable_protocol_monitoring);
        assert!(monitor.config.enable_port_monitoring);
        assert!(monitor.config.enable_connection_tracking);
        assert!(!monitor.config.enable_quality_monitoring);
        assert_eq!(monitor.config.max_connections, 2048);
        assert_eq!(monitor.config.update_interval_secs, 30);
        assert_eq!(monitor.config.monitored_ports.len(), 3);
        assert_eq!(monitor.config.monitored_protocols.len(), 3);
    }

    #[test]
    fn test_format_ip_addr() {
        let ipv4 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ipv6 = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        
        assert_eq!(format_ip_addr(ipv4), "192.168.1.1");
        assert_eq!(format_ip_addr(ipv6), "2001:db8::1");
    }

    #[test]
    fn test_network_interface_delta_default() {
        let delta = NetworkInterfaceDelta::default();
        assert_eq!(delta.name, String::new());
        assert_eq!(delta.rx_bytes_delta, 0);
        assert_eq!(delta.tx_bytes_delta, 0);
        assert_eq!(delta.rx_packets_delta, 0);
        assert_eq!(delta.tx_packets_delta, 0);
    }

    #[test]
    fn test_network_traffic_deltas_default() {
        let deltas = NetworkTrafficDeltas::default();
        assert_eq!(deltas.interface_deltas.len(), 0);
        assert_eq!(deltas.total_rx_bytes_delta, 0);
        assert_eq!(deltas.total_tx_bytes_delta, 0);
        assert_eq!(deltas.total_rx_packets_delta, 0);
        assert_eq!(deltas.total_tx_packets_delta, 0);
    }

    #[test]
    fn test_network_error_handling_empty_data() {
        // Test error handling with empty interface data
        let monitor = NetworkMonitor::new();
        
        // Test with empty interface name
        let interface_type = monitor.detect_interface_type("");
        assert!(matches!(interface_type, NetworkInterfaceType::Unknown));
        
        // Test with invalid interface name
        let interface_type = monitor.detect_interface_type("invalid!@#");
        assert!(matches!(interface_type, NetworkInterfaceType::Unknown));
    }

    #[test]
    fn test_network_error_handling_invalid_parsing() {
        // Test error handling with invalid numeric parsing
        let result: Result<u64> = "invalid".parse::<u64>()
            .with_context(|| "Test parsing error".to_string());
        
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Test parsing error"));
        }
    }

    #[test]
    fn test_network_config_validation() {
        // Test configuration validation
        let config = NetworkMonitorConfig {
            max_connections: 0, // Edge case: zero connections
            ..Default::default()
        };
        
        let monitor = NetworkMonitor::with_config(config);
        assert_eq!(monitor.config.max_connections, 0);
        
        // Test with very large max_connections
        let config = NetworkMonitorConfig {
            max_connections: usize::MAX,
            ..Default::default()
        };
        
        let monitor = NetworkMonitor::with_config(config);
        assert_eq!(monitor.config.max_connections, usize::MAX);
    }

    #[test]
    fn test_network_interface_stats_edge_cases() {
        // Test edge cases for interface statistics
        let mut stats = NetworkInterfaceStats::default();
        
        // Test with maximum values
        stats.rx_bytes = u64::MAX;
        stats.tx_bytes = u64::MAX;
        stats.rx_packets = u64::MAX;
        stats.tx_packets = u64::MAX;
        
        assert_eq!(stats.rx_bytes, u64::MAX);
        assert_eq!(stats.tx_bytes, u64::MAX);
        
        // Test with zero values
        stats.rx_bytes = 0;
        stats.tx_bytes = 0;
        assert_eq!(stats.rx_bytes, 0);
        assert_eq!(stats.tx_bytes, 0);
    }

    #[test]
    fn test_network_connection_stats_edge_cases() {
        // Test edge cases for connection statistics
        let mut stats = NetworkConnectionStats::default();
        
        // Test with maximum port values
        stats.src_port = u16::MAX;
        stats.dst_port = u16::MAX;
        assert_eq!(stats.src_port, u16::MAX);
        assert_eq!(stats.dst_port, u16::MAX);
        
        // Test with zero port values
        stats.src_port = 0;
        stats.dst_port = 0;
        assert_eq!(stats.src_port, 0);
        assert_eq!(stats.dst_port, 0);
        
        // Test with maximum byte values
        stats.bytes_transmitted = u64::MAX;
        stats.bytes_received = u64::MAX;
        assert_eq!(stats.bytes_transmitted, u64::MAX);
        assert_eq!(stats.bytes_received, u64::MAX);
    }

    #[test]
    fn test_network_quality_metrics_edge_cases() {
        // Test edge cases for quality metrics
        let mut metrics = NetworkQualityMetrics::default();
        
        // Test with maximum values
        metrics.packet_loss = 1.0;
        metrics.latency_ms = f64::MAX;
        metrics.jitter_ms = f64::MAX;
        metrics.bandwidth_utilization = 1.0;
        metrics.stability_score = 1.0;
        
        assert_eq!(metrics.packet_loss, 1.0);
        assert_eq!(metrics.latency_ms, f64::MAX);
        
        // Test with zero values
        metrics.packet_loss = 0.0;
        metrics.latency_ms = 0.0;
        metrics.jitter_ms = 0.0;
        metrics.bandwidth_utilization = 0.0;
        metrics.stability_score = 0.0;
        
        assert_eq!(metrics.packet_loss, 0.0);
        assert_eq!(metrics.latency_ms, 0.0);
    }

    #[test]
    fn test_network_port_usage_edge_cases() {
        // Test edge cases for port usage statistics
        let mut stats = PortUsageStats::default();
        
        // Test with maximum port value
        stats.port = u16::MAX;
        assert_eq!(stats.port, u16::MAX);
        
        // Test with zero port value
        stats.port = 0;
        assert_eq!(stats.port, 0);
        
        // Test with maximum connection count
        stats.connection_count = u64::MAX;
        assert_eq!(stats.connection_count, u64::MAX);
        
        // Test with empty processes list
        stats.processes = Vec::new();
        assert!(stats.processes.is_empty());
    }

    #[test]
    fn test_network_traffic_deltas_edge_cases() {
        // Test edge cases for traffic deltas
        let mut current = ComprehensiveNetworkStats::default();
        let mut previous = ComprehensiveNetworkStats::default();
        
        // Test with maximum values
        current.total_rx_bytes = u64::MAX;
        current.total_tx_bytes = u64::MAX;
        previous.total_rx_bytes = u64::MAX;
        previous.total_tx_bytes = u64::MAX;
        
        let monitor = NetworkMonitor::new();
        let deltas = monitor.calculate_traffic_deltas(&current, &previous);
        
        // Should handle overflow gracefully
        assert_eq!(deltas.total_rx_bytes_delta, 0);
        assert_eq!(deltas.total_tx_bytes_delta, 0);
        
        // Test with zero values
        current.total_rx_bytes = 0;
        current.total_tx_bytes = 0;
        previous.total_rx_bytes = 0;
        previous.total_tx_bytes = 0;
        
        let deltas = monitor.calculate_traffic_deltas(&current, &previous);
        assert_eq!(deltas.total_rx_bytes_delta, 0);
        assert_eq!(deltas.total_tx_bytes_delta, 0);
    }

    #[test]
    fn test_network_config_serialization_edge_cases() {
        // Test serialization edge cases
        let config = NetworkMonitorConfig {
            monitored_ports: vec![], // Empty ports
            monitored_protocols: vec![], // Empty protocols
            max_connections: 0, // Zero connections
            ..Default::default()
        };
        
        let json = serde_json::to_string(&config).expect("Serialization should work");
        let deserialized: NetworkMonitorConfig = serde_json::from_str(&json).expect("Deserialization should work");
        
        assert_eq!(deserialized.monitored_ports.len(), 0);
        assert_eq!(deserialized.monitored_protocols.len(), 0);
        assert_eq!(deserialized.max_connections, 0);
    }

    #[test]
    fn test_network_interface_type_detection_edge_cases() {
        // Test edge cases for interface type detection
        let monitor = NetworkMonitor::new();
        
        // Test with various edge case names
        assert!(matches!(monitor.detect_interface_type("lo"), NetworkInterfaceType::Loopback));
        assert!(matches!(monitor.detect_interface_type("loopback"), NetworkInterfaceType::Loopback));
        assert!(matches!(monitor.detect_interface_type("eth"), NetworkInterfaceType::Ethernet));
        assert!(matches!(monitor.detect_interface_type("ethernet"), NetworkInterfaceType::Ethernet));
        assert!(matches!(monitor.detect_interface_type("wlan"), NetworkInterfaceType::Wifi));
        assert!(matches!(monitor.detect_interface_type("wireless"), NetworkInterfaceType::Unknown));
        
        // Test with special characters
        assert!(matches!(monitor.detect_interface_type("eth0:1"), NetworkInterfaceType::Ethernet));
        assert!(matches!(monitor.detect_interface_type("eth0@"), NetworkInterfaceType::Ethernet));
    }

    #[test]
    fn test_network_monitor_config_edge_cases() {
        // Test edge cases for monitor configuration
        let config = NetworkMonitorConfig {
            update_interval_secs: 0, // Zero interval
            max_connections: 1, // Minimum connections
            ..Default::default()
        };
        
        let monitor = NetworkMonitor::with_config(config);
        assert_eq!(monitor.config.update_interval_secs, 0);
        assert_eq!(monitor.config.max_connections, 1);
        
        // Test with maximum values
        let config = NetworkMonitorConfig {
            update_interval_secs: u64::MAX,
            max_connections: usize::MAX,
            ..Default::default()
        };
        
        let monitor = NetworkMonitor::with_config(config);
        assert_eq!(monitor.config.update_interval_secs, u64::MAX);
        assert_eq!(monitor.config.max_connections, usize::MAX);
    }

    #[test]
    fn test_network_stats_serialization_edge_cases() {
        // Test serialization of edge case network stats
        let mut stats = ComprehensiveNetworkStats::default();
        
        // Add interface with maximum values
        stats.interfaces.push(NetworkInterfaceStats {
            name: "eth0".to_string(),
            rx_bytes: u64::MAX,
            tx_bytes: u64::MAX,
            rx_packets: u64::MAX,
            tx_packets: u64::MAX,
            ..Default::default()
        });
        
        // Set maximum totals
        stats.total_rx_bytes = u64::MAX;
        stats.total_tx_bytes = u64::MAX;
        stats.total_rx_packets = u64::MAX;
        stats.total_tx_packets = u64::MAX;
        
        let json = serde_json::to_string(&stats).expect("Serialization should work");
        let deserialized: ComprehensiveNetworkStats = serde_json::from_str(&json).expect("Deserialization should work");
        
        assert_eq!(deserialized.interfaces.len(), 1);
        assert_eq!(deserialized.total_rx_bytes, u64::MAX);
    }

    #[test]
    fn test_network_monitor_cache_operations() {
        // Test cache operations in NetworkMonitor
        let mut monitor = NetworkMonitor::new();
        
        // Test initial cache state
        assert!(monitor.interface_cache.is_empty());
        assert!(!monitor.is_interface_cache_valid());
        
        // Test cache clearing
        monitor.clear_interface_cache();
        assert!(monitor.interface_cache.is_empty());
    }

    #[test]
    fn test_network_monitor_with_custom_cache_ttl() {
        // Test NetworkMonitor with custom cache TTL
        let config = NetworkMonitorConfig::default();
        let monitor = NetworkMonitor::with_config_and_cache(config, 10);
        
        assert_eq!(monitor.cache_ttl, Duration::from_secs(10));
    }

    #[test]
    fn test_network_benchmark_results_default() {
        // Test NetworkBenchmarkResults default values
        let results = NetworkBenchmarkResults::default();
        
        assert_eq!(results.iterations, 0);
        assert_eq!(results.interface_collection_time, Duration::from_secs(0));
        assert_eq!(results.protocol_collection_time, Duration::from_secs(0));
        assert_eq!(results.connection_collection_time, Duration::from_secs(0));
        assert_eq!(results.full_collection_time, Duration::from_secs(0));
    }

    #[test]
    fn test_network_benchmark_results_serialization() {
        // Test NetworkBenchmarkResults serialization
        let mut results = NetworkBenchmarkResults::default();
        results.iterations = 10;
        results.interface_collection_time = Duration::from_millis(100);
        results.avg_interface_time = Duration::from_millis(10);
        
        let json = serde_json::to_string(&results).expect("Serialization should work");
        let deserialized: NetworkBenchmarkResults = serde_json::from_str(&json).expect("Deserialization should work");
        
        assert_eq!(deserialized.iterations, 10);
        assert_eq!(deserialized.interface_collection_time, Duration::from_millis(100));
    }

    #[test]
    fn test_network_optimized_methods() {
        // Test optimized methods
        let monitor = NetworkMonitor::new();
        
        // Test fast parsing
        let result = monitor.parse_u64_fast("12345", "eth0", "test_field", 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 12345);
        
        // Test invalid parsing
        let result = monitor.parse_u64_fast("invalid", "eth0", "test_field", 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_network_interface_cache_optimization() {
        // Test interface cache optimization
        let mut monitor = NetworkMonitor::with_config_and_cache(NetworkMonitorConfig::default(), 1);
        
        // Cache should be invalid initially
        assert!(!monitor.is_interface_cache_valid());
        
        // Add some data to cache
        let mut stats = NetworkInterfaceStats::default();
        stats.name = "eth0".to_string();
        monitor.interface_cache.insert("eth0".to_string(), stats);
        monitor.last_cache_update = SystemTime::now();
        
        // Cache should be valid now
        assert!(monitor.is_interface_cache_valid());
        
        // Clear cache
        monitor.clear_interface_cache();
        assert!(!monitor.is_interface_cache_valid());
    }

    #[test]
    fn test_network_memory_optimization() {
        // Test memory optimization in data structures
        let mut stats = ComprehensiveNetworkStats::default();
        
        // Test capacity reservation
        stats.interfaces.reserve(8);
        assert!(stats.interfaces.capacity() >= 8);
        
        // Test with actual data
        for i in 0..5 {
            let mut iface = NetworkInterfaceStats::default();
            iface.name = format!("eth{}", i);
            stats.interfaces.push(iface);
        }
        
        assert_eq!(stats.interfaces.len(), 5);
    }

    #[test]
    fn test_network_optimized_interface_methods() {
        // Test optimized interface methods
        let monitor = NetworkMonitor::new();
        
        // Test interface up check
        let _is_up = monitor.check_interface_up_optimized("lo");
        // Should return false for non-existent interface or handle gracefully
        
        // Test MAC address retrieval
        let _mac = monitor.get_mac_address_optimized("lo");
        // Should return None for non-existent interface or handle gracefully
        
        // Test IP addresses retrieval
        let _ips = monitor.get_ip_addresses_optimized("lo");
        // Should return empty vec for non-existent interface or handle gracefully
        
        // Test interface speed retrieval
        let _speed = monitor.get_interface_speed_optimized("lo");
        // Should return None for non-existent interface or handle gracefully
    }

    #[test]
    fn test_ip_conversion_functions() {
        // Test u32_to_ipaddr function
        let ip1 = u32_to_ipaddr(0x01020304); // 1.2.3.4
        assert_eq!(ip1, IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)));

        let ip2 = u32_to_ipaddr(0x7F000001); // 127.0.0.1
        assert_eq!(ip2, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));

        // Test format_ip_addr function
        assert_eq!(format_ip_addr(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))), "1.2.3.4");
        assert_eq!(format_ip_addr(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1))), "::1");
    }



    #[test]
    fn test_network_stats_equality() {
        // Test equality for network stats structures
        let mut stats1 = NetworkInterfaceStats::default();
        stats1.name = "eth0".to_string();
        stats1.rx_bytes = 1000;

        let mut stats2 = NetworkInterfaceStats::default();
        stats2.name = "eth0".to_string();
        stats2.rx_bytes = 1000;

        assert_eq!(stats1, stats2);

        let mut stats3 = NetworkInterfaceStats::default();
        stats3.name = "eth1".to_string();
        stats3.rx_bytes = 1000;

        assert_ne!(stats1, stats3);
    }

    #[test]
    fn test_network_error_handling() {
        // Test that network functions handle errors gracefully
        // This is a basic test - more comprehensive error handling tests
        // would require mocking system calls
        
        // Test that we can create a monitor even if some interfaces don't exist
        let mut monitor = NetworkMonitor::new();
        
        // Test that cache operations don't panic
        monitor.clear_interface_cache();
        
        // Test that we can work with empty data
        let empty_stats = ComprehensiveNetworkStats::default();
        assert_eq!(empty_stats.interfaces.len(), 0);
        assert_eq!(empty_stats.total_rx_bytes, 0);
    }
}
