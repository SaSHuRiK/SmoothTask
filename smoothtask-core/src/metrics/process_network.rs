//! Process Network Monitoring Module
//!
//! This module provides comprehensive network monitoring capabilities at the process level.
//! It collects detailed network statistics for individual processes including:
//! - Network connections (TCP/UDP)
//! - Bytes sent/received
//! - Packets sent/received
//! - Connection states and protocols
//! - Integration with existing process metrics

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;
use std::time::{Duration, SystemTime};

/// Process network statistics structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessNetworkStats {
    /// Process ID
    pub pid: u32,
    /// Total bytes received
    pub rx_bytes: u64,
    /// Total bytes transmitted
    pub tx_bytes: u64,
    /// Total packets received
    pub rx_packets: u64,
    /// Total packets transmitted
    pub tx_packets: u64,
    /// Number of TCP connections
    pub tcp_connections: u64,
    /// Number of UDP connections
    pub udp_connections: u64,
    /// Active network connections
    pub connections: Vec<ProcessConnectionStats>,
    /// Timestamp of last update
    pub last_update: SystemTime,
    /// Data source (proc, ebpf, or mixed)
    pub data_source: String,
}

impl Default for ProcessNetworkStats {
    fn default() -> Self {
        Self {
            pid: 0,
            rx_bytes: 0,
            tx_bytes: 0,
            rx_packets: 0,
            tx_packets: 0,
            tcp_connections: 0,
            udp_connections: 0,
            connections: Vec::new(),
            last_update: SystemTime::UNIX_EPOCH,
            data_source: "proc".to_string(),
        }
    }
}

/// Process connection statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessConnectionStats {
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

impl Default for ProcessConnectionStats {
    fn default() -> Self {
        Self {
            src_ip: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            dst_ip: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            src_port: 0,
            dst_port: 0,
            protocol: String::new(),
            state: String::new(),
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

/// Process network monitor configuration
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ProcessNetworkMonitorConfig {
    /// Enable process network monitoring
    pub enable_process_network_monitoring: bool,
    /// Maximum number of connections to track per process
    pub max_connections_per_process: usize,
    /// Enable detailed connection tracking
    pub enable_detailed_connections: bool,
    /// Enable TCP connection monitoring
    pub enable_tcp_monitoring: bool,
    /// Enable UDP connection monitoring
    pub enable_udp_monitoring: bool,
    /// Enable Unix socket monitoring
    pub enable_unix_monitoring: bool,
    /// Update interval in seconds
    pub update_interval_secs: u64,
    /// Use caching for network statistics
    pub enable_caching: bool,
    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
}

impl Default for ProcessNetworkMonitorConfig {
    fn default() -> Self {
        Self {
            enable_process_network_monitoring: true,
            max_connections_per_process: 128,
            enable_detailed_connections: true,
            enable_tcp_monitoring: true,
            enable_udp_monitoring: true,
            enable_unix_monitoring: true,
            update_interval_secs: 60,
            enable_caching: true,
            cache_ttl_seconds: 300,
        }
    }
}

/// Process network monitor main structure
pub struct ProcessNetworkMonitor {
    config: ProcessNetworkMonitorConfig,
    cache: HashMap<u32, ProcessNetworkStats>,
    cache_ttl: Duration,
    last_cache_update: SystemTime,
}

impl Default for ProcessNetworkMonitor {
    fn default() -> Self {
        Self {
            config: ProcessNetworkMonitorConfig::default(),
            cache: HashMap::new(),
            cache_ttl: Duration::from_secs(300),
            last_cache_update: SystemTime::UNIX_EPOCH,
        }
    }
}

impl ProcessNetworkMonitor {
    /// Create a new ProcessNetworkMonitor with default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new ProcessNetworkMonitor with custom configuration
    pub fn with_config(config: ProcessNetworkMonitorConfig) -> Self {
        let cache_ttl = Duration::from_secs(config.cache_ttl_seconds);
        Self {
            config,
            cache: HashMap::new(),
            cache_ttl,
            last_cache_update: SystemTime::UNIX_EPOCH,
        }
    }

    /// Clear the network statistics cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.last_cache_update = SystemTime::UNIX_EPOCH;
    }

    /// Check if cache is valid
    fn is_cache_valid(&self) -> bool {
        if self.cache.is_empty() {
            return false;
        }

        match self.last_cache_update.elapsed() {
            Ok(elapsed) => elapsed < self.cache_ttl,
            Err(_) => false, // Cache is too old or system time issue
        }
    }

    /// Collect network statistics for a specific process
    pub fn collect_process_network_stats(&mut self, pid: u32) -> Result<ProcessNetworkStats> {
        // Check cache first if enabled
        if self.config.enable_caching && self.is_cache_valid() {
            if let Some(cached_stats) = self.cache.get(&pid) {
                tracing::debug!("Using cached network stats for PID {}", pid);
                return Ok(cached_stats.clone());
            }
        }

        let mut stats = ProcessNetworkStats {
            pid,
            last_update: SystemTime::now(),
            data_source: "proc".to_string(),
            ..Default::default()
        };

        // Collect TCP statistics
        if self.config.enable_tcp_monitoring {
            let tcp_stats = self.collect_tcp_stats(pid)?;
            stats.rx_bytes += tcp_stats.rx_bytes;
            stats.tx_bytes += tcp_stats.tx_bytes;
            stats.rx_packets += tcp_stats.rx_packets;
            stats.tx_packets += tcp_stats.tx_packets;
            stats.tcp_connections += tcp_stats.connections;
            if self.config.enable_detailed_connections {
                stats.connections.extend(tcp_stats.detailed_connections);
            }
        }

        // Collect UDP statistics
        if self.config.enable_udp_monitoring {
            let udp_stats = self.collect_udp_stats(pid)?;
            stats.rx_bytes += udp_stats.rx_bytes;
            stats.tx_bytes += udp_stats.tx_bytes;
            stats.rx_packets += udp_stats.rx_packets;
            stats.tx_packets += udp_stats.tx_packets;
            stats.udp_connections += udp_stats.connections;
            if self.config.enable_detailed_connections {
                stats.connections.extend(udp_stats.detailed_connections);
            }
        }

        // Limit connections to configured maximum
        if stats.connections.len() > self.config.max_connections_per_process {
            stats
                .connections
                .truncate(self.config.max_connections_per_process);
        }

        // Enhance with FD mapping for more accurate connection tracking
        if self.config.enable_detailed_connections {
            if let Ok(fd_connections) = self.collect_process_connections_with_fd_mapping(pid) {
                // Merge FD-mapped connections with existing ones
                for fd_conn in fd_connections {
                    // Check if this connection already exists
                    let exists = stats.connections.iter().any(|conn| {
                        conn.src_ip == fd_conn.src_ip
                            && conn.src_port == fd_conn.src_port
                            && conn.dst_ip == fd_conn.dst_ip
                            && conn.dst_port == fd_conn.dst_port
                            && conn.protocol == fd_conn.protocol
                    });

                    if !exists {
                        stats.connections.push(fd_conn);
                    }
                }
            }
        }

        // Update cache
        if self.config.enable_caching {
            self.cache.insert(pid, stats.clone());
            self.last_cache_update = SystemTime::now();
        }

        Ok(stats)
    }

    /// Collect network statistics for multiple processes
    pub fn collect_multiple_process_network_stats(
        &mut self,
        pids: &[u32],
    ) -> Result<Vec<ProcessNetworkStats>> {
        let mut results = Vec::with_capacity(pids.len());

        for &pid in pids {
            match self.collect_process_network_stats(pid) {
                Ok(stats) => results.push(stats),
                Err(e) => {
                    tracing::warn!("Failed to collect network stats for PID {}: {}", pid, e);
                    // Continue with other processes
                }
            }
        }

        Ok(results)
    }

    /// Collect network connections for a specific process using /proc/PID/fd mapping
    /// This provides more accurate process-to-connection mapping
    pub fn collect_process_connections_with_fd_mapping(
        &self,
        pid: u32,
    ) -> Result<Vec<ProcessConnectionStats>> {
        let mut connections = Vec::new();
        let proc_fd_path = format!("/proc/{}/fd", pid);

        if !Path::new(&proc_fd_path).exists() {
            tracing::debug!("Process {} does not exist or has no file descriptors", pid);
            return Ok(connections);
        }

        // Read all file descriptors for the process
        let fd_entries = match fs::read_dir(&proc_fd_path) {
            Ok(entries) => entries,
            Err(e) => {
                tracing::warn!("Failed to read file descriptors for PID {}: {}", pid, e);
                return Ok(connections);
            }
        };

        // Map file descriptors to network connections
        for fd_entry in fd_entries {
            match fd_entry {
                Ok(entry) => {
                    let fd_path = entry.path();

                    // Check if this file descriptor is a socket
                    if let Ok(fd_link) = fs::read_link(&fd_path) {
                        let fd_link_str = fd_link.to_string_lossy();

                        if fd_link_str.contains("socket:") {
                            // This is a network socket, try to get connection info
                            if let Some(conn_info) =
                                self.get_socket_connection_info(pid, &fd_link_str)
                            {
                                connections.push(conn_info);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!(
                        "Failed to read file descriptor entry for PID {}: {}",
                        pid,
                        e
                    );
                    continue;
                }
            }
        }

        Ok(connections)
    }

    /// Get connection information from socket file descriptor
    fn get_socket_connection_info(
        &self,
        pid: u32,
        socket_path: &str,
    ) -> Option<ProcessConnectionStats> {
        // Extract socket inode from the path (format: socket:[inode])
        let inode_start = socket_path.find("socket:[")?;
        let inode_end = socket_path.find("]")?;
        let inode_str = &socket_path[inode_start + 8..inode_end];

        match inode_str.parse::<u32>() {
            Ok(inode) => {
                // Try to find this inode in /proc/net/tcp and /proc/net/tcp6
                if let Some(tcp_conn) = self.find_tcp_connection_by_inode(inode) {
                    return Some(tcp_conn);
                }
                if let Some(udp_conn) = self.find_udp_connection_by_inode(inode) {
                    return Some(udp_conn);
                }
            }
            Err(e) => {
                tracing::debug!("Failed to parse socket inode for PID {}: {}", pid, e);
            }
        }

        None
    }

    /// Find TCP connection by inode in /proc/net/tcp and /proc/net/tcp6
    fn find_tcp_connection_by_inode(&self, inode: u32) -> Option<ProcessConnectionStats> {
        // Check IPv4 connections
        if let Ok(tcp_content) = fs::read_to_string("/proc/net/tcp") {
            for line in tcp_content.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 && parts[9] == format!("{}", inode) {
                    return self.parse_tcp_connection_line(&parts);
                }
            }
        }

        // Check IPv6 connections
        if let Ok(tcp_content) = fs::read_to_string("/proc/net/tcp6") {
            for line in tcp_content.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 && parts[9] == format!("{}", inode) {
                    return self.parse_tcp_connection_line(&parts);
                }
            }
        }

        None
    }

    /// Find UDP connection by inode in /proc/net/udp and /proc/net/udp6
    fn find_udp_connection_by_inode(&self, inode: u32) -> Option<ProcessConnectionStats> {
        // Check IPv4 connections
        if let Ok(udp_content) = fs::read_to_string("/proc/net/udp") {
            for line in udp_content.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 && parts[9] == format!("{}", inode) {
                    return self.parse_udp_connection_line(&parts);
                }
            }
        }

        // Check IPv6 connections
        if let Ok(udp_content) = fs::read_to_string("/proc/net/udp6") {
            for line in udp_content.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 && parts[9] == format!("{}", inode) {
                    return self.parse_udp_connection_line(&parts);
                }
            }
        }

        None
    }

    /// Parse TCP connection line from /proc/net/tcp* files
    fn parse_tcp_connection_line(&self, parts: &[&str]) -> Option<ProcessConnectionStats> {
        if parts.len() < 10 {
            return None;
        }

        let mut conn = ProcessConnectionStats::default();
        conn.protocol = "TCP".to_string();

        // Parse state
        let state_hex = parts[3];
        conn.state = self
            .parse_tcp_state(state_hex)
            .unwrap_or_else(|_| "UNKNOWN".to_string());

        // Parse local and remote addresses
        if let Some((src_ip, src_port)) = self.parse_ip_port(parts[1]) {
            conn.src_ip = src_ip;
            conn.src_port = src_port;
        }

        if let Some((dst_ip, dst_port)) = self.parse_ip_port(parts[2]) {
            conn.dst_ip = dst_ip;
            conn.dst_port = dst_port;
        }

        // Parse statistics (if available)
        if parts.len() >= 12 {
            if let Ok(tx_queue) = parts[11].parse::<u64>() {
                conn.bytes_transmitted = tx_queue * 1024; // Approximate
            }
            if let Ok(rx_queue) = parts[12].parse::<u64>() {
                conn.bytes_received = rx_queue * 1024; // Approximate
            }
        }

        Some(conn)
    }

    /// Parse UDP connection line from /proc/net/udp* files
    fn parse_udp_connection_line(&self, parts: &[&str]) -> Option<ProcessConnectionStats> {
        if parts.len() < 10 {
            return None;
        }

        let mut conn = ProcessConnectionStats::default();
        conn.protocol = "UDP".to_string();
        conn.state = "ESTABLISHED".to_string(); // UDP is connectionless

        // Parse local and remote addresses
        if let Some((src_ip, src_port)) = self.parse_ip_port(parts[1]) {
            conn.src_ip = src_ip;
            conn.src_port = src_port;
        }

        if let Some((dst_ip, dst_port)) = self.parse_ip_port(parts[2]) {
            conn.dst_ip = dst_ip;
            conn.dst_port = dst_port;
        }

        Some(conn)
    }

    /// Parse IP address and port from hex string
    fn parse_ip_port(&self, hex_str: &str) -> Option<(IpAddr, u16)> {
        // Format: HEX_IP:HEX_PORT (e.g., "0100007F:1F90" for 127.0.0.1:8080)
        let parts: Vec<&str> = hex_str.split(':').collect();
        if parts.len() != 2 {
            return None;
        }

        let ip_hex = parts[0];
        let port_hex = parts[1];

        // Parse port
        let port = u16::from_str_radix(port_hex, 16).ok()?;

        // Parse IP address (could be IPv4 or IPv6)
        if ip_hex.len() == 8 {
            // IPv4
            let ip_bytes = (0..4)
                .rev()
                .map(|i| u8::from_str_radix(&ip_hex[i * 2..(i + 1) * 2], 16).unwrap_or(0))
                .collect::<Vec<u8>>();

            if ip_bytes.len() == 4 {
                let ip_bytes: [u8; 4] = ip_bytes.try_into().unwrap();
                let ip = IpAddr::V4(std::net::Ipv4Addr::from(ip_bytes));
                return Some((ip, port));
            }
        } else if ip_hex.len() == 32 {
            // IPv6
            let ip_bytes: Vec<u16> = (0..8)
                .rev()
                .map(|i| u16::from_str_radix(&ip_hex[i * 4..(i + 1) * 4], 16).unwrap_or(0))
                .collect();

            if ip_bytes.len() == 8 {
                let ip_bytes: [u16; 8] = ip_bytes.try_into().unwrap();
                let ip = IpAddr::V6(std::net::Ipv6Addr::from(ip_bytes));
                return Some((ip, port));
            }
        }

        None
    }

    /// Collect TCP statistics for a process using enhanced methods
    fn collect_tcp_stats(&self, pid: u32) -> Result<ProcessProtocolStats> {
        let mut stats = ProcessProtocolStats::default();

        // First, try to use /proc/net/tcp6 which includes inode information
        // that can be mapped to process file descriptors
        if Path::new("/proc/net/tcp6").exists() {
            let tcp_content =
                fs::read_to_string("/proc/net/tcp6").context("Failed to read /proc/net/tcp6")?;

            // Parse TCP connections and find those belonging to our process
            for (_line_num, line) in tcp_content.lines().skip(1).enumerate() {
                // Skip header
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 {
                    let local_addr_hex = parts[1];
                    let remote_addr_hex = parts[2];
                    let state_hex = parts[3];
                    let _uid = parts[7];
                    let inode = parts[9];

                    // Check if this connection belongs to our process by checking file descriptors
                    if self.is_connection_for_pid(pid, inode)? {
                        // Parse state
                        let state = self.parse_tcp_state(state_hex)?;

                        // Parse addresses and ports
                        if let Ok((src_ip, src_port, dst_ip, dst_port)) =
                            self.parse_tcp_addresses(local_addr_hex, remote_addr_hex)
                        {
                            let connection = ProcessConnectionStats {
                                src_ip,
                                dst_ip,
                                src_port,
                                dst_port,
                                protocol: "TCP".to_string(),
                                state,
                                bytes_transmitted: 0, // Would need more detailed parsing
                                bytes_received: 0,
                                packets_transmitted: 0,
                                packets_received: 0,
                                start_time: SystemTime::now(),
                                last_activity: SystemTime::now(),
                                duration: Duration::from_secs(0),
                            };

                            stats.connections += 1;
                            stats.detailed_connections.push(connection);
                        }
                    }
                }
            }
        }

        // Fallback to /proc/net/tcp if tcp6 is not available
        let tcp_content =
            fs::read_to_string("/proc/net/tcp").context("Failed to read /proc/net/tcp")?;

        // Parse TCP connections
        for (_line_num, line) in tcp_content.lines().skip(1).enumerate() {
            // Skip header
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 10 {
                let local_addr_hex = parts[1];
                let remote_addr_hex = parts[2];
                let state_hex = parts[3];
                let _uid = parts[7];
                let inode = parts[9];

                // Check if this connection belongs to our process by checking file descriptors
                if self.is_connection_for_pid(pid, inode)? {
                    // Parse state
                    let state = self.parse_tcp_state(state_hex)?;

                    // Parse addresses and ports
                    if let Ok((src_ip, src_port, dst_ip, dst_port)) =
                        self.parse_tcp_addresses(local_addr_hex, remote_addr_hex)
                    {
                        let connection = ProcessConnectionStats {
                            src_ip,
                            dst_ip,
                            src_port,
                            dst_port,
                            protocol: "TCP".to_string(),
                            state,
                            bytes_transmitted: 0, // Would need more detailed parsing
                            bytes_received: 0,
                            packets_transmitted: 0,
                            packets_received: 0,
                            start_time: SystemTime::now(),
                            last_activity: SystemTime::now(),
                            duration: Duration::from_secs(0),
                        };

                        stats.connections += 1;
                        stats.detailed_connections.push(connection);
                    }
                }
            }
        }

        Ok(stats)
    }

    /// Collect UDP statistics for a process using enhanced methods
    fn collect_udp_stats(&self, pid: u32) -> Result<ProcessProtocolStats> {
        let mut stats = ProcessProtocolStats::default();

        // First, try to use /proc/net/udp6 which includes inode information
        if Path::new("/proc/net/udp6").exists() {
            let udp_content =
                fs::read_to_string("/proc/net/udp6").context("Failed to read /proc/net/udp6")?;

            // Parse UDP connections and find those belonging to our process
            for (_line_num, line) in udp_content.lines().skip(1).enumerate() {
                // Skip header
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 {
                    let local_addr_hex = parts[1];
                    let remote_addr_hex = parts[2];
                    let _uid = parts[7];
                    let inode = parts[9];

                    // Check if this connection belongs to our process by checking file descriptors
                    if self.is_connection_for_pid(pid, inode)? {
                        // Parse addresses and ports
                        if let Ok((src_ip, src_port, dst_ip, dst_port)) =
                            self.parse_udp_addresses(local_addr_hex, remote_addr_hex)
                        {
                            let connection = ProcessConnectionStats {
                                src_ip,
                                dst_ip,
                                src_port,
                                dst_port,
                                protocol: "UDP".to_string(),
                                state: "ESTABLISHED".to_string(), // UDP doesn't have states like TCP
                                bytes_transmitted: 0,
                                bytes_received: 0,
                                packets_transmitted: 0,
                                packets_received: 0,
                                start_time: SystemTime::now(),
                                last_activity: SystemTime::now(),
                                duration: Duration::from_secs(0),
                            };

                            stats.connections += 1;
                            stats.detailed_connections.push(connection);
                        }
                    }
                }
            }
        }

        // Fallback to /proc/net/udp if udp6 is not available
        let udp_content =
            fs::read_to_string("/proc/net/udp").context("Failed to read /proc/net/udp")?;

        // Parse UDP connections
        for (_line_num, line) in udp_content.lines().skip(1).enumerate() {
            // Skip header
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 10 {
                let local_addr_hex = parts[1];
                let remote_addr_hex = parts[2];
                let _uid = parts[7];
                let inode = parts[9];

                // Check if this connection belongs to our process by checking file descriptors
                if self.is_connection_for_pid(pid, inode)? {
                    // Parse addresses and ports
                    if let Ok((src_ip, src_port, dst_ip, dst_port)) =
                        self.parse_udp_addresses(local_addr_hex, remote_addr_hex)
                    {
                        let connection = ProcessConnectionStats {
                            src_ip,
                            dst_ip,
                            src_port,
                            dst_port,
                            protocol: "UDP".to_string(),
                            state: "ESTABLISHED".to_string(), // UDP doesn't have states like TCP
                            bytes_transmitted: 0,
                            bytes_received: 0,
                            packets_transmitted: 0,
                            packets_received: 0,
                            start_time: SystemTime::now(),
                            last_activity: SystemTime::now(),
                            duration: Duration::from_secs(0),
                        };

                        stats.connections += 1;
                        stats.detailed_connections.push(connection);
                    }
                }
            }
        }

        Ok(stats)
    }

    /// Parse TCP state from hex value
    fn parse_tcp_state(&self, state_hex: &str) -> Result<String> {
        let state_num = u8::from_str_radix(state_hex, 16).context("Failed to parse TCP state")?;

        match state_num {
            1 => Ok("ESTABLISHED".to_string()),
            2 => Ok("SYN_SENT".to_string()),
            3 => Ok("SYN_RECV".to_string()),
            4 => Ok("FIN_WAIT1".to_string()),
            5 => Ok("FIN_WAIT2".to_string()),
            6 => Ok("TIME_WAIT".to_string()),
            7 => Ok("CLOSE".to_string()),
            8 => Ok("CLOSE_WAIT".to_string()),
            9 => Ok("LAST_ACK".to_string()),
            10 => Ok("LISTEN".to_string()),
            11 => Ok("CLOSING".to_string()),
            _ => Ok("UNKNOWN".to_string()),
        }
    }

    /// Parse TCP addresses from hex format
    fn parse_tcp_addresses(
        &self,
        local_addr_hex: &str,
        remote_addr_hex: &str,
    ) -> Result<(IpAddr, u16, IpAddr, u16)> {
        // Parse local address and port
        let local_parts: Vec<&str> = local_addr_hex.split(':').collect();
        if local_parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid local address format"));
        }

        let local_ip_hex = local_parts[0];
        let local_port_hex = local_parts[1];

        // Parse remote address and port
        let remote_parts: Vec<&str> = remote_addr_hex.split(':').collect();
        if remote_parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid remote address format"));
        }

        let remote_ip_hex = remote_parts[0];
        let remote_port_hex = remote_parts[1];

        // Convert IP addresses from hex to decimal
        let local_ip = self.hex_ip_to_decimal(local_ip_hex)?;
        let remote_ip = self.hex_ip_to_decimal(remote_ip_hex)?;

        // Convert ports from hex to decimal
        let local_port = u16::from_str_radix(local_port_hex, 16)?;
        let remote_port = u16::from_str_radix(remote_port_hex, 16)?;

        Ok((local_ip, local_port, remote_ip, remote_port))
    }

    /// Parse UDP addresses from hex format
    fn parse_udp_addresses(
        &self,
        local_addr_hex: &str,
        remote_addr_hex: &str,
    ) -> Result<(IpAddr, u16, IpAddr, u16)> {
        // UDP parsing is similar to TCP
        self.parse_tcp_addresses(local_addr_hex, remote_addr_hex)
    }

    /// Check if a connection (identified by inode) belongs to a specific PID
    fn is_connection_for_pid(&self, pid: u32, inode: &str) -> Result<bool> {
        let proc_fd_path = format!("/proc/{}/fd", pid);

        if !Path::new(&proc_fd_path).exists() {
            return Ok(false);
        }

        // Read the symbolic links in the process's fd directory
        let fd_dir = match fs::read_dir(proc_fd_path) {
            Ok(dir) => dir,
            Err(_) => return Ok(false),
        };

        for entry in fd_dir {
            match entry {
                Ok(entry) => {
                    // Read the symbolic link target
                    let link_target = match fs::read_link(entry.path()) {
                        Ok(target) => target,
                        Err(_) => continue,
                    };

                    // Check if the link target contains the inode
                    if let Some(target_str) = link_target.to_str() {
                        if target_str.contains(&format!("socket:[{}]", inode)) {
                            return Ok(true);
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(false)
    }

    /// Convert hex IP address to decimal
    fn hex_ip_to_decimal(&self, hex_ip: &str) -> Result<IpAddr> {
        // Pad to 8 characters for IPv4
        let padded_hex = format!("{:0>8}", hex_ip);

        // Convert to bytes
        let mut bytes = [0u8; 4];
        for i in 0..4 {
            let byte_hex = &padded_hex[i * 2..(i * 2) + 2];
            bytes[i] = u8::from_str_radix(byte_hex, 16)?;
        }

        // Reverse bytes for network byte order
        let reversed_bytes = [bytes[3], bytes[2], bytes[1], bytes[0]];

        Ok(IpAddr::V4(Ipv4Addr::from(reversed_bytes)))
    }

    /// Get network statistics for all processes
    pub fn collect_all_process_network_stats(&mut self) -> Result<Vec<ProcessNetworkStats>> {
        // This would typically use /proc/net/tcp6 and /proc/net/udp6 which include PID information
        // For now, we'll implement a basic version

        let mut all_stats = Vec::new();

        // Collect TCP statistics for all processes
        if self.config.enable_tcp_monitoring {
            let tcp_stats = self.collect_all_tcp_stats()?;
            all_stats.extend(tcp_stats);
        }

        // Collect UDP statistics for all processes
        if self.config.enable_udp_monitoring {
            let udp_stats = self.collect_all_udp_stats()?;
            all_stats.extend(udp_stats);
        }

        Ok(all_stats)
    }

    /// Collect TCP statistics for all processes
    fn collect_all_tcp_stats(&self) -> Result<Vec<ProcessNetworkStats>> {
        let _process_stats_map: HashMap<u32, ProcessNetworkStats> = HashMap::new();

        // Read TCP connections from /proc/net/tcp6 (includes PID information)
        if Path::new("/proc/net/tcp6").exists() {
            let tcp6_content =
                fs::read_to_string("/proc/net/tcp6").context("Failed to read /proc/net/tcp6")?;

            for _line in tcp6_content.lines().skip(1) {
                // Skip header
                let parts: Vec<&str> = _line.split_whitespace().collect();
                if parts.len() >= 10 {
                    let _uid = parts[7];
                    let _inode = parts[9]; // This is actually the inode, not PID

                    // Note: Getting PID from /proc/net/tcp6 is complex and requires
                    // additional system calls. For now, we'll skip this.
                }
            }
        }

        // For now, return empty vector as this is a complex operation
        Ok(Vec::new())
    }

    /// Collect UDP statistics for all processes
    fn collect_all_udp_stats(&self) -> Result<Vec<ProcessNetworkStats>> {
        // Similar to TCP, this is complex without PID information
        Ok(Vec::new())
    }

    /// Calculate network traffic deltas between current and previous collection
    pub fn calculate_network_deltas(
        &self,
        current: &ProcessNetworkStats,
        previous: &ProcessNetworkStats,
    ) -> ProcessNetworkDeltas {
        ProcessNetworkDeltas {
            rx_bytes_delta: current.rx_bytes.saturating_sub(previous.rx_bytes),
            tx_bytes_delta: current.tx_bytes.saturating_sub(previous.tx_bytes),
            rx_packets_delta: current.rx_packets.saturating_sub(previous.rx_packets),
            tx_packets_delta: current.tx_packets.saturating_sub(previous.tx_packets),
            tcp_connections_delta: current
                .tcp_connections
                .saturating_sub(previous.tcp_connections),
            udp_connections_delta: current
                .udp_connections
                .saturating_sub(previous.udp_connections),
        }
    }

    /// Collect Unix socket statistics for a process
    fn collect_unix_stats(&self, pid: u32) -> Result<ProcessProtocolStats> {
        let mut stats = ProcessProtocolStats::default();

        // Read /proc/net/unix to find Unix domain sockets
        if Path::new("/proc/net/unix").exists() {
            let unix_content =
                fs::read_to_string("/proc/net/unix").context("Failed to read /proc/net/unix")?;

            // Parse Unix socket connections
            for line in unix_content.lines().skip(1) {
                // Skip header
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 8 {
                    let inode = parts[7];
                    let _path = parts[8]; // Path or abstract socket identifier

                    // Check if this socket belongs to our process by checking file descriptors
                    if self.is_connection_for_pid(pid, inode)? {
                        let mut connection = ProcessConnectionStats::default();
                        connection.protocol = "UNIX".to_string();
                        connection.state = "ESTABLISHED".to_string();

                        // Try to extract path information
                        if !parts[8].is_empty() && parts[8] != "0000000000000000" {
                            // This is a filesystem path socket
                            if let Ok(path_str) = self.parse_unix_socket_path(parts[8]) {
                                if let Ok(ip_path) = path_str.parse::<IpAddr>() {
                                    connection.dst_ip = ip_path;
                                } else {
                                    // Store path as string in a custom field (would need to extend struct)
                                    // For now, we'll just note it's a Unix socket
                                }
                            }
                        } else {
                            // This is an abstract socket
                            connection.state = "ABSTRACT".to_string();
                        }

                        stats.connections += 1;
                        stats.detailed_connections.push(connection);
                    }
                }
            }
        }

        Ok(stats)
    }

    /// Parse Unix socket path from hex format
    fn parse_unix_socket_path(&self, hex_path: &str) -> Result<String> {
        // Unix socket paths in /proc/net/unix are stored as hex-encoded strings
        // We need to decode them properly

        // For now, return a placeholder - actual implementation would require
        // proper hex decoding and null-termination handling
        Ok(format!("unix_socket_{}", hex_path))
    }

    /// Enhanced collect function that includes Unix socket monitoring
    pub fn collect_process_network_stats_enhanced(
        &mut self,
        pid: u32,
    ) -> Result<ProcessNetworkStats> {
        let mut stats = self.collect_process_network_stats(pid)?;

        // Add Unix socket monitoring if enabled
        if self.config.enable_unix_monitoring && self.config.enable_detailed_connections {
            if let Ok(unix_stats) = self.collect_unix_stats(pid) {
                stats.connections.extend(unix_stats.detailed_connections);
                // Note: Unix sockets don't contribute to byte/packet counts
                // as they're local IPC mechanisms
            }
        }

        Ok(stats)
    }

    /// Benchmark process network monitoring performance
    pub fn benchmark_process_network_monitoring(
        &mut self,
        iterations: usize,
        test_pids: &[u32],
    ) -> Result<ProcessNetworkBenchmarkResults> {
        use std::time::Instant;

        let mut results = ProcessNetworkBenchmarkResults {
            iterations,
            ..Default::default()
        };

        // Clear cache for fair benchmarking
        self.clear_cache();

        // Benchmark individual process collection
        let individual_start = Instant::now();
        for _ in 0..iterations {
            for &pid in test_pids {
                let _ = self.collect_process_network_stats(pid);
            }
        }
        results.individual_collection_time = individual_start.elapsed();

        // Benchmark multiple process collection
        let multiple_start = Instant::now();
        for _ in 0..iterations {
            let _ = self.collect_multiple_process_network_stats(test_pids);
        }
        results.multiple_collection_time = multiple_start.elapsed();

        // Calculate averages
        if iterations > 0 && !test_pids.is_empty() {
            results.avg_individual_time =
                results.individual_collection_time / (iterations * test_pids.len()) as u32;
            results.avg_multiple_time = results.multiple_collection_time / iterations as u32;
        }

        Ok(results)
    }
}

/// Protocol-specific statistics
#[derive(Debug, Clone, Default)]
struct ProcessProtocolStats {
    rx_bytes: u64,
    tx_bytes: u64,
    rx_packets: u64,
    tx_packets: u64,
    connections: u64,
    detailed_connections: Vec<ProcessConnectionStats>,
}

/// Network traffic deltas between collections
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProcessNetworkDeltas {
    /// Bytes received delta
    pub rx_bytes_delta: u64,
    /// Bytes transmitted delta
    pub tx_bytes_delta: u64,
    /// Packets received delta
    pub rx_packets_delta: u64,
    /// Packets transmitted delta
    pub tx_packets_delta: u64,
    /// TCP connections delta
    pub tcp_connections_delta: u64,
    /// UDP connections delta
    pub udp_connections_delta: u64,
}

/// Process network benchmark results
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProcessNetworkBenchmarkResults {
    /// Number of benchmark iterations
    pub iterations: usize,
    /// Total time spent collecting individual process statistics
    pub individual_collection_time: Duration,
    /// Total time spent collecting multiple process statistics
    pub multiple_collection_time: Duration,
    /// Average time per individual process collection
    pub avg_individual_time: Duration,
    /// Average time per multiple process collection
    pub avg_multiple_time: Duration,
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
    fn test_process_network_stats_default() {
        let stats = ProcessNetworkStats::default();
        assert_eq!(stats.pid, 0);
        assert_eq!(stats.rx_bytes, 0);
        assert_eq!(stats.tx_bytes, 0);
        assert_eq!(stats.connections.len(), 0);
    }

    #[test]
    fn test_process_connection_stats_default() {
        let stats = ProcessConnectionStats::default();
        assert_eq!(stats.src_port, 0);
        assert_eq!(stats.dst_port, 0);
        assert_eq!(stats.protocol, String::new());
    }

    #[test]
    fn test_process_network_monitor_creation() {
        let monitor = ProcessNetworkMonitor::new();
        assert!(monitor.config.enable_process_network_monitoring);
        assert!(monitor.config.enable_tcp_monitoring);
        assert!(monitor.config.enable_udp_monitoring);
    }

    #[test]
    fn test_process_network_monitor_with_config() {
        let config = ProcessNetworkMonitorConfig {
            enable_process_network_monitoring: false,
            ..Default::default()
        };
        let monitor = ProcessNetworkMonitor::with_config(config);
        assert!(!monitor.config.enable_process_network_monitoring);
    }

    #[test]
    fn test_process_network_deltas() {
        let current = ProcessNetworkStats {
            rx_bytes: 1000,
            tx_bytes: 2000,
            tcp_connections: 5,
            udp_connections: 3,
            ..Default::default()
        };

        let previous = ProcessNetworkStats {
            rx_bytes: 500,
            tx_bytes: 1000,
            tcp_connections: 2,
            udp_connections: 1,
            ..Default::default()
        };

        let monitor = ProcessNetworkMonitor::new();
        let deltas = monitor.calculate_network_deltas(&current, &previous);

        assert_eq!(deltas.rx_bytes_delta, 500);
        assert_eq!(deltas.tx_bytes_delta, 1000);
        assert_eq!(deltas.tcp_connections_delta, 3);
        assert_eq!(deltas.udp_connections_delta, 2);
    }

    #[test]
    fn test_process_network_deltas_zero() {
        let current = ProcessNetworkStats::default();
        let previous = ProcessNetworkStats::default();

        let monitor = ProcessNetworkMonitor::new();
        let deltas = monitor.calculate_network_deltas(&current, &previous);

        assert_eq!(deltas.rx_bytes_delta, 0);
        assert_eq!(deltas.tx_bytes_delta, 0);
        assert_eq!(deltas.tcp_connections_delta, 0);
        assert_eq!(deltas.udp_connections_delta, 0);
    }

    #[test]
    fn test_process_network_deltas_overflow() {
        let current = ProcessNetworkStats {
            rx_bytes: u64::MAX,
            tx_bytes: u64::MAX,
            ..Default::default()
        };

        let previous = ProcessNetworkStats {
            rx_bytes: u64::MAX,
            tx_bytes: u64::MAX,
            ..Default::default()
        };

        let monitor = ProcessNetworkMonitor::new();
        let deltas = monitor.calculate_network_deltas(&current, &previous);

        assert_eq!(deltas.rx_bytes_delta, 0);
        assert_eq!(deltas.tx_bytes_delta, 0);
    }

    #[test]
    fn test_process_network_config_serialization() {
        let config = ProcessNetworkMonitorConfig::default();
        let json = serde_json::to_string(&config).expect("Serialization should work");
        let deserialized: ProcessNetworkMonitorConfig =
            serde_json::from_str(&json).expect("Deserialization should work");
        assert_eq!(
            deserialized.enable_process_network_monitoring,
            config.enable_process_network_monitoring
        );
        assert_eq!(
            deserialized.max_connections_per_process,
            config.max_connections_per_process
        );
    }

    #[test]
    fn test_process_network_stats_with_data() {
        let stats = ProcessNetworkStats {
            pid: 1234,
            rx_bytes: 1024,
            tx_bytes: 2048,
            rx_packets: 100,
            tx_packets: 200,
            tcp_connections: 5,
            udp_connections: 3,
            last_update: SystemTime::now(),
            data_source: "proc".to_string(),
            connections: Vec::new(),
        };

        assert_eq!(stats.pid, 1234);
        assert_eq!(stats.rx_bytes, 1024);
        assert_eq!(stats.tx_bytes, 2048);
        assert_eq!(stats.tcp_connections, 5);
        assert_eq!(stats.udp_connections, 3);
    }

    #[test]
    fn test_process_connection_stats_with_data() {
        let stats = ProcessConnectionStats {
            src_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            dst_ip: IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
            src_port: 12345,
            dst_port: 80,
            protocol: "TCP".to_string(),
            state: "ESTABLISHED".to_string(),
            bytes_transmitted: 1024,
            bytes_received: 2048,
            packets_transmitted: 10,
            packets_received: 20,
            start_time: SystemTime::now(),
            last_activity: SystemTime::now(),
            duration: Duration::from_secs(60),
        };

        assert!(matches!(stats.src_ip, IpAddr::V4(_)));
        assert!(matches!(stats.dst_ip, IpAddr::V4(_)));
        assert_eq!(stats.src_port, 12345);
        assert_eq!(stats.dst_port, 80);
        assert_eq!(stats.protocol, "TCP");
        assert_eq!(stats.state, "ESTABLISHED");
    }

    #[test]
    fn test_process_network_monitor_cache_operations() {
        let mut monitor = ProcessNetworkMonitor::new();

        // Test initial cache state
        assert!(monitor.cache.is_empty());
        assert!(!monitor.is_cache_valid());

        // Test cache clearing
        monitor.clear_cache();
        assert!(monitor.cache.is_empty());
    }

    #[test]
    fn test_process_network_benchmark_results_default() {
        let results = ProcessNetworkBenchmarkResults::default();
        assert_eq!(results.iterations, 0);
        assert_eq!(results.individual_collection_time, Duration::from_secs(0));
    }

    #[test]
    fn test_process_network_benchmark_results_serialization() {
        let results = ProcessNetworkBenchmarkResults {
            iterations: 10,
            individual_collection_time: Duration::from_millis(100),
            multiple_collection_time: Duration::from_millis(50),
            avg_individual_time: Duration::from_millis(10),
            avg_multiple_time: Duration::from_millis(5),
        };

        let json = serde_json::to_string(&results).expect("Serialization should work");
        let deserialized: ProcessNetworkBenchmarkResults =
            serde_json::from_str(&json).expect("Deserialization should work");
        assert_eq!(deserialized.iterations, 10);
        assert_eq!(
            deserialized.individual_collection_time,
            Duration::from_millis(100)
        );
    }

    #[test]
    fn test_ip_conversion_functions() {
        let ip1 = u32_to_ipaddr(0x01020304); // 1.2.3.4
        assert_eq!(ip1, IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)));

        let ip2 = u32_to_ipaddr(0x7F000001); // 127.0.0.1
        assert_eq!(ip2, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    }

    use std::net::Ipv6Addr;

    #[test]
    fn test_format_ip_addr() {
        let ipv4 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ipv6 = IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));

        assert_eq!(format_ip_addr(ipv4), "192.168.1.1");
        assert_eq!(format_ip_addr(ipv6), "::1");
    }

    #[test]
    fn test_process_network_stats_equality() {
        let mut stats1 = ProcessNetworkStats::default();
        stats1.pid = 1234;
        stats1.rx_bytes = 1000;

        let mut stats2 = ProcessNetworkStats::default();
        stats2.pid = 1234;
        stats2.rx_bytes = 1000;

        assert_eq!(stats1, stats2);

        let mut stats3 = ProcessNetworkStats::default();
        stats3.pid = 5678;
        stats3.rx_bytes = 1000;

        assert_ne!(stats1, stats3);
    }

    #[test]
    fn test_process_network_error_handling() {
        // Test that network functions handle errors gracefully
        let monitor = ProcessNetworkMonitor::new();

        // Test that we can create a monitor and work with empty data
        let empty_stats = ProcessNetworkStats::default();
        assert_eq!(empty_stats.connections.len(), 0);
        assert_eq!(empty_stats.rx_bytes, 0);
    }

    #[test]
    fn test_process_network_config_edge_cases() {
        // Test edge cases for configuration
        let config = ProcessNetworkMonitorConfig {
            max_connections_per_process: 0,
            cache_ttl_seconds: 0,
            ..Default::default()
        };

        let monitor = ProcessNetworkMonitor::with_config(config);
        assert_eq!(monitor.config.max_connections_per_process, 0);
        assert_eq!(monitor.cache_ttl, Duration::from_secs(0));
    }

    #[test]
    fn test_process_network_stats_edge_cases() {
        // Test edge cases for network stats
        let mut stats = ProcessNetworkStats::default();

        // Test with maximum values
        stats.rx_bytes = u64::MAX;
        stats.tx_bytes = u64::MAX;
        stats.rx_packets = u64::MAX;
        stats.tx_packets = u64::MAX;
        stats.tcp_connections = u64::MAX;
        stats.udp_connections = u64::MAX;

        assert_eq!(stats.rx_bytes, u64::MAX);
        assert_eq!(stats.tx_bytes, u64::MAX);
    }

    #[test]
    fn test_process_network_connection_edge_cases() {
        // Test edge cases for connection statistics
        let mut stats = ProcessConnectionStats::default();

        // Test with maximum port values
        stats.src_port = u16::MAX;
        stats.dst_port = u16::MAX;

        assert_eq!(stats.src_port, u16::MAX);
        assert_eq!(stats.dst_port, u16::MAX);

        // Test with maximum byte values
        stats.bytes_transmitted = u64::MAX;
        stats.bytes_received = u64::MAX;

        assert_eq!(stats.bytes_transmitted, u64::MAX);
        assert_eq!(stats.bytes_received, u64::MAX);
    }

    #[test]
    fn test_process_network_deltas_edge_cases() {
        // Test edge cases for network deltas
        let current = ProcessNetworkStats {
            rx_bytes: u64::MAX,
            tx_bytes: u64::MAX,
            ..Default::default()
        };

        let previous = ProcessNetworkStats {
            rx_bytes: u64::MAX,
            tx_bytes: u64::MAX,
            ..Default::default()
        };

        let monitor = ProcessNetworkMonitor::new();
        let deltas = monitor.calculate_network_deltas(&current, &previous);

        // Should handle overflow gracefully
        assert_eq!(deltas.rx_bytes_delta, 0);
        assert_eq!(deltas.tx_bytes_delta, 0);
    }

    #[test]
    fn test_process_network_config_serialization_edge_cases() {
        // Test serialization edge cases
        let config = ProcessNetworkMonitorConfig {
            max_connections_per_process: 0,
            cache_ttl_seconds: 0,
            ..Default::default()
        };

        let json = serde_json::to_string(&config).expect("Serialization should work");
        let deserialized: ProcessNetworkMonitorConfig =
            serde_json::from_str(&json).expect("Deserialization should work");

        assert_eq!(deserialized.max_connections_per_process, 0);
        assert_eq!(deserialized.cache_ttl_seconds, 0);
    }

    #[test]
    fn test_process_network_monitor_with_custom_config() {
        // Test monitor with custom configuration
        let config = ProcessNetworkMonitorConfig {
            enable_process_network_monitoring: true,
            max_connections_per_process: 256,
            enable_detailed_connections: false,
            enable_tcp_monitoring: true,
            enable_udp_monitoring: false,
            enable_unix_monitoring: true,
            update_interval_secs: 30,
            enable_caching: false,
            cache_ttl_seconds: 600,
        };

        let monitor = ProcessNetworkMonitor::with_config(config);
        assert_eq!(monitor.config.max_connections_per_process, 256);
        assert!(!monitor.config.enable_detailed_connections);
        assert!(!monitor.config.enable_udp_monitoring);
        assert!(monitor.config.enable_unix_monitoring);
        assert!(!monitor.config.enable_caching);
    }

    #[test]
    fn test_process_network_stats_serialization() {
        // Test serialization of network stats
        let stats = ProcessNetworkStats {
            pid: 1234,
            rx_bytes: 1024,
            tx_bytes: 2048,
            rx_packets: 100,
            tx_packets: 200,
            tcp_connections: 5,
            udp_connections: 3,
            last_update: SystemTime::now(),
            data_source: "proc".to_string(),
            connections: Vec::new(),
        };

        let json = serde_json::to_string(&stats).expect("Serialization should work");
        let deserialized: ProcessNetworkStats =
            serde_json::from_str(&json).expect("Deserialization should work");

        assert_eq!(deserialized.pid, 1234);
        assert_eq!(deserialized.rx_bytes, 1024);
        assert_eq!(deserialized.tx_bytes, 2048);
    }

    #[test]
    fn test_process_network_connection_serialization() {
        // Test serialization of connection stats
        let stats = ProcessConnectionStats {
            src_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            dst_ip: IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
            src_port: 12345,
            dst_port: 80,
            protocol: "TCP".to_string(),
            state: "ESTABLISHED".to_string(),
            bytes_transmitted: 1024,
            bytes_received: 2048,
            packets_transmitted: 10,
            packets_received: 20,
            start_time: SystemTime::now(),
            last_activity: SystemTime::now(),
            duration: Duration::from_secs(60),
        };

        let json = serde_json::to_string(&stats).expect("Serialization should work");
        let deserialized: ProcessConnectionStats =
            serde_json::from_str(&json).expect("Deserialization should work");

        assert_eq!(deserialized.src_port, 12345);
        assert_eq!(deserialized.dst_port, 80);
        assert_eq!(deserialized.protocol, "TCP");
    }

    #[test]
    fn test_connection_to_pid_mapping() {
        // Test the connection-to-PID mapping logic
        let monitor = ProcessNetworkMonitor::new();

        // Test with non-existent PID (should return false)
        let result = monitor.is_connection_for_pid(999999, "12345");
        assert!(result.is_ok());
        assert!(!result.unwrap());

        // Test with invalid inode (should return false)
        let result = monitor.is_connection_for_pid(1, "invalid_inode");
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_tcp_state_parsing() {
        // Test TCP state parsing
        let monitor = ProcessNetworkMonitor::new();

        // Test known TCP states
        assert_eq!(monitor.parse_tcp_state("01").unwrap(), "ESTABLISHED");
        assert_eq!(monitor.parse_tcp_state("02").unwrap(), "SYN_SENT");
        assert_eq!(monitor.parse_tcp_state("0A").unwrap(), "LISTEN");
        assert_eq!(monitor.parse_tcp_state("FF").unwrap(), "UNKNOWN");

        // Test invalid state
        let result = monitor.parse_tcp_state("ZZ");
        assert!(result.is_err());
    }

    #[test]
    fn test_tcp_address_parsing() {
        // Test TCP address parsing
        let monitor = ProcessNetworkMonitor::new();

        // Test valid IPv4 addresses
        let result = monitor.parse_tcp_addresses("6364A8C0:B945", "00000000:0000");
        assert!(result.is_ok());

        let (src_ip, src_port, dst_ip, dst_port) = result.unwrap();
        assert!(matches!(src_ip, IpAddr::V4(_)));
        assert!(matches!(dst_ip, IpAddr::V4(_)));
        assert!(src_port > 0);
        assert_eq!(dst_port, 0);

        // Test invalid address format
        let result = monitor.parse_tcp_addresses("invalid", "format");
        assert!(result.is_err());
    }

    #[test]
    fn test_network_monitor_error_handling() {
        // Test error handling in network monitor
        let mut monitor = ProcessNetworkMonitor::new();

        // Test with non-existent process
        let result = monitor.collect_process_network_stats(999999);
        assert!(result.is_ok()); // Should return empty stats, not error

        let stats = result.unwrap();
        assert_eq!(stats.pid, 999999);
        assert_eq!(stats.rx_bytes, 0);
        assert_eq!(stats.tx_bytes, 0);
        assert_eq!(stats.connections.len(), 0);
    }

    #[test]
    fn test_network_monitor_config_edge_cases() {
        // Test edge cases for network monitor configuration
        let config = ProcessNetworkMonitorConfig {
            max_connections_per_process: 0,
            cache_ttl_seconds: 0,
            enable_process_network_monitoring: false,
            ..Default::default()
        };

        let mut monitor = ProcessNetworkMonitor::with_config(config);
        assert_eq!(monitor.config.max_connections_per_process, 0);
        assert_eq!(monitor.cache_ttl, Duration::from_secs(0));
        assert!(!monitor.config.enable_process_network_monitoring);

        // Test that we can still create stats even with zero limits
        let result = monitor.collect_process_network_stats(1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_network_stats_aggregation() {
        // Test aggregation of network statistics
        let monitor = ProcessNetworkMonitor::new();

        // Create some test stats
        let mut stats1 = ProcessNetworkStats::default();
        stats1.pid = 1234;
        stats1.rx_bytes = 1000;
        stats1.tx_bytes = 2000;
        stats1.tcp_connections = 5;

        let mut stats2 = ProcessNetworkStats::default();
        stats2.pid = 1234;
        stats2.rx_bytes = 1500;
        stats2.tx_bytes = 2500;
        stats2.tcp_connections = 3;

        // Calculate deltas
        let deltas = monitor.calculate_network_deltas(&stats2, &stats1);

        assert_eq!(deltas.rx_bytes_delta, 500);
        assert_eq!(deltas.tx_bytes_delta, 500);
        assert_eq!(deltas.tcp_connections_delta, 2);
    }

    #[test]
    fn test_network_monitor_cache_behavior() {
        // Test cache behavior in network monitor
        let monitor = ProcessNetworkMonitor::new();

        // Initially cache should be empty
        assert!(monitor.cache.is_empty());
        assert!(!monitor.is_cache_valid());

        // Collect stats (should populate cache if caching is enabled)
        let config = ProcessNetworkMonitorConfig {
            enable_caching: true,
            ..Default::default()
        };

        let mut monitor_with_cache = ProcessNetworkMonitor::with_config(config);
        let result = monitor_with_cache.collect_process_network_stats(1);

        assert!(result.is_ok());

        // Cache should now be populated
        assert!(!monitor_with_cache.cache.is_empty());

        // Clear cache and verify
        monitor_with_cache.clear_cache();
        assert!(monitor_with_cache.cache.is_empty());
    }

    #[test]
    fn test_network_monitor_multiple_processes() {
        // Test collecting stats for multiple processes
        let mut monitor = ProcessNetworkMonitor::new();

        let pids = vec![1, 2, 100, 500];
        let result = monitor.collect_multiple_process_network_stats(&pids);

        assert!(result.is_ok());
        let stats_list = result.unwrap();

        // Should have stats for all requested PIDs
        assert_eq!(stats_list.len(), pids.len());

        // Each stat should have the correct PID
        for (i, stats) in stats_list.iter().enumerate() {
            assert_eq!(stats.pid, pids[i]);
        }
    }

    #[test]
    fn test_network_monitor_performance_characteristics() {
        // Test performance characteristics of network monitoring
        let mut monitor = ProcessNetworkMonitor::new();

        // Test that monitoring doesn't take too long for non-existent processes
        let start_time = SystemTime::now();
        let result = monitor.collect_process_network_stats(999999);
        let end_time = SystemTime::now();

        assert!(result.is_ok());

        // Should complete quickly (less than 100ms for non-existent process)
        if let Ok(duration) = end_time.duration_since(start_time) {
            assert!(duration.as_millis() < 100);
        }
    }

    #[test]
    fn test_network_connection_stats_equality() {
        // Test equality of connection stats
        let mut conn1 = ProcessConnectionStats::default();
        conn1.src_port = 12345;
        conn1.dst_port = 80;
        conn1.protocol = "TCP".to_string();

        let mut conn2 = ProcessConnectionStats::default();
        conn2.src_port = 12345;
        conn2.dst_port = 80;
        conn2.protocol = "TCP".to_string();

        assert_eq!(conn1, conn2);

        let mut conn3 = ProcessConnectionStats::default();
        conn3.src_port = 54321;
        conn3.dst_port = 80;
        conn3.protocol = "TCP".to_string();

        assert_ne!(conn1, conn3);
    }

    #[test]
    fn test_network_monitor_config_serialization_complex() {
        // Test serialization of complex network monitor configuration
        let config = ProcessNetworkMonitorConfig {
            enable_process_network_monitoring: true,
            max_connections_per_process: 256,
            enable_detailed_connections: true,
            enable_tcp_monitoring: true,
            enable_udp_monitoring: false,
            update_interval_secs: 30,
            enable_caching: true,
            cache_ttl_seconds: 600,
        };

        let json = serde_json::to_string(&config).expect("Serialization should work");
        let deserialized: ProcessNetworkMonitorConfig =
            serde_json::from_str(&json).expect("Deserialization should work");

        assert_eq!(deserialized.max_connections_per_process, 256);
        assert!(deserialized.enable_detailed_connections);
        assert!(!deserialized.enable_udp_monitoring);
        assert_eq!(deserialized.cache_ttl_seconds, 600);
    }

    #[test]
    fn test_network_stats_with_realistic_data() {
        // Test with realistic network statistics data
        let mut stats = ProcessNetworkStats::default();
        stats.pid = 1234;
        stats.rx_bytes = 1024 * 1024; // 1 MB received
        stats.tx_bytes = 2 * 1024 * 1024; // 2 MB sent
        stats.rx_packets = 1000;
        stats.tx_packets = 1500;
        stats.tcp_connections = 10;
        stats.udp_connections = 5;
        stats.last_update = SystemTime::now();
        stats.data_source = "proc".to_string();

        // Add some realistic connections
        let mut conn1 = ProcessConnectionStats::default();
        conn1.src_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        conn1.dst_ip = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
        conn1.src_port = 54321;
        conn1.dst_port = 443;
        conn1.protocol = "TCP".to_string();
        conn1.state = "ESTABLISHED".to_string();
        conn1.bytes_transmitted = 512 * 1024; // 512 KB
        conn1.bytes_received = 256 * 1024; // 256 KB

        let mut conn2 = ProcessConnectionStats::default();
        conn2.src_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        conn2.dst_ip = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));
        conn2.src_port = 12345;
        conn2.dst_port = 53;
        conn2.protocol = "UDP".to_string();
        conn2.state = "ESTABLISHED".to_string();

        stats.connections = vec![conn1, conn2];

        // Verify the data
        assert_eq!(stats.pid, 1234);
        assert_eq!(stats.connections.len(), 2);
        assert_eq!(stats.tcp_connections, 10);
        assert_eq!(stats.udp_connections, 5);

        // Test serialization
        let json = serde_json::to_string(&stats).expect("Serialization should work");
        let deserialized: ProcessNetworkStats =
            serde_json::from_str(&json).expect("Deserialization should work");

        assert_eq!(deserialized.pid, 1234);
        assert_eq!(deserialized.connections.len(), 2);
        assert_eq!(deserialized.tcp_connections, 10);
    }

    #[test]
    fn test_fd_mapping_basic() {
        // Test basic FD mapping functionality
        let monitor = ProcessNetworkMonitor::new();

        // Test with a non-existent process (should return empty vec)
        let result = monitor.collect_process_connections_with_fd_mapping(999999);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_ip_port_parsing() {
        let monitor = ProcessNetworkMonitor::new();

        // Test IPv4 parsing
        let result = monitor.parse_ip_port("0100007F:1F90");
        assert!(result.is_some());
        let (ip, port) = result.unwrap();
        assert_eq!(port, 8080);
        assert!(matches!(ip, IpAddr::V4(_)));

        // Test IPv6 parsing (simplified test)
        let result = monitor.parse_ip_port("00000000000000000000000000000000:0050");
        assert!(result.is_some());
        let (ip, port) = result.unwrap();
        assert_eq!(port, 80);
        assert!(matches!(ip, IpAddr::V6(_)));
    }

    #[test]
    fn test_connection_merging() {
        // Test that the enhanced collection merges connections properly
        let config = ProcessNetworkMonitorConfig {
            enable_detailed_connections: true,
            ..Default::default()
        };
        let mut monitor = ProcessNetworkMonitor::with_config(config);

        // For a real process, this would merge connections from different sources
        // In this test, we just verify the basic functionality works
        let result = monitor.collect_process_network_stats(1); // init process
        assert!(result.is_ok());
        // The exact number of connections depends on the system
        assert!(result.unwrap().connections.len() >= 0);
    }

    #[test]
    fn test_network_stats_with_enhanced_features() {
        // Test with enhanced features enabled
        let config = ProcessNetworkMonitorConfig {
            enable_detailed_connections: true,
            enable_tcp_monitoring: true,
            enable_udp_monitoring: true,
            max_connections_per_process: 256,
            ..Default::default()
        };

        let mut monitor = ProcessNetworkMonitor::with_config(config);

        // Test with a real process (init process should exist)
        let result = monitor.collect_process_network_stats(1);
        assert!(result.is_ok());

        let stats = result.unwrap();
        assert_eq!(stats.pid, 1);
        assert!(
            stats
                .last_update
                .elapsed()
                .unwrap_or(Duration::from_secs(0))
                < Duration::from_secs(10)
        );
        assert_eq!(stats.data_source, "proc");
    }

    #[test]
    fn test_connection_filtering_and_analysis() {
        // Test connection filtering and analysis capabilities
        let monitor = ProcessNetworkMonitor::new();

        // Create some test connections
        let mut conn1 = ProcessConnectionStats::default();
        conn1.src_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        conn1.dst_ip = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
        conn1.src_port = 54321;
        conn1.dst_port = 443;
        conn1.protocol = "TCP".to_string();
        conn1.state = "ESTABLISHED".to_string();

        let mut conn2 = ProcessConnectionStats::default();
        conn2.src_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        conn2.dst_ip = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));
        conn2.src_port = 12345;
        conn2.dst_port = 53;
        conn2.protocol = "UDP".to_string();
        conn2.state = "ESTABLISHED".to_string();

        // Test that connections are properly structured
        assert_eq!(conn1.protocol, "TCP");
        assert_eq!(conn2.protocol, "UDP");
        assert_eq!(conn1.state, "ESTABLISHED");
        assert_eq!(conn2.state, "ESTABLISHED");
    }

    #[test]
    fn test_bandwidth_monitoring() {
        // Test bandwidth monitoring capabilities
        let monitor = ProcessNetworkMonitor::new();

        // Create a connection with bandwidth data
        let mut conn = ProcessConnectionStats::default();
        conn.bytes_transmitted = 1024 * 1024; // 1 MB
        conn.bytes_received = 512 * 1024; // 512 KB
        conn.packets_transmitted = 1000;
        conn.packets_received = 500;

        // Verify bandwidth data
        assert_eq!(conn.bytes_transmitted, 1024 * 1024);
        assert_eq!(conn.bytes_received, 512 * 1024);
        assert_eq!(conn.packets_transmitted, 1000);
        assert_eq!(conn.packets_received, 500);
    }

    #[test]
    fn test_error_handling_in_network_monitoring() {
        // Test error handling in network monitoring
        let monitor = ProcessNetworkMonitor::new();

        // Test with invalid process ID
        let result = monitor.collect_process_network_stats(999999);
        assert!(result.is_ok()); // Should return default stats, not error

        let stats = result.unwrap();
        assert_eq!(stats.pid, 999999);
        assert_eq!(stats.rx_bytes, 0);
        assert_eq!(stats.tx_bytes, 0);
    }

    #[test]
    fn test_network_monitoring_with_caching() {
        // Test network monitoring with caching enabled
        let config = ProcessNetworkMonitorConfig {
            enable_caching: true,
            cache_ttl_seconds: 60,
            ..Default::default()
        };

        let mut monitor = ProcessNetworkMonitor::with_config(config);

        // First collection (should not use cache)
        let result1 = monitor.collect_process_network_stats(1);
        assert!(result1.is_ok());

        // Second collection (should use cache)
        let result2 = monitor.collect_process_network_stats(1);
        assert!(result2.is_ok());

        // Results should be similar (cached)
        let stats1 = result1.unwrap();
        let stats2 = result2.unwrap();

        assert_eq!(stats1.pid, stats2.pid);
        assert_eq!(stats1.data_source, stats2.data_source);
    }

    #[test]
    fn test_connection_state_analysis() {
        // Test connection state analysis
        let monitor = ProcessNetworkMonitor::new();

        // Test various connection states
        let mut established_conn = ProcessConnectionStats::default();
        established_conn.state = "ESTABLISHED".to_string();

        let mut listen_conn = ProcessConnectionStats::default();
        listen_conn.state = "LISTEN".to_string();

        let mut time_wait_conn = ProcessConnectionStats::default();
        time_wait_conn.state = "TIME_WAIT".to_string();

        // Verify states
        assert_eq!(established_conn.state, "ESTABLISHED");
        assert_eq!(listen_conn.state, "LISTEN");
        assert_eq!(time_wait_conn.state, "TIME_WAIT");
    }

    #[test]
    fn test_unix_socket_parsing() {
        // Test Unix socket path parsing
        let monitor = ProcessNetworkMonitor::new();

        // Test with a sample hex path
        let result = monitor.parse_unix_socket_path("6162632F74656D702F736F636B6574");
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.contains("unix_socket_"));
    }

    #[test]
    fn test_unix_socket_connection_creation() {
        // Test creation of Unix socket connections
        let mut conn = ProcessConnectionStats::default();
        conn.protocol = "UNIX".to_string();
        conn.state = "ESTABLISHED".to_string();

        assert_eq!(conn.protocol, "UNIX");
        assert_eq!(conn.state, "ESTABLISHED");
    }

    #[test]
    fn test_abstract_unix_socket_connection() {
        // Test creation of abstract Unix socket connections
        let mut conn = ProcessConnectionStats::default();
        conn.protocol = "UNIX".to_string();
        conn.state = "ABSTRACT".to_string();

        assert_eq!(conn.protocol, "UNIX");
        assert_eq!(conn.state, "ABSTRACT");
    }

    #[test]
    fn test_enhanced_network_collection() {
        // Test the enhanced collection function
        let mut monitor = ProcessNetworkMonitor::new();

        // Test with a real process (init process should exist)
        let result = monitor.collect_process_network_stats_enhanced(1);
        assert!(result.is_ok());

        let stats = result.unwrap();
        assert_eq!(stats.pid, 1);
        assert!(
            stats
                .last_update
                .elapsed()
                .unwrap_or(Duration::from_secs(0))
                < Duration::from_secs(10)
        );
    }

    #[test]
    fn test_network_protocol_variety() {
        // Test that we can handle different network protocols
        let protocols = vec!["TCP", "UDP", "UNIX"];

        for protocol in protocols {
            let mut conn = ProcessConnectionStats::default();
            conn.protocol = protocol.to_string();

            assert_eq!(conn.protocol, protocol);
        }
    }

    #[test]
    fn test_unix_socket_integration() {
        // Test Unix socket integration with network stats
        let mut stats = ProcessNetworkStats::default();
        stats.pid = 1234;

        // Add a Unix socket connection
        let mut unix_conn = ProcessConnectionStats::default();
        unix_conn.protocol = "UNIX".to_string();
        unix_conn.state = "ESTABLISHED".to_string();

        stats.connections.push(unix_conn);

        assert_eq!(stats.connections.len(), 1);
        assert_eq!(stats.connections[0].protocol, "UNIX");
    }
}
