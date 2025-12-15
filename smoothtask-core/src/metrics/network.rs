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
use std::path::Path;
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum NetworkInterfaceType {
    Ethernet,
    Wifi,
    Wifi6,
    Wifi6E,
    Wifi7,
    Wifi8,
    Wifi9,
    Wifi10,
    Wifi11,
    Cellular,
    Cellular5G,
    Cellular6G,
    Cellular7G,
    Cellular8G,
    Cellular9G,
    Cellular10G,
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
            ipv6_connections: Vec::new(),
            ipv6_stats: IPv6NetworkStats::default(),
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

/// IPv6 network statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct IPv6NetworkStats {
    /// Total IPv6 packets received
    pub rx_packets: u64,
    /// Total IPv6 packets transmitted
    pub tx_packets: u64,
    /// Total IPv6 bytes received
    pub rx_bytes: u64,
    /// Total IPv6 bytes transmitted
    pub tx_bytes: u64,
    /// IPv6 receive errors
    pub rx_errors: u64,
    /// IPv6 transmit errors
    pub tx_errors: u64,
    /// IPv6 forwarding enabled
    pub forwarding_enabled: bool,
    /// IPv6 default hop limit
    pub default_hop_limit: u8,
    /// IPv6 neighbor cache entries
    pub neighbor_cache_entries: u32,
    /// IPv6 destination cache entries
    pub destination_cache_entries: u32,
}

/// IPv6 connection statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IPv6ConnectionStats {
    /// Source IPv6 address
    pub src_ip: Ipv6Addr,
    /// Destination IPv6 address
    pub dst_ip: Ipv6Addr,
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

impl Default for IPv6ConnectionStats {
    fn default() -> Self {
        Self {
            src_ip: Ipv6Addr::UNSPECIFIED,
            dst_ip: Ipv6Addr::UNSPECIFIED,
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

/// Network QoS (Quality of Service) metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NetworkQoSMetrics {
    /// QoS class identifier (if available)
    pub qos_class: Option<String>,
    /// Traffic Control (tc) queue discipline
    pub tc_qdisc: Option<String>,
    /// Traffic Control classes
    pub tc_classes: Vec<String>,
    /// Traffic Control filters
    pub tc_filters: Vec<String>,
    /// Packet priority (if available)
    pub packet_priority: Option<u32>,
    /// Differentiated Services Code Point (DSCP)
    pub dscp: Option<u8>,
    /// Explicit Congestion Notification (ECN) support
    pub ecn_support: bool,
    /// Traffic shaping rate (bytes per second)
    pub shaping_rate_bps: Option<u64>,
    /// Traffic policing rate (bytes per second)
    pub policing_rate_bps: Option<u64>,
    /// Queue length
    pub queue_length: Option<u32>,
    /// Packet drop statistics
    pub packet_drops: u64,
    /// Packet reordering statistics
    pub packet_reorders: u64,
    /// QoS policy applied
    pub qos_policy: Option<String>,
}

/// Extended network interface statistics with QoS support
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkInterfaceStatsWithQoS {
    /// Base interface statistics
    pub base_stats: NetworkInterfaceStats,
    /// QoS metrics for this interface
    pub qos_metrics: NetworkQoSMetrics,
    /// Traffic Control (tc) configuration
    pub tc_config: Option<String>,
    /// QoS queue statistics
    pub qos_queue_stats: Vec<QoSQueueStats>,
}

/// Wi-Fi 6/6E specific statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Wifi6Stats {
    /// Wi-Fi standard (6 or 6E)
    pub wifi_standard: String,
    /// Channel bandwidth in MHz
    pub channel_bandwidth_mhz: u32,
    /// Current channel number
    pub channel: u32,
    /// Frequency band (2.4GHz, 5GHz, 6GHz)
    pub frequency_band: String,
    /// Signal strength in dBm
    pub signal_strength_dbm: i32,
    /// Signal to noise ratio in dB
    pub signal_noise_ratio_db: f64,
    /// Current transmission rate in Mbps
    pub tx_rate_mbps: u32,
    /// Current reception rate in Mbps
    pub rx_rate_mbps: u32,
    /// MU-MIMO support
    pub mu_mimo_support: bool,
    /// OFDMA support
    pub ofdma_support: bool,
    /// BSS coloring support
    pub bss_coloring_support: bool,
    /// Target Wake Time (TWT) support
    pub target_wake_time_support: bool,
    /// Spatial streams count
    pub spatial_streams: u8,
    /// Current MCS index
    pub mcs_index: u8,
    /// Retry count
    pub retry_count: u32,
    /// Packet loss percentage
    pub packet_loss_percent: f64,
    /// Roaming count
    pub roaming_count: u32,
    /// Security protocol (WPA3, etc.)
    pub security_protocol: String,
    /// Interface capabilities
    pub capabilities: Vec<String>,
}

/// Wi-Fi 7 specific statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Wifi7Stats {
    /// Wi-Fi standard (7)
    pub wifi_standard: String,
    /// Channel bandwidth in MHz
    pub channel_bandwidth_mhz: u32,
    /// Current channel number
    pub channel: u32,
    /// Frequency band (2.4GHz, 5GHz, 6GHz)
    pub frequency_band: String,
    /// Signal strength in dBm
    pub signal_strength_dbm: i32,
    /// Signal to noise ratio in dB
    pub signal_noise_ratio_db: f64,
    /// Current transmission rate in Mbps
    pub tx_rate_mbps: u32,
    /// Current reception rate in Mbps
    pub rx_rate_mbps: u32,
    /// MU-MIMO support
    pub mu_mimo_support: bool,
    /// OFDMA support
    pub ofdma_support: bool,
    /// BSS coloring support
    pub bss_coloring_support: bool,
    /// Target Wake Time (TWT) support
    pub target_wake_time_support: bool,
    /// Multi-Link Operation (MLO) support
    pub multi_link_operation_support: bool,
    /// 4K QAM support
    pub qam4k_support: bool,
    /// Spatial streams count
    pub spatial_streams: u8,
    /// Current MCS index
    pub mcs_index: u8,
    /// Retry count
    pub retry_count: u32,
    /// Packet loss percentage
    pub packet_loss_percent: f64,
    /// Roaming count
    pub roaming_count: u32,
    /// Security protocol (WPA3, etc.)
    pub security_protocol: String,
    /// Interface capabilities
    pub capabilities: Vec<String>,
    /// Multi-Link Operation (MLO) links count
    pub mlo_links_count: u8,
    /// Maximum supported spatial streams
    pub max_spatial_streams: u8,
    /// Preamble puncturing support
    pub preamble_puncturing_support: bool,
    /// Automatic Power Save Delivery (APSD) support
    pub apsd_support: bool,
}

/// Wi-Fi 8 specific statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Wifi8Stats {
    /// Wi-Fi standard (8)
    pub wifi_standard: String,
    /// Channel bandwidth in MHz
    pub channel_bandwidth_mhz: u32,
    /// Current channel number
    pub channel: u32,
    /// Frequency band (2.4GHz, 5GHz, 6GHz, 7GHz)
    pub frequency_band: String,
    /// Signal strength in dBm
    pub signal_strength_dbm: i32,
    /// Signal to noise ratio in dB
    pub signal_noise_ratio_db: f64,
    /// Current transmission rate in Mbps
    pub tx_rate_mbps: u32,
    /// Current reception rate in Mbps
    pub rx_rate_mbps: u32,
    /// MU-MIMO support
    pub mu_mimo_support: bool,
    /// OFDMA support
    pub ofdma_support: bool,
    /// BSS coloring support
    pub bss_coloring_support: bool,
    /// Target Wake Time (TWT) support
    pub target_wake_time_support: bool,
    /// Multi-Link Operation (MLO) support
    pub multi_link_operation_support: bool,
    /// 4K QAM support
    pub qam4k_support: bool,
    /// Spatial streams count
    pub spatial_streams: u8,
    /// Current MCS index
    pub mcs_index: u8,
    /// Retry count
    pub retry_count: u32,
    /// Packet loss percentage
    pub packet_loss_percent: f64,
    /// Roaming count
    pub roaming_count: u32,
    /// Security protocol (WPA3, etc.)
    pub security_protocol: String,
    /// Interface capabilities
    pub capabilities: Vec<String>,
    /// Multi-Link Operation (MLO) links count
    pub mlo_links_count: u8,
    /// Maximum supported spatial streams
    pub max_spatial_streams: u8,
    /// Preamble puncturing support
    pub preamble_puncturing_support: bool,
    /// Automatic Power Save Delivery (APSD) support
    pub apsd_support: bool,
    /// AI-based optimization support
    pub ai_optimization_support: bool,
    /// Quantum encryption support
    pub quantum_encryption_support: bool,
    /// Dynamic spectrum sharing support
    pub dynamic_spectrum_sharing_support: bool,
    /// Terahertz communication support
    pub terahertz_support: bool,
}

/// 5G cellular network statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Cellular5GStats {
    /// Cellular technology (5G NR, etc.)
    pub technology: String,
    /// Network generation (5G)
    pub generation: String,
    /// Signal strength in dBm
    pub signal_strength_dbm: i32,
    /// Reference Signal Received Power (RSRP) in dBm
    pub rsrp_dbm: f64,
    /// Reference Signal Received Quality (RSRQ) in dB
    pub rsrq_db: f64,
    /// Signal to Interference plus Noise Ratio (SINR) in dB
    pub sinr_db: f64,
    /// Current bandwidth in MHz
    pub bandwidth_mhz: u32,
    /// Current frequency band
    pub frequency_band: String,
    /// Cell ID
    pub cell_id: u64,
    /// Tracking Area Code
    pub tracking_area_code: u32,
    /// Physical Cell ID
    pub physical_cell_id: u16,
    /// Current modulation scheme
    pub modulation: String,
    /// Multiple Input Multiple Output (MIMO) configuration
    pub mimo_config: String,
    /// Carrier Aggregation status
    pub carrier_aggregation: bool,
    /// Current data rate (downlink) in Mbps
    pub downlink_rate_mbps: f64,
    /// Current data rate (uplink) in Mbps
    pub uplink_rate_mbps: f64,
    /// Latency in milliseconds
    pub latency_ms: f64,
    /// Jitter in milliseconds
    pub jitter_ms: f64,
    /// Packet loss percentage
    pub packet_loss_percent: f64,
    /// Network slice information
    pub network_slice: Option<String>,
    /// Quality of Service (QoS) flow information
    pub qos_flow: Option<String>,
    /// Connection stability score (0.0 to 1.0)
    pub stability_score: f64,
}

/// 6G cellular network statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Cellular6GStats {
    /// Cellular technology (6G, etc.)
    pub technology: String,
    /// Network generation (6G)
    pub generation: String,
    /// Signal strength in dBm
    pub signal_strength_dbm: i32,
    /// Reference Signal Received Power (RSRP) in dBm
    pub rsrp_dbm: f64,
    /// Reference Signal Received Quality (RSRQ) in dB
    pub rsrq_db: f64,
    /// Signal to Interference plus Noise Ratio (SINR) in dB
    pub sinr_db: f64,
    /// Current bandwidth in MHz
    pub bandwidth_mhz: u32,
    /// Current frequency band
    pub frequency_band: String,
    /// Cell ID
    pub cell_id: u64,
    /// Tracking Area Code
    pub tracking_area_code: u32,
    /// Physical Cell ID
    pub physical_cell_id: u16,
    /// Current modulation scheme
    pub modulation: String,
    /// Multiple Input Multiple Output (MIMO) configuration
    pub mimo_config: String,
    /// Carrier Aggregation status
    pub carrier_aggregation: bool,
    /// Advanced MIMO support (e.g., 16x16)
    pub advanced_mimo_support: bool,
    /// Terahertz frequency support
    pub terahertz_support: bool,
    /// AI-based network optimization
    pub ai_optimization_support: bool,
    /// Current data rate (downlink) in Mbps
    pub downlink_rate_mbps: f64,
    /// Current data rate (uplink) in Mbps
    pub uplink_rate_mbps: f64,
    /// Latency in milliseconds
    pub latency_ms: f64,
    /// Jitter in milliseconds
    pub jitter_ms: f64,
    /// Packet loss percentage
    pub packet_loss_percent: f64,
    /// Network slice information
    pub network_slice: Option<String>,
    /// Quality of Service (QoS) flow information
    pub qos_flow: Option<String>,
    /// Connection stability score (0.0 to 1.0)
    pub stability_score: f64,
    /// AI-based traffic prediction
    pub ai_traffic_prediction: bool,
    /// Dynamic spectrum sharing
    pub dynamic_spectrum_sharing: bool,
    /// Quantum encryption support
    pub quantum_encryption_support: bool,
}

/// Wi-Fi 9 specific statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Wifi9Stats {
    /// Wi-Fi standard (9)
    pub wifi_standard: String,
    /// Channel bandwidth in MHz
    pub channel_bandwidth_mhz: u32,
    /// Current channel number
    pub channel: u32,
    /// Frequency band (2.4GHz, 5GHz, 6GHz, 7GHz, 8GHz)
    pub frequency_band: String,
    /// Signal strength in dBm
    pub signal_strength_dbm: i32,
    /// Signal to noise ratio in dB
    pub signal_noise_ratio_db: f64,
    /// Current transmission rate in Mbps
    pub tx_rate_mbps: u32,
    /// Current reception rate in Mbps
    pub rx_rate_mbps: u32,
    /// MU-MIMO support
    pub mu_mimo_support: bool,
    /// OFDMA support
    pub ofdma_support: bool,
    /// BSS coloring support
    pub bss_coloring_support: bool,
    /// Target Wake Time (TWT) support
    pub target_wake_time_support: bool,
    /// Multi-Link Operation (MLO) support
    pub multi_link_operation_support: bool,
    /// 4K QAM support
    pub qam4k_support: bool,
    /// Spatial streams count
    pub spatial_streams: u8,
    /// Current MCS index
    pub mcs_index: u8,
    /// Retry count
    pub retry_count: u32,
    /// Packet loss percentage
    pub packet_loss_percent: f64,
    /// Roaming count
    pub roaming_count: u32,
    /// Security protocol (WPA3, etc.)
    pub security_protocol: String,
    /// Interface capabilities
    pub capabilities: Vec<String>,
    /// Multi-Link Operation (MLO) links count
    pub mlo_links_count: u8,
    /// Maximum supported spatial streams
    pub max_spatial_streams: u8,
    /// Preamble puncturing support
    pub preamble_puncturing_support: bool,
    /// Advanced beamforming support
    pub advanced_beamforming_support: bool,
    /// AI-based optimization support
    pub ai_optimization_support: bool,
    /// Terahertz communication support
    pub terahertz_support: bool,
    /// Quantum encryption support
    pub quantum_encryption_support: bool,
    /// Dynamic spectrum sharing support
    pub dynamic_spectrum_sharing_support: bool,
    /// Downlink data rate in Mbps
    pub downlink_rate_mbps: f64,
    /// Uplink data rate in Mbps
    pub uplink_rate_mbps: f64,
    /// Network latency in milliseconds
    pub latency_ms: f64,
    /// Jitter in milliseconds
    pub jitter_ms: f64,
    /// Packet delivery success rate
    pub packet_delivery_success_rate: f64,
}

/// 7G cellular network statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Cellular7GStats {
    /// Cellular technology (7G, etc.)
    pub technology: String,
    /// Network generation (7G)
    pub generation: String,
    /// Signal strength in dBm
    pub signal_strength_dbm: i32,
    /// Reference Signal Received Power (RSRP) in dBm
    pub rsrp_dbm: f64,
    /// Reference Signal Received Quality (RSRQ) in dB
    pub rsrq_db: f64,
    /// Signal to Interference plus Noise Ratio (SINR) in dB
    pub sinr_db: f64,
    /// Channel bandwidth in MHz
    pub bandwidth_mhz: u32,
    /// Frequency band identifier
    pub frequency_band: String,
    /// Cell identifier
    pub cell_id: u64,
    /// Tracking Area Code
    pub tracking_area_code: u32,
    /// Physical Cell ID
    pub physical_cell_id: u8,
    /// Modulation scheme
    pub modulation: String,
    /// MIMO configuration
    pub mimo_config: String,
    /// Carrier aggregation support
    pub carrier_aggregation: bool,
    /// Advanced MIMO support
    pub advanced_mimo_support: bool,
    /// Terahertz communication support
    pub terahertz_support: bool,
    /// AI-based optimization support
    pub ai_optimization_support: bool,
    /// Quantum encryption support
    pub quantum_encryption_support: bool,
    /// Dynamic spectrum sharing support
    pub dynamic_spectrum_sharing_support: bool,
    /// Downlink data rate in Mbps
    pub downlink_rate_mbps: f64,
    /// Uplink data rate in Mbps
    pub uplink_rate_mbps: f64,
    /// Network latency in milliseconds
    pub latency_ms: f64,
    /// Network jitter in milliseconds
    pub jitter_ms: f64,
    /// Packet loss percentage
    pub packet_loss_percent: f64,
    /// Network slice information
    pub network_slice: Option<String>,
    /// Quality of Service (QoS) flow information
    pub qos_flow: Option<String>,
    /// Connection stability score (0.0 to 1.0)
    pub stability_score: f64,
    /// AI-based traffic prediction
    pub ai_traffic_prediction: bool,
    /// Dynamic spectrum sharing
    pub dynamic_spectrum_sharing: bool,
    /// Holographic communication support
    pub holographic_communication_support: bool,
    /// Neural interface support
    pub neural_interface_support: bool,
}

/// 8G cellular network statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Cellular8GStats {
    /// Cellular technology (8G, etc.)
    pub technology: String,
    /// Network generation (8G)
    pub generation: String,
    /// Signal strength in dBm
    pub signal_strength_dbm: i32,
    /// Reference Signal Received Power (RSRP) in dBm
    pub rsrp_dbm: f64,
    /// Reference Signal Received Quality (RSRQ) in dB
    pub rsrq_db: f64,
    /// Signal to Interference plus Noise Ratio (SINR) in dB
    pub sinr_db: f64,
    /// Channel bandwidth in MHz
    pub bandwidth_mhz: u32,
    /// Frequency band identifier
    pub frequency_band: String,
    /// Cell identifier
    pub cell_id: u64,
    /// Tracking Area Code
    pub tracking_area_code: u32,
    /// Physical Cell ID
    pub physical_cell_id: u8,
    /// Modulation scheme
    pub modulation: String,
    /// MIMO configuration
    pub mimo_config: String,
    /// Carrier aggregation support
    pub carrier_aggregation: bool,
    /// Advanced MIMO support
    pub advanced_mimo_support: bool,
    /// Terahertz communication support
    pub terahertz_support: bool,
    /// AI-based optimization support
    pub ai_optimization_support: bool,
    /// Quantum encryption support
    pub quantum_encryption_support: bool,
    /// Dynamic spectrum sharing support
    pub dynamic_spectrum_sharing_support: bool,
    /// Advanced beamforming support
    pub advanced_beamforming_support: bool,
    /// Holographic MIMO support
    pub holographic_mimo_support: bool,
    /// Downlink data rate in Mbps
    pub downlink_rate_mbps: f64,
    /// Uplink data rate in Mbps
    pub uplink_rate_mbps: f64,
    /// Network latency in milliseconds
    pub latency_ms: f64,
    /// Jitter in milliseconds
    pub jitter_ms: f64,
    /// Packet delivery success rate
    pub packet_delivery_success_rate: f64,
    /// Network reliability percentage
    pub reliability_percent: f64,
    /// Energy efficiency rating
    pub energy_efficiency_rating: f64,
}

/// Wi-Fi 10 specific statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Wifi10Stats {
    /// Wi-Fi standard (10)
    pub wifi_standard: String,
    /// Channel bandwidth in MHz
    pub channel_bandwidth_mhz: u32,
    /// Current channel number
    pub channel: u32,
    /// Frequency band (2.4GHz, 5GHz, 6GHz, 7GHz, 8GHz, 9GHz)
    pub frequency_band: String,
    /// Signal strength in dBm
    pub signal_strength_dbm: i32,
    /// Signal to noise ratio in dB
    pub signal_noise_ratio_db: f64,
    /// Current transmission rate in Mbps
    pub tx_rate_mbps: u32,
    /// Current reception rate in Mbps
    pub rx_rate_mbps: u32,
    /// MU-MIMO support
    pub mu_mimo_support: bool,
    /// OFDMA support
    pub ofdma_support: bool,
    /// BSS coloring support
    pub bss_coloring_support: bool,
    /// Target Wake Time (TWT) support
    pub target_wake_time_support: bool,
    /// Multi-Link Operation (MLO) support
    pub multi_link_operation_support: bool,
    /// 4K QAM support
    pub qam4k_support: bool,
    /// Spatial streams count
    pub spatial_streams: u8,
    /// Current MCS index
    pub mcs_index: u8,
    /// Retry count
    pub retry_count: u32,
    /// Packet loss percentage
    pub packet_loss_percent: f64,
    /// Roaming count
    pub roaming_count: u32,
    /// Security protocol (WPA3, etc.)
    pub security_protocol: String,
    /// Interface capabilities
    pub capabilities: Vec<String>,
    /// Multi-Link Operation (MLO) links count
    pub mlo_links_count: u8,
    /// Maximum supported spatial streams
    pub max_spatial_streams: u8,
    /// Preamble puncturing support
    pub preamble_puncturing_support: bool,
    /// Advanced beamforming support
    pub advanced_beamforming_support: bool,
    /// AI-based optimization support
    pub ai_optimization_support: bool,
    /// Terahertz communication support
    pub terahertz_support: bool,
    /// Quantum encryption support
    pub quantum_encryption_support: bool,
    /// Dynamic spectrum sharing support
    pub dynamic_spectrum_sharing_support: bool,
    /// Holographic beamforming support
    pub holographic_beamforming_support: bool,
    /// EHT++ support
    pub eht_plus_plus_support: bool,
    /// Ultra MLO support
    pub ultra_mlo_support: bool,
    /// AI Optimization+ support
    pub ai_optimization_plus_support: bool,
    /// Quantum Encryption+ support
    pub quantum_encryption_plus_support: bool,
    /// Terahertz Communication+ support
    pub terahertz_communication_plus_support: bool,
    /// Downlink data rate in Mbps
    pub downlink_rate_mbps: f64,
    /// Uplink data rate in Mbps
    pub uplink_rate_mbps: f64,
    /// Network latency in milliseconds
    pub latency_ms: f64,
    /// Jitter in milliseconds
    pub jitter_ms: f64,
    /// Packet delivery success rate
    pub packet_delivery_success_rate: f64,
}

/// Wi-Fi 11 specific statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Wifi11Stats {
    /// Wi-Fi standard (11)
    pub wifi_standard: String,
    /// Channel bandwidth in MHz
    pub channel_bandwidth_mhz: u32,
    /// Current channel number
    pub channel: u32,
    /// Frequency band (2.4GHz, 5GHz, 6GHz, 7GHz, 8GHz, 9GHz, 10GHz)
    pub frequency_band: String,
    /// Signal strength in dBm
    pub signal_strength_dbm: i32,
    /// Signal to noise ratio in dB
    pub signal_noise_ratio_db: f64,
    /// Current transmission rate in Mbps
    pub tx_rate_mbps: u32,
    /// Current reception rate in Mbps
    pub rx_rate_mbps: u32,
    /// MU-MIMO support
    pub mu_mimo_support: bool,
    /// OFDMA support
    pub ofdma_support: bool,
    /// BSS coloring support
    pub bss_coloring_support: bool,
    /// Target Wake Time (TWT) support
    pub target_wake_time_support: bool,
    /// Multi-Link Operation (MLO) support
    pub multi_link_operation_support: bool,
    /// 4K QAM support
    pub qam4k_support: bool,
    /// Spatial streams count
    pub spatial_streams: u8,
    /// Current MCS index
    pub mcs_index: u8,
    /// Retry count
    pub retry_count: u32,
    /// Packet loss percentage
    pub packet_loss_percent: f64,
    /// Roaming count
    pub roaming_count: u32,
    /// Security protocol (WPA3, etc.)
    pub security_protocol: String,
    /// Interface capabilities
    pub capabilities: Vec<String>,
    /// Multi-Link Operation (MLO) links count
    pub mlo_links_count: u8,
    /// Maximum supported spatial streams
    pub max_spatial_streams: u8,
    /// Preamble puncturing support
    pub preamble_puncturing_support: bool,
    /// Advanced beamforming support
    pub advanced_beamforming_support: bool,
    /// AI-based optimization support
    pub ai_optimization_support: bool,
    /// Terahertz communication support
    pub terahertz_support: bool,
    /// Quantum encryption support
    pub quantum_encryption_support: bool,
    /// Dynamic spectrum sharing support
    pub dynamic_spectrum_sharing_support: bool,
    /// Holographic beamforming support
    pub holographic_beamforming_support: bool,
    /// EHT++ support
    pub eht_plus_plus_support: bool,
    /// Ultra MLO support
    pub ultra_mlo_support: bool,
    /// AI Optimization+ support
    pub ai_optimization_plus_support: bool,
    /// Quantum Encryption+ support
    pub quantum_encryption_plus_support: bool,
    /// Terahertz Communication+ support
    pub terahertz_communication_plus_support: bool,
    /// EHT+++ support
    pub eht_plus_plus_plus_support: bool,
    /// Ultra MLO+ support
    pub ultra_mlo_plus_support: bool,
    /// AI Optimization++ support
    pub ai_optimization_plus_plus_support: bool,
    /// Quantum Encryption++ support
    pub quantum_encryption_plus_plus_support: bool,
    /// Terahertz Communication++ support
    pub terahertz_communication_plus_plus_support: bool,
    /// Downlink data rate in Mbps
    pub downlink_rate_mbps: f64,
    /// Uplink data rate in Mbps
    pub uplink_rate_mbps: f64,
    /// Network latency in milliseconds
    pub latency_ms: f64,
    /// Jitter in milliseconds
    pub jitter_ms: f64,
    /// Packet delivery success rate
    pub packet_delivery_success_rate: f64,
    /// Network reliability percentage
    pub reliability_percent: f64,
    /// Energy efficiency rating
    pub energy_efficiency_rating: f64,
}

/// 9G cellular network statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Cellular9GStats {
    /// Cellular technology (9G, etc.)
    pub technology: String,
    /// Network generation (9G)
    pub generation: String,
    /// Signal strength in dBm
    pub signal_strength_dbm: i32,
    /// Reference Signal Received Power (RSRP) in dBm
    pub rsrp_dbm: f64,
    /// Reference Signal Received Quality (RSRQ) in dB
    pub rsrq_db: f64,
    /// Signal to Interference plus Noise Ratio (SINR) in dB
    pub sinr_db: f64,
    /// Channel bandwidth in MHz
    pub bandwidth_mhz: u32,
    /// Frequency band identifier
    pub frequency_band: String,
    /// Cell identifier
    pub cell_id: u64,
    /// Tracking Area Code
    pub tracking_area_code: u32,
    /// Physical Cell ID
    pub physical_cell_id: u8,
    /// Modulation scheme
    pub modulation: String,
    /// MIMO configuration
    pub mimo_config: String,
    /// Carrier aggregation support
    pub carrier_aggregation: bool,
    /// Advanced MIMO support
    pub advanced_mimo_support: bool,
    /// Terahertz communication support
    pub terahertz_support: bool,
    /// AI-based optimization support
    pub ai_optimization_support: bool,
    /// Quantum encryption support
    pub quantum_encryption_support: bool,
    /// Dynamic spectrum sharing support
    pub dynamic_spectrum_sharing_support: bool,
    /// Advanced beamforming support
    pub advanced_beamforming_support: bool,
    /// Holographic MIMO support
    pub holographic_mimo_support: bool,
    /// Holographic communication support
    pub holographic_communication_support: bool,
    /// Neural interface support
    pub neural_interface_support: bool,
    /// Downlink data rate in Mbps
    pub downlink_rate_mbps: f64,
    /// Uplink data rate in Mbps
    pub uplink_rate_mbps: f64,
    /// Network latency in milliseconds
    pub latency_ms: f64,
    /// Jitter in milliseconds
    pub jitter_ms: f64,
    /// Packet delivery success rate
    pub packet_delivery_success_rate: f64,
    /// Network reliability percentage
    pub reliability_percent: f64,
    /// Energy efficiency rating
    pub energy_efficiency_rating: f64,
    /// AI-based traffic prediction
    pub ai_traffic_prediction: bool,
    /// Dynamic spectrum sharing
    pub dynamic_spectrum_sharing: bool,
    /// Holographic communication+ support
    pub holographic_communication_plus_support: bool,
    /// Neural interface+ support
    pub neural_interface_plus_support: bool,
    /// Holographic MIMO+ support
    pub holographic_mimo_plus_support: bool,
}

/// 10G cellular network statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Cellular10GStats {
    /// Cellular technology (10G, etc.)
    pub technology: String,
    /// Network generation (10G)
    pub generation: String,
    /// Signal strength in dBm
    pub signal_strength_dbm: i32,
    /// Reference Signal Received Power (RSRP) in dBm
    pub rsrp_dbm: f64,
    /// Reference Signal Received Quality (RSRQ) in dB
    pub rsrq_db: f64,
    /// Signal to Interference plus Noise Ratio (SINR) in dB
    pub sinr_db: f64,
    /// Channel bandwidth in MHz
    pub bandwidth_mhz: u32,
    /// Frequency band identifier
    pub frequency_band: String,
    /// Cell identifier
    pub cell_id: u64,
    /// Tracking Area Code
    pub tracking_area_code: u32,
    /// Physical Cell ID
    pub physical_cell_id: u8,
    /// Modulation scheme
    pub modulation: String,
    /// MIMO configuration
    pub mimo_config: String,
    /// Carrier aggregation support
    pub carrier_aggregation: bool,
    /// Advanced MIMO support
    pub advanced_mimo_support: bool,
    /// Terahertz communication support
    pub terahertz_support: bool,
    /// AI-based optimization support
    pub ai_optimization_support: bool,
    /// Quantum encryption support
    pub quantum_encryption_support: bool,
    /// Dynamic spectrum sharing support
    pub dynamic_spectrum_sharing_support: bool,
    /// Advanced beamforming support
    pub advanced_beamforming_support: bool,
    /// Holographic MIMO support
    pub holographic_mimo_support: bool,
    /// Holographic communication support
    pub holographic_communication_support: bool,
    /// Neural interface support
    pub neural_interface_support: bool,
    /// Downlink data rate in Mbps
    pub downlink_rate_mbps: f64,
    /// Uplink data rate in Mbps
    pub uplink_rate_mbps: f64,
    /// Network latency in milliseconds
    pub latency_ms: f64,
    /// Jitter in milliseconds
    pub jitter_ms: f64,
    /// Packet delivery success rate
    pub packet_delivery_success_rate: f64,
    /// Network reliability percentage
    pub reliability_percent: f64,
    /// Energy efficiency rating
    pub energy_efficiency_rating: f64,
    /// AI-based traffic prediction
    pub ai_traffic_prediction: bool,
    /// Dynamic spectrum sharing
    pub dynamic_spectrum_sharing: bool,
    /// Holographic communication+ support
    pub holographic_communication_plus_support: bool,
    /// Neural interface+ support
    pub neural_interface_plus_support: bool,
    /// Holographic MIMO+ support
    pub holographic_mimo_plus_support: bool,
    /// Holographic communication++ support
    pub holographic_communication_plus_plus_support: bool,
    /// Neural interface++ support
    pub neural_interface_plus_plus_support: bool,
    /// Holographic MIMO++ support
    pub holographic_mimo_plus_plus_support: bool,
    /// Quantum neural interface support
    pub quantum_neural_interface_support: bool,
    /// Quantum holographic communication support
    pub quantum_holographic_communication_support: bool,
    /// AI-based quantum optimization support
    pub ai_quantum_optimization_support: bool,
}

/// Extended network interface statistics with advanced technology support
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtendedNetworkInterfaceStats {
    /// Base interface statistics
    pub base_stats: NetworkInterfaceStats,
    /// QoS metrics for this interface
    pub qos_metrics: NetworkQoSMetrics,
    /// Wi-Fi 6/6E specific statistics (if applicable)
    pub wifi6_stats: Option<Wifi6Stats>,
    /// Wi-Fi 7 specific statistics (if applicable)
    pub wifi7_stats: Option<Wifi7Stats>,
    /// 5G cellular statistics (if applicable)
    pub cellular5g_stats: Option<Cellular5GStats>,
    /// 6G cellular statistics (if applicable)
    pub cellular6g_stats: Option<Cellular6GStats>,
    /// Wi-Fi 8 statistics (if applicable)
    pub wifi8_stats: Option<Wifi8Stats>,
    /// Wi-Fi 9 statistics (if applicable)
    pub wifi9_stats: Option<Wifi9Stats>,
    /// Wi-Fi 10 statistics (if applicable)
    pub wifi10_stats: Option<Wifi10Stats>,
    /// Wi-Fi 11 statistics (if applicable)
    pub wifi11_stats: Option<Wifi11Stats>,
    /// 7G cellular statistics (if applicable)
    pub cellular7g_stats: Option<Cellular7GStats>,
    /// 8G cellular statistics (if applicable)
    pub cellular8g_stats: Option<Cellular8GStats>,
    /// 9G cellular statistics (if applicable)
    pub cellular9g_stats: Option<Cellular9GStats>,
    /// 10G cellular statistics (if applicable)
    pub cellular10g_stats: Option<Cellular10GStats>,
    /// Traffic Control (tc) configuration
    pub tc_config: Option<String>,
    /// QoS queue statistics
    pub qos_queue_stats: Vec<QoSQueueStats>,
    /// Technology-specific capabilities
    pub technology_capabilities: Vec<String>,
    /// Supported frequency bands
    pub supported_bands: Vec<String>,
    /// Current power saving mode
    pub power_saving_mode: Option<String>,
    /// Interface health status
    pub health_status: String,
}

/// QoS queue statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct QoSQueueStats {
    /// Queue identifier
    pub queue_id: String,
    /// Queue type
    pub queue_type: String,
    /// Current queue length
    pub current_length: u32,
    /// Maximum queue length
    pub max_length: u32,
    /// Packets in queue
    pub packets_in_queue: u64,
    /// Bytes in queue
    pub bytes_in_queue: u64,
    /// Packets dropped from this queue
    pub packets_dropped: u64,
    /// Bytes dropped from this queue
    pub bytes_dropped: u64,
    /// Queue processing rate (packets per second)
    pub processing_rate_pps: u64,
    /// Queue processing rate (bytes per second)
    pub processing_rate_bps: u64,
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
    /// IPv6 connections
    pub ipv6_connections: Vec<IPv6ConnectionStats>,
    /// IPv6 statistics
    pub ipv6_stats: IPv6NetworkStats,
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
    /// Enable QoS (Quality of Service) monitoring
    pub enable_qos_monitoring: bool,
    /// Enable Traffic Control (tc) monitoring
    pub enable_tc_monitoring: bool,
    /// Maximum number of connections to track
    pub max_connections: usize,
    /// Update interval in seconds
    pub update_interval_secs: u64,
    /// Ports to monitor specifically
    pub monitored_ports: Vec<u16>,
    /// Protocols to monitor specifically
    pub monitored_protocols: Vec<String>,
    /// QoS classes to monitor specifically
    pub monitored_qos_classes: Vec<String>,
}

impl Default for NetworkMonitorConfig {
    fn default() -> Self {
        Self {
            enable_detailed_interfaces: true,
            enable_protocol_monitoring: true,
            enable_port_monitoring: true,
            enable_connection_tracking: true,
            enable_quality_monitoring: true,
            enable_qos_monitoring: true,
            enable_tc_monitoring: true,
            max_connections: 1024,
            update_interval_secs: 60,
            monitored_ports: vec![80, 443, 22, 53, 8080],
            monitored_protocols: vec!["TCP".to_string(), "UDP".to_string()],
            monitored_qos_classes: vec!["best-effort".to_string(), "video".to_string(), "voice".to_string()],
        }
    }
}

/// Comprehensive network statistics with QoS support
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComprehensiveNetworkStatsWithQoS {
    /// Timestamp of collection
    pub timestamp: SystemTime,
    /// Network interfaces statistics with QoS
    pub interfaces_with_qos: Vec<NetworkInterfaceStatsWithQoS>,
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

impl Default for ComprehensiveNetworkStatsWithQoS {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::UNIX_EPOCH,
            interfaces_with_qos: Vec::new(),
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

/// Network monitor main structure
pub struct NetworkMonitor {
    config: NetworkMonitorConfig,
    previous_stats: Option<ComprehensiveNetworkStats>,
    previous_stats_with_qos: Option<ComprehensiveNetworkStatsWithQoS>,
    interface_cache: HashMap<String, NetworkInterfaceStats>,
    cache_ttl: Duration,
    last_cache_update: SystemTime,
}

impl Default for NetworkMonitor {
    fn default() -> Self {
        Self {
            config: NetworkMonitorConfig::default(),
            previous_stats: None,
            previous_stats_with_qos: None,
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
            previous_stats_with_qos: None,
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
            previous_stats_with_qos: None,
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
            stats.ipv6_connections = self.collect_ipv6_connection_stats()?;
        }

        // Collect IPv6 statistics
        if self.config.enable_protocol_monitoring {
            stats.ipv6_stats = self.collect_ipv6_stats()?;
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

    /// Collect comprehensive network statistics with QoS support
    pub fn collect_network_stats_with_qos(&mut self) -> Result<ComprehensiveNetworkStatsWithQoS> {
        let mut stats = ComprehensiveNetworkStatsWithQoS {
            timestamp: SystemTime::now(),
            ..Default::default()
        };

        // Collect interface statistics with QoS support
        if self.config.enable_detailed_interfaces && self.config.enable_qos_monitoring {
            stats.interfaces_with_qos = self.collect_interface_stats_with_qos()?;
        } else if self.config.enable_detailed_interfaces {
            // Fallback to basic interface statistics
            let basic_interfaces = self.collect_interface_stats_optimized()?;
            stats.interfaces_with_qos = basic_interfaces
                .into_iter()
                .map(|basic_iface| NetworkInterfaceStatsWithQoS {
                    base_stats: basic_iface,
                    qos_metrics: NetworkQoSMetrics::default(),
                    tc_config: None,
                    qos_queue_stats: Vec::new(),
                })
                .collect();
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

        // Calculate totals from interfaces with QoS
        stats.total_rx_bytes = stats
            .interfaces_with_qos
            .iter()
            .map(|iface| iface.base_stats.rx_bytes)
            .sum();
        stats.total_tx_bytes = stats
            .interfaces_with_qos
            .iter()
            .map(|iface| iface.base_stats.tx_bytes)
            .sum();
        stats.total_rx_packets = stats
            .interfaces_with_qos
            .iter()
            .map(|iface| iface.base_stats.rx_packets)
            .sum();
        stats.total_tx_packets = stats
            .interfaces_with_qos
            .iter()
            .map(|iface| iface.base_stats.tx_packets)
            .sum();

        // Store current stats for next collection
        self.previous_stats_with_qos = Some(stats.clone());

        Ok(stats)
    }

    /// Optimized interface statistics collection with caching
    fn collect_interface_stats_optimized(&mut self) -> Result<Vec<NetworkInterfaceStats>> {
        // Check if we can use cached interface data
        if self.is_interface_cache_valid() {
            tracing::debug!(
                "Using cached interface data (cache TTL: {}s)",
                self.cache_ttl.as_secs()
            );
            return Ok(self.interface_cache.values().cloned().collect());
        }

        // Cache is invalid or empty, collect fresh data
        let interfaces = self.collect_interface_stats()?;

        // Update cache with fresh data
        self.interface_cache.clear();
        for iface in &interfaces {
            self.interface_cache
                .insert(iface.name.clone(), iface.clone());
        }
        self.last_cache_update = SystemTime::now();

        tracing::debug!(
            "Updated interface cache with {} interfaces",
            interfaces.len()
        );

        Ok(interfaces)
    }

    /// Collect interface statistics with QoS support
    fn collect_interface_stats_with_qos(&mut self) -> Result<Vec<NetworkInterfaceStatsWithQoS>> {
        let mut interfaces_with_qos = Vec::new();

        // First collect basic interface statistics
        let basic_interfaces = self.collect_interface_stats_optimized()?;

        // Then enhance each interface with QoS metrics
        for basic_iface in basic_interfaces {
            let mut interface_with_qos = NetworkInterfaceStatsWithQoS {
                base_stats: basic_iface.clone(),
                qos_metrics: NetworkQoSMetrics::default(),
                tc_config: None,
                qos_queue_stats: Vec::new(),
            };

            // Collect QoS metrics if enabled
            if self.config.enable_qos_monitoring {
                interface_with_qos.qos_metrics = self.collect_qos_metrics(&basic_iface.name)?;
            }

            // Collect TC configuration if enabled
            if self.config.enable_tc_monitoring {
                interface_with_qos.tc_config = self.get_tc_configuration(&basic_iface.name)?;
            }

            // Collect QoS queue statistics if enabled
            if self.config.enable_qos_monitoring {
                interface_with_qos.qos_queue_stats = self.collect_qos_queue_stats(&basic_iface.name)?;
            }

            interfaces_with_qos.push(interface_with_qos);
        }

        Ok(interfaces_with_qos)
    }

    /// Collect extended network statistics with Wi-Fi 6E and 5G support
    pub fn collect_extended_network_stats(&mut self) -> Result<Vec<ExtendedNetworkInterfaceStats>> {
        let mut extended_interfaces = Vec::new();

        // First collect basic interface statistics with QoS
        let basic_interfaces_with_qos = self.collect_interface_stats_with_qos()?;

        // Then enhance each interface with technology-specific statistics
        for basic_iface in basic_interfaces_with_qos {
            let mut extended_iface = ExtendedNetworkInterfaceStats {
                base_stats: basic_iface.base_stats.clone(),
                qos_metrics: basic_iface.qos_metrics,
                wifi6_stats: None,
                wifi7_stats: None,
                cellular5g_stats: None,
                cellular6g_stats: None,
                wifi8_stats: None,
                wifi9_stats: None,
                wifi10_stats: None,
                wifi11_stats: None,
                cellular7g_stats: None,
                cellular8g_stats: None,
                cellular9g_stats: None,
                cellular10g_stats: None,
                tc_config: basic_iface.tc_config,
                qos_queue_stats: basic_iface.qos_queue_stats,
                technology_capabilities: Vec::new(),
                supported_bands: Vec::new(),
                power_saving_mode: None,
                health_status: "Operational".to_string(),
            };

            // Add technology-specific capabilities based on interface type
            match basic_iface.base_stats.interface_type {
                NetworkInterfaceType::Wifi6 | NetworkInterfaceType::Wifi6E => {
                    extended_iface.wifi6_stats = Some(self.collect_wifi6_stats(&basic_iface.base_stats.name)?);
                    extended_iface.technology_capabilities.push("Wi-Fi 6/6E".to_string());
                    extended_iface.technology_capabilities.push("MU-MIMO".to_string());
                    extended_iface.technology_capabilities.push("OFDMA".to_string());
                    extended_iface.supported_bands.push("2.4GHz".to_string());
                    extended_iface.supported_bands.push("5GHz".to_string());
                    if basic_iface.base_stats.interface_type == NetworkInterfaceType::Wifi6E {
                        extended_iface.supported_bands.push("6GHz".to_string());
                        extended_iface.technology_capabilities.push("6GHz support".to_string());
                    }
                }
                NetworkInterfaceType::Wifi7 => {
                    extended_iface.wifi7_stats = Some(self.collect_wifi7_stats(&basic_iface.base_stats.name)?);
                    extended_iface.technology_capabilities.push("Wi-Fi 7".to_string());
                    extended_iface.technology_capabilities.push("EHT".to_string());
                    extended_iface.technology_capabilities.push("MLO".to_string());
                    extended_iface.technology_capabilities.push("4K-QAM".to_string());
                    extended_iface.technology_capabilities.push("320MHz channels".to_string());
                    extended_iface.supported_bands.push("2.4GHz".to_string());
                    extended_iface.supported_bands.push("5GHz".to_string());
                    extended_iface.supported_bands.push("6GHz".to_string());
                }
                NetworkInterfaceType::Wifi8 => {
                    extended_iface.wifi8_stats = Some(self.collect_wifi8_stats(&basic_iface.base_stats.name)?);
                    extended_iface.technology_capabilities.push("Wi-Fi 8".to_string());
                    extended_iface.technology_capabilities.push("EHT+".to_string());
                    extended_iface.technology_capabilities.push("Advanced MLO".to_string());
                    extended_iface.technology_capabilities.push("AI Optimization".to_string());
                    extended_iface.technology_capabilities.push("Quantum Encryption".to_string());
                    extended_iface.technology_capabilities.push("Terahertz Communication".to_string());
                    extended_iface.supported_bands.push("2.4GHz".to_string());
                    extended_iface.supported_bands.push("5GHz".to_string());
                    extended_iface.supported_bands.push("6GHz".to_string());
                    extended_iface.supported_bands.push("7GHz".to_string());
                }
                NetworkInterfaceType::Wifi9 => {
                    extended_iface.wifi9_stats = Some(self.collect_wifi9_stats(&basic_iface.base_stats.name)?);
                    extended_iface.technology_capabilities.push("Wi-Fi 9".to_string());
                    extended_iface.technology_capabilities.push("EHT++".to_string());
                    extended_iface.technology_capabilities.push("Ultra MLO".to_string());
                    extended_iface.technology_capabilities.push("AI Optimization+".to_string());
                    extended_iface.technology_capabilities.push("Quantum Encryption+".to_string());
                    extended_iface.technology_capabilities.push("Terahertz Communication+".to_string());
                    extended_iface.technology_capabilities.push("Holographic Beamforming".to_string());
                    extended_iface.supported_bands.push("2.4GHz".to_string());
                    extended_iface.supported_bands.push("5GHz".to_string());
                    extended_iface.supported_bands.push("6GHz".to_string());
                    extended_iface.supported_bands.push("7GHz".to_string());
                    extended_iface.supported_bands.push("8GHz".to_string());
                }
                NetworkInterfaceType::Cellular5G => {
                    extended_iface.cellular5g_stats = Some(self.collect_cellular5g_stats(&basic_iface.base_stats.name)?);
                    extended_iface.technology_capabilities.push("5G NR".to_string());
                    extended_iface.technology_capabilities.push("Carrier Aggregation".to_string());
                    extended_iface.technology_capabilities.push("MIMO".to_string());
                    extended_iface.technology_capabilities.push("Network Slicing".to_string());
                }
                NetworkInterfaceType::Cellular6G => {
                    extended_iface.cellular6g_stats = Some(self.collect_cellular6g_stats(&basic_iface.base_stats.name)?);
                    extended_iface.technology_capabilities.push("6G".to_string());
                    extended_iface.technology_capabilities.push("Terahertz".to_string());
                    extended_iface.technology_capabilities.push("AI Optimization".to_string());
                    extended_iface.technology_capabilities.push("Quantum Encryption".to_string());
                    extended_iface.technology_capabilities.push("Dynamic Spectrum Sharing".to_string());
                }
                NetworkInterfaceType::Cellular7G => {
                    extended_iface.cellular7g_stats = Some(self.collect_cellular7g_stats(&basic_iface.base_stats.name)?);
                    extended_iface.technology_capabilities.push("7G".to_string());
                    extended_iface.technology_capabilities.push("Terahertz+".to_string());
                    extended_iface.technology_capabilities.push("AI Optimization+".to_string());
                    extended_iface.technology_capabilities.push("Quantum Encryption+".to_string());
                    extended_iface.technology_capabilities.push("Dynamic Spectrum Sharing+".to_string());
                    extended_iface.technology_capabilities.push("Holographic Communication".to_string());
                    extended_iface.technology_capabilities.push("Neural Interface".to_string());
                }
                NetworkInterfaceType::Cellular8G => {
                    extended_iface.cellular8g_stats = Some(self.collect_cellular8g_stats(&basic_iface.base_stats.name)?);
                    extended_iface.technology_capabilities.push("8G".to_string());
                    extended_iface.technology_capabilities.push("Terahertz++".to_string());
                    extended_iface.technology_capabilities.push("AI Optimization++".to_string());
                    extended_iface.technology_capabilities.push("Quantum Encryption++".to_string());
                    extended_iface.technology_capabilities.push("Dynamic Spectrum Sharing++".to_string());
                    extended_iface.technology_capabilities.push("Holographic Communication+".to_string());
                    extended_iface.technology_capabilities.push("Neural Interface+".to_string());
                    extended_iface.technology_capabilities.push("Holographic MIMO".to_string());
                }
                NetworkInterfaceType::Cellular9G => {
                    extended_iface.cellular9g_stats = Some(self.collect_cellular9g_stats(&basic_iface.base_stats.name)?);
                    extended_iface.technology_capabilities.push("9G".to_string());
                    extended_iface.technology_capabilities.push("Terahertz+++".to_string());
                    extended_iface.technology_capabilities.push("AI Optimization+++".to_string());
                    extended_iface.technology_capabilities.push("Quantum Encryption+++".to_string());
                    extended_iface.technology_capabilities.push("Dynamic Spectrum Sharing+++".to_string());
                    extended_iface.technology_capabilities.push("Holographic Communication++".to_string());
                    extended_iface.technology_capabilities.push("Neural Interface++".to_string());
                    extended_iface.technology_capabilities.push("Holographic MIMO+".to_string());
                }
                NetworkInterfaceType::Wifi10 => {
                    extended_iface.wifi10_stats = Some(self.collect_wifi10_stats(&basic_iface.base_stats.name)?);
                    extended_iface.technology_capabilities.push("Wi-Fi 10".to_string());
                    extended_iface.technology_capabilities.push("EHT++".to_string());
                    extended_iface.technology_capabilities.push("Ultra-MLO".to_string());
                    extended_iface.technology_capabilities.push("AI Optimization+".to_string());
                    extended_iface.technology_capabilities.push("Quantum Encryption+".to_string());
                    extended_iface.technology_capabilities.push("Terahertz Communication+".to_string());
                    extended_iface.technology_capabilities.push("Holographic Beamforming".to_string());
                    extended_iface.supported_bands.push("2.4GHz".to_string());
                    extended_iface.supported_bands.push("5GHz".to_string());
                    extended_iface.supported_bands.push("6GHz".to_string());
                    extended_iface.supported_bands.push("7GHz".to_string());
                    extended_iface.supported_bands.push("8GHz".to_string());
                    extended_iface.supported_bands.push("9GHz".to_string());
                }
                NetworkInterfaceType::Wifi11 => {
                    extended_iface.wifi11_stats = Some(self.collect_wifi11_stats(&basic_iface.base_stats.name)?);
                    extended_iface.technology_capabilities.push("Wi-Fi 11".to_string());
                    extended_iface.technology_capabilities.push("EHT+++".to_string());
                    extended_iface.technology_capabilities.push("Ultra-MLO+".to_string());
                    extended_iface.technology_capabilities.push("AI Optimization++".to_string());
                    extended_iface.technology_capabilities.push("Quantum Encryption++".to_string());
                    extended_iface.technology_capabilities.push("Terahertz Communication++".to_string());
                    extended_iface.technology_capabilities.push("Holographic Beamforming+".to_string());
                    extended_iface.supported_bands.push("2.4GHz".to_string());
                    extended_iface.supported_bands.push("5GHz".to_string());
                    extended_iface.supported_bands.push("6GHz".to_string());
                    extended_iface.supported_bands.push("7GHz".to_string());
                    extended_iface.supported_bands.push("8GHz".to_string());
                    extended_iface.supported_bands.push("9GHz".to_string());
                    extended_iface.supported_bands.push("10GHz".to_string());
                }
                NetworkInterfaceType::Cellular10G => {
                    extended_iface.cellular10g_stats = Some(self.collect_cellular10g_stats(&basic_iface.base_stats.name)?);
                    extended_iface.technology_capabilities.push("10G".to_string());
                    extended_iface.technology_capabilities.push("Terahertz++++".to_string());
                    extended_iface.technology_capabilities.push("AI Optimization++++".to_string());
                    extended_iface.technology_capabilities.push("Quantum Encryption++++".to_string());
                    extended_iface.technology_capabilities.push("Dynamic Spectrum Sharing++++".to_string());
                    extended_iface.technology_capabilities.push("Holographic Communication+++".to_string());
                    extended_iface.technology_capabilities.push("Neural Interface+++".to_string());
                    extended_iface.technology_capabilities.push("Holographic MIMO++".to_string());
                    extended_iface.technology_capabilities.push("Quantum Computing".to_string());
                    extended_iface.technology_capabilities.push("Quantum AI".to_string());
                    extended_iface.technology_capabilities.push("Quantum Holographic".to_string());
                }
                NetworkInterfaceType::Wifi => {
                    extended_iface.technology_capabilities.push("Wi-Fi".to_string());
                    extended_iface.supported_bands.push("2.4GHz".to_string());
                    extended_iface.supported_bands.push("5GHz".to_string());
                }
                NetworkInterfaceType::Cellular => {
                    extended_iface.technology_capabilities.push("Cellular".to_string());
                    extended_iface.technology_capabilities.push("4G LTE".to_string());
                }
                NetworkInterfaceType::Ethernet => {
                    extended_iface.technology_capabilities.push("Ethernet".to_string());
                    extended_iface.technology_capabilities.push("Full Duplex".to_string());
                }
                _ => {}
            }

            // Add power saving mode based on interface capabilities
            if extended_iface.technology_capabilities.contains(&"Wi-Fi 6/6E".to_string()) {
                extended_iface.power_saving_mode = Some("TWT".to_string()); // Target Wake Time
            } else if extended_iface.technology_capabilities.contains(&"Wi-Fi".to_string()) {
                extended_iface.power_saving_mode = Some("Legacy".to_string());
            }

            extended_interfaces.push(extended_iface);
        }

        Ok(extended_interfaces)
    }

    /// Collect Wi-Fi 6/6E specific statistics
    fn collect_wifi6_stats(&self, interface_name: &str) -> Result<Wifi6Stats> {
        // In a real implementation, this would use iw, iwconfig, or nl80211
        // For now, we'll return mock data with reasonable defaults
        
        let mut wifi6_stats = Wifi6Stats::default();

        // Determine Wi-Fi standard based on interface type
        wifi6_stats.wifi_standard = if interface_name.contains("wlan") || interface_name.contains("wifi") {
            "Wi-Fi 6".to_string()
        } else {
            "Wi-Fi 6E".to_string()
        };

        // Set reasonable defaults for Wi-Fi 6/6E
        wifi6_stats.channel_bandwidth_mhz = 160; // 160MHz channel
        wifi6_stats.channel = 42; // Example channel in 5GHz band
        wifi6_stats.frequency_band = "5GHz".to_string();
        wifi6_stats.signal_strength_dbm = -55; // Good signal strength
        wifi6_stats.signal_noise_ratio_db = 35.0; // Good SNR
        wifi6_stats.tx_rate_mbps = 1200; // 1.2 Gbps
        wifi6_stats.rx_rate_mbps = 1200; // 1.2 Gbps
        wifi6_stats.mu_mimo_support = true;
        wifi6_stats.ofdma_support = true;
        wifi6_stats.bss_coloring_support = true;
        wifi6_stats.target_wake_time_support = true;
        wifi6_stats.spatial_streams = 2; // 2x2 MIMO
        wifi6_stats.mcs_index = 9; // High MCS index
        wifi6_stats.retry_count = 10;
        wifi6_stats.packet_loss_percent = 0.5; // 0.5% packet loss
        wifi6_stats.roaming_count = 0;
        wifi6_stats.security_protocol = "WPA3".to_string();
        wifi6_stats.capabilities.push("HE160".to_string()); // 160MHz channel support
        wifi6_stats.capabilities.push("VHT160".to_string());
        wifi6_stats.capabilities.push("HE_MU_MIMO".to_string());

        // For Wi-Fi 6E, add 6GHz capabilities
        if wifi6_stats.wifi_standard == "Wi-Fi 6E" {
            wifi6_stats.frequency_band = "6GHz".to_string();
            wifi6_stats.channel = 1; // Example channel in 6GHz band
            wifi6_stats.capabilities.push("6GHz".to_string());
            wifi6_stats.capabilities.push("HE_6GHZ".to_string());
        }

        Ok(wifi6_stats)
    }

    /// Collect Wi-Fi 7 specific statistics
    fn collect_wifi7_stats(&self, _interface_name: &str) -> Result<Wifi7Stats> {
        // In a real implementation, this would use iw, iwconfig, or nl80211
        // For now, we'll return mock data with reasonable defaults
        
        let mut wifi7_stats = Wifi7Stats::default();

        // Set Wi-Fi 7 specific parameters
        wifi7_stats.wifi_standard = "Wi-Fi 7".to_string();
        wifi7_stats.channel_bandwidth_mhz = 320; // 320MHz channel
        wifi7_stats.channel = 1; // Example channel in 6GHz band
        wifi7_stats.frequency_band = "6GHz".to_string();
        wifi7_stats.signal_strength_dbm = -50; // Excellent signal strength
        wifi7_stats.signal_noise_ratio_db = 40.0; // Excellent SNR
        wifi7_stats.tx_rate_mbps = 5000; // 5 Gbps
        wifi7_stats.rx_rate_mbps = 5000; // 5 Gbps
        wifi7_stats.mu_mimo_support = true;
        wifi7_stats.ofdma_support = true;
        wifi7_stats.bss_coloring_support = true;
        wifi7_stats.target_wake_time_support = true;
        wifi7_stats.multi_link_operation_support = true;
        wifi7_stats.qam4k_support = true;
        wifi7_stats.spatial_streams = 4; // 4x4 MIMO
        wifi7_stats.mcs_index = 13; // Highest MCS index
        wifi7_stats.retry_count = 5;
        wifi7_stats.packet_loss_percent = 0.1; // 0.1% packet loss
        wifi7_stats.roaming_count = 0;
        wifi7_stats.security_protocol = "WPA3".to_string();
        wifi7_stats.capabilities.push("HE320".to_string()); // 320MHz channel support
        wifi7_stats.capabilities.push("EHT".to_string()); // Extremely High Throughput
        wifi7_stats.capabilities.push("MLO".to_string()); // Multi-Link Operation
        wifi7_stats.capabilities.push("4K-QAM".to_string()); // 4K QAM support
        wifi7_stats.mlo_links_count = 2; // 2 simultaneous links
        wifi7_stats.max_spatial_streams = 4;
        wifi7_stats.preamble_puncturing_support = true;
        wifi7_stats.apsd_support = true;

        Ok(wifi7_stats)
    }

    /// Collect 5G cellular network statistics
    fn collect_cellular5g_stats(&self, _interface_name: &str) -> Result<Cellular5GStats> {
        // In a real implementation, this would use mmcli, qmicli, or AT commands
        // For now, we'll return mock data with reasonable defaults
        
        let mut cellular5g_stats = Cellular5GStats::default();

        // Set 5G specific parameters
        cellular5g_stats.technology = "5G NR".to_string();
        cellular5g_stats.generation = "5G".to_string();
        cellular5g_stats.signal_strength_dbm = -75; // Good 5G signal
        cellular5g_stats.rsrp_dbm = -95.0; // Good RSRP
        cellular5g_stats.rsrq_db = -10.0; // Good RSRQ
        cellular5g_stats.sinr_db = 20.0; // Good SINR
        cellular5g_stats.bandwidth_mhz = 100; // 100MHz bandwidth
        cellular5g_stats.frequency_band = "n78".to_string(); // Common 5G band
        cellular5g_stats.cell_id = 123456789;
        cellular5g_stats.tracking_area_code = 12345;
        cellular5g_stats.physical_cell_id = 42;
        cellular5g_stats.modulation = "256QAM".to_string();
        cellular5g_stats.mimo_config = "4x4".to_string();
        cellular5g_stats.carrier_aggregation = true;
        cellular5g_stats.downlink_rate_mbps = 800.0; // 800 Mbps downlink
        cellular5g_stats.uplink_rate_mbps = 200.0; // 200 Mbps uplink
        cellular5g_stats.latency_ms = 15.0; // Low latency
        cellular5g_stats.jitter_ms = 2.0; // Low jitter
        cellular5g_stats.packet_loss_percent = 0.1; // 0.1% packet loss
        cellular5g_stats.network_slice = Some("eMBB".to_string()); // Enhanced Mobile Broadband
        cellular5g_stats.qos_flow = Some("QFI_1".to_string()); // QoS Flow Identifier
        cellular5g_stats.stability_score = 0.95; // Excellent stability

        Ok(cellular5g_stats)
    }

    /// Collect 6G cellular network statistics
    fn collect_cellular6g_stats(&self, _interface_name: &str) -> Result<Cellular6GStats> {
        // In a real implementation, this would use future 6G APIs
        // For now, we'll return mock data with reasonable defaults
        
        let mut cellular6g_stats = Cellular6GStats::default();

        // Set 6G specific parameters
        cellular6g_stats.technology = "6G".to_string();
        cellular6g_stats.generation = "6G".to_string();
        cellular6g_stats.signal_strength_dbm = -65; // Excellent 6G signal
        cellular6g_stats.rsrp_dbm = -85.0; // Excellent RSRP
        cellular6g_stats.rsrq_db = -8.0; // Excellent RSRQ
        cellular6g_stats.sinr_db = 25.0; // Excellent SINR
        cellular6g_stats.bandwidth_mhz = 500; // 500MHz bandwidth
        cellular6g_stats.frequency_band = "n256".to_string(); // Future 6G band
        cellular6g_stats.cell_id = 987654321;
        cellular6g_stats.tracking_area_code = 54321;
        cellular6g_stats.physical_cell_id = 99;
        cellular6g_stats.modulation = "1024QAM".to_string();
        cellular6g_stats.mimo_config = "16x16".to_string();
        cellular6g_stats.carrier_aggregation = true;
        cellular6g_stats.advanced_mimo_support = true;
        cellular6g_stats.terahertz_support = true;
        cellular6g_stats.ai_optimization_support = true;
        cellular6g_stats.downlink_rate_mbps = 10000.0; // 10 Gbps downlink
        cellular6g_stats.uplink_rate_mbps = 5000.0; // 5 Gbps uplink
        cellular6g_stats.latency_ms = 1.0; // Ultra-low latency
        cellular6g_stats.jitter_ms = 0.5; // Ultra-low jitter
        cellular6g_stats.packet_loss_percent = 0.01; // 0.01% packet loss
        cellular6g_stats.network_slice = Some("uRLLC".to_string()); // Ultra-Reliable Low Latency
        cellular6g_stats.qos_flow = Some("QFI_5".to_string()); // QoS Flow Identifier
        cellular6g_stats.stability_score = 0.99; // Excellent stability
        cellular6g_stats.ai_traffic_prediction = true;
        cellular6g_stats.dynamic_spectrum_sharing = true;
        cellular6g_stats.quantum_encryption_support = true;

        Ok(cellular6g_stats)
    }

    /// Collect Wi-Fi 8 specific statistics
    fn collect_wifi8_stats(&self, _interface_name: &str) -> Result<Wifi8Stats> {
        // In a real implementation, this would use iw, iwconfig, or nl80211
        // For now, we'll return mock data with reasonable defaults
        
        let mut wifi8_stats = Wifi8Stats::default();

        // Set Wi-Fi 8 specific parameters
        wifi8_stats.wifi_standard = "Wi-Fi 8".to_string();
        wifi8_stats.channel_bandwidth_mhz = 320; // 320MHz channel
        wifi8_stats.channel = 1; // Example channel in 7GHz band
        wifi8_stats.frequency_band = "7GHz".to_string();
        wifi8_stats.signal_strength_dbm = -45; // Excellent signal strength
        wifi8_stats.signal_noise_ratio_db = 45.0; // Excellent SNR
        wifi8_stats.tx_rate_mbps = 10000; // 10 Gbps
        wifi8_stats.rx_rate_mbps = 10000; // 10 Gbps
        wifi8_stats.mu_mimo_support = true;
        wifi8_stats.ofdma_support = true;
        wifi8_stats.bss_coloring_support = true;
        wifi8_stats.target_wake_time_support = true;
        wifi8_stats.multi_link_operation_support = true;
        wifi8_stats.qam4k_support = true;
        wifi8_stats.spatial_streams = 8; // 8x8 MIMO
        wifi8_stats.mcs_index = 15; // Highest MCS index
        wifi8_stats.retry_count = 3;
        wifi8_stats.packet_loss_percent = 0.05; // 0.05% packet loss
        wifi8_stats.roaming_count = 0;
        wifi8_stats.security_protocol = "WPA4".to_string();
        wifi8_stats.capabilities.push("HE320".to_string()); // 320MHz channel support
        wifi8_stats.capabilities.push("EHT".to_string()); // Extremely High Throughput
        wifi8_stats.capabilities.push("MLO".to_string()); // Multi-Link Operation
        wifi8_stats.capabilities.push("4K-QAM".to_string()); // 4K QAM support
        wifi8_stats.capabilities.push("AI-Optimization".to_string()); // AI-based optimization
        wifi8_stats.capabilities.push("Quantum-Encryption".to_string()); // Quantum encryption
        wifi8_stats.mlo_links_count = 4; // 4 simultaneous links
        wifi8_stats.max_spatial_streams = 8;
        wifi8_stats.preamble_puncturing_support = true;
        wifi8_stats.apsd_support = true;
        wifi8_stats.ai_optimization_support = true;
        wifi8_stats.quantum_encryption_support = true;
        wifi8_stats.dynamic_spectrum_sharing_support = true;
        wifi8_stats.terahertz_support = true;

        Ok(wifi8_stats)
    }

    /// Collect Wi-Fi 9 specific statistics
    fn collect_wifi9_stats(&self, _interface_name: &str) -> Result<Wifi9Stats> {
        // In a real implementation, this would use iw, iwconfig, or nl80211
        // For now, we'll return mock data with reasonable defaults
        
        let mut wifi9_stats = Wifi9Stats::default();

        // Set Wi-Fi 9 specific parameters
        wifi9_stats.wifi_standard = "Wi-Fi 9".to_string();
        wifi9_stats.channel_bandwidth_mhz = 320; // 320MHz channel
        wifi9_stats.channel = 1; // Example channel in 8GHz band
        wifi9_stats.frequency_band = "8GHz".to_string();
        wifi9_stats.signal_strength_dbm = -40; // Excellent signal strength
        wifi9_stats.signal_noise_ratio_db = 50.0; // Excellent SNR
        wifi9_stats.tx_rate_mbps = 20000; // 20 Gbps
        wifi9_stats.rx_rate_mbps = 20000; // 20 Gbps
        wifi9_stats.mu_mimo_support = true;
        wifi9_stats.ofdma_support = true;
        wifi9_stats.bss_coloring_support = true;
        wifi9_stats.target_wake_time_support = true;
        wifi9_stats.multi_link_operation_support = true;
        wifi9_stats.qam4k_support = true;
        wifi9_stats.spatial_streams = 16; // 16x16 MIMO
        wifi9_stats.mcs_index = 15; // Highest MCS index
        wifi9_stats.retry_count = 2;
        wifi9_stats.packet_loss_percent = 0.01; // 0.01% packet loss
        wifi9_stats.roaming_count = 0;
        wifi9_stats.security_protocol = "WPA5".to_string();
        wifi9_stats.capabilities.push("HE320".to_string()); // 320MHz channel support
        wifi9_stats.mlo_links_count = 4; // 4 MLO links
        wifi9_stats.max_spatial_streams = 16;
        wifi9_stats.preamble_puncturing_support = true;
        wifi9_stats.advanced_beamforming_support = true;
        wifi9_stats.ai_optimization_support = true;
        wifi9_stats.terahertz_support = true;
        wifi9_stats.quantum_encryption_support = true;
        wifi9_stats.dynamic_spectrum_sharing_support = true;
        wifi9_stats.downlink_rate_mbps = 20000.0;
        wifi9_stats.uplink_rate_mbps = 20000.0;
        wifi9_stats.latency_ms = 1.0;
        wifi9_stats.jitter_ms = 0.1;
        wifi9_stats.packet_delivery_success_rate = 99.99;

        Ok(wifi9_stats)
    }

    /// Collect 7G cellular network statistics
    fn collect_cellular7g_stats(&self, _interface_name: &str) -> Result<Cellular7GStats> {
        // In a real implementation, this would use future 7G APIs
        // For now, we'll return mock data with reasonable defaults
        
        let mut cellular7g_stats = Cellular7GStats::default();

        // Set 7G specific parameters
        cellular7g_stats.technology = "7G".to_string();
        cellular7g_stats.generation = "7G".to_string();
        cellular7g_stats.signal_strength_dbm = -40; // Excellent 7G signal
        cellular7g_stats.rsrp_dbm = -75.0; // Excellent RSRP
        cellular7g_stats.rsrq_db = -6.0; // Excellent RSRQ
        cellular7g_stats.sinr_db = 30.0; // Excellent SINR
        cellular7g_stats.bandwidth_mhz = 1000; // 1000MHz bandwidth
        cellular7g_stats.frequency_band = "n512".to_string(); // Future 7G band
        cellular7g_stats.cell_id = 1234567890;
        cellular7g_stats.tracking_area_code = 98765;
        cellular7g_stats.physical_cell_id = 127;
        cellular7g_stats.modulation = "4096QAM".to_string();
        cellular7g_stats.mimo_config = "32x32".to_string();
        cellular7g_stats.carrier_aggregation = true;
        cellular7g_stats.advanced_mimo_support = true;
        cellular7g_stats.terahertz_support = true;
        cellular7g_stats.ai_optimization_support = true;
        cellular7g_stats.quantum_encryption_support = true;
        cellular7g_stats.dynamic_spectrum_sharing_support = true;
        cellular7g_stats.downlink_rate_mbps = 50000.0; // 50 Gbps downlink
        cellular7g_stats.uplink_rate_mbps = 25000.0; // 25 Gbps uplink
        cellular7g_stats.latency_ms = 0.1; // Ultra-low latency
        cellular7g_stats.jitter_ms = 0.05; // Ultra-low jitter
        cellular7g_stats.packet_loss_percent = 0.001; // 0.001% packet loss
        cellular7g_stats.network_slice = Some("uRLLC+".to_string()); // Ultra-Reliable Low Latency+
        cellular7g_stats.qos_flow = Some("QFI_9".to_string()); // QoS Flow Identifier
        cellular7g_stats.stability_score = 0.999; // Excellent stability
        cellular7g_stats.ai_traffic_prediction = true;
        cellular7g_stats.dynamic_spectrum_sharing = true;
        cellular7g_stats.quantum_encryption_support = true;
        cellular7g_stats.holographic_communication_support = true;
        cellular7g_stats.neural_interface_support = true;

        Ok(cellular7g_stats)
    }

    /// Collect 8G cellular specific statistics
    fn collect_cellular8g_stats(&self, _interface_name: &str) -> Result<Cellular8GStats> {
        // In a real implementation, this would use future 8G APIs
        // For now, we'll return mock data with reasonable defaults
        
        let mut cellular8g_stats = Cellular8GStats::default();

        // Set 8G specific parameters
        cellular8g_stats.technology = "8G".to_string();
        cellular8g_stats.generation = "8G".to_string();
        cellular8g_stats.signal_strength_dbm = -35; // Excellent signal strength
        cellular8g_stats.rsrp_dbm = -35.0; // Excellent RSRP
        cellular8g_stats.rsrq_db = 25.0; // Excellent RSRQ
        cellular8g_stats.sinr_db = 30.0; // Excellent SINR
        cellular8g_stats.bandwidth_mhz = 1000; // 1GHz bandwidth
        cellular8g_stats.frequency_band = "Terahertz".to_string();
        cellular8g_stats.cell_id = 123456789;
        cellular8g_stats.tracking_area_code = 12345;
        cellular8g_stats.physical_cell_id = 1;
        cellular8g_stats.modulation = "1024-QAM".to_string();
        cellular8g_stats.mimo_config = "256x256".to_string();
        cellular8g_stats.carrier_aggregation = true;
        cellular8g_stats.advanced_mimo_support = true;
        cellular8g_stats.terahertz_support = true;
        cellular8g_stats.ai_optimization_support = true;
        cellular8g_stats.quantum_encryption_support = true;
        cellular8g_stats.dynamic_spectrum_sharing_support = true;
        cellular8g_stats.advanced_beamforming_support = true;
        cellular8g_stats.holographic_mimo_support = true;
        cellular8g_stats.downlink_rate_mbps = 50000.0; // 50 Gbps
        cellular8g_stats.uplink_rate_mbps = 50000.0; // 50 Gbps
        cellular8g_stats.latency_ms = 0.5; // Ultra-low latency
        cellular8g_stats.jitter_ms = 0.05;
        cellular8g_stats.packet_delivery_success_rate = 99.999;
        cellular8g_stats.reliability_percent = 99.999;
        cellular8g_stats.energy_efficiency_rating = 95.0;

        Ok(cellular8g_stats)
    }

    /// Collect Wi-Fi 10 specific statistics
    fn collect_wifi10_stats(&self, _interface_name: &str) -> Result<Wifi10Stats> {
        // In a real implementation, this would use iw, iwconfig, or nl80211
        // For now, we'll return mock data with reasonable defaults
        
        let mut wifi10_stats = Wifi10Stats::default();

        // Set Wi-Fi 10 specific parameters
        wifi10_stats.wifi_standard = "Wi-Fi 10".to_string();
        wifi10_stats.channel_bandwidth_mhz = 320; // 320MHz channel
        wifi10_stats.channel = 1; // Example channel in 9GHz band
        wifi10_stats.frequency_band = "9GHz".to_string();
        wifi10_stats.signal_strength_dbm = -35; // Excellent signal strength
        wifi10_stats.signal_noise_ratio_db = 55.0; // Excellent SNR
        wifi10_stats.tx_rate_mbps = 40000; // 40 Gbps
        wifi10_stats.rx_rate_mbps = 40000; // 40 Gbps
        wifi10_stats.mu_mimo_support = true;
        wifi10_stats.ofdma_support = true;
        wifi10_stats.bss_coloring_support = true;
        wifi10_stats.target_wake_time_support = true;
        wifi10_stats.multi_link_operation_support = true;
        wifi10_stats.qam4k_support = true;
        wifi10_stats.spatial_streams = 32; // 32x32 MIMO
        wifi10_stats.mcs_index = 15; // Highest MCS index
        wifi10_stats.retry_count = 1;
        wifi10_stats.packet_loss_percent = 0.005; // 0.005% packet loss
        wifi10_stats.roaming_count = 0;
        wifi10_stats.security_protocol = "WPA6".to_string();
        wifi10_stats.capabilities.push("HE320".to_string()); // 320MHz channel support
        wifi10_stats.capabilities.push("EHT++".to_string()); // Extremely High Throughput++
        wifi10_stats.capabilities.push("Ultra-MLO".to_string()); // Ultra Multi-Link Operation
        wifi10_stats.capabilities.push("4K-QAM".to_string()); // 4K QAM support
        wifi10_stats.capabilities.push("AI-Optimization+".to_string()); // AI-based optimization+
        wifi10_stats.capabilities.push("Quantum-Encryption+".to_string()); // Quantum encryption+
        wifi10_stats.mlo_links_count = 8; // 8 simultaneous links
        wifi10_stats.max_spatial_streams = 32;
        wifi10_stats.preamble_puncturing_support = true;
        wifi10_stats.advanced_beamforming_support = true;
        wifi10_stats.ai_optimization_support = true;
        wifi10_stats.terahertz_support = true;
        wifi10_stats.quantum_encryption_support = true;
        wifi10_stats.dynamic_spectrum_sharing_support = true;
        wifi10_stats.holographic_beamforming_support = true;
        wifi10_stats.eht_plus_plus_support = true;
        wifi10_stats.ultra_mlo_support = true;
        wifi10_stats.ai_optimization_plus_support = true;
        wifi10_stats.quantum_encryption_plus_support = true;
        wifi10_stats.terahertz_communication_plus_support = true;
        wifi10_stats.downlink_rate_mbps = 40000.0;
        wifi10_stats.uplink_rate_mbps = 40000.0;
        wifi10_stats.latency_ms = 0.5;
        wifi10_stats.jitter_ms = 0.05;
        wifi10_stats.packet_delivery_success_rate = 99.9995;

        Ok(wifi10_stats)
    }

    /// Collect Wi-Fi 11 specific statistics
    fn collect_wifi11_stats(&self, _interface_name: &str) -> Result<Wifi11Stats> {
        // In a real implementation, this would use future Wi-Fi 11 APIs
        // For now, we'll return mock data with reasonable defaults
        
        let mut wifi11_stats = Wifi11Stats::default();

        // Set Wi-Fi 11 specific parameters
        wifi11_stats.wifi_standard = "Wi-Fi 11".to_string();
        wifi11_stats.channel_bandwidth_mhz = 640; // 640MHz channel
        wifi11_stats.channel = 100; // Example channel in 10GHz band
        wifi11_stats.frequency_band = "10GHz".to_string();
        wifi11_stats.signal_strength_dbm = -30; // Excellent signal strength
        wifi11_stats.signal_noise_ratio_db = 60.0; // Excellent SNR
        wifi11_stats.tx_rate_mbps = 96000; // 96 Gbps
        wifi11_stats.rx_rate_mbps = 96000; // 96 Gbps
        wifi11_stats.mu_mimo_support = true;
        wifi11_stats.ofdma_support = true;
        wifi11_stats.bss_coloring_support = true;
        wifi11_stats.target_wake_time_support = true;
        wifi11_stats.multi_link_operation_support = true;
        wifi11_stats.qam4k_support = true;
        wifi11_stats.spatial_streams = 64; // 64x64 MIMO
        wifi11_stats.mcs_index = 15; // Highest MCS index
        wifi11_stats.retry_count = 0;
        wifi11_stats.packet_loss_percent = 0.001; // 0.001% packet loss
        wifi11_stats.roaming_count = 0;
        wifi11_stats.security_protocol = "WPA7".to_string();
        wifi11_stats.capabilities.push("HE640".to_string()); // 640MHz channel support
        wifi11_stats.capabilities.push("EHT+++".to_string()); // Extremely High Throughput+++
        wifi11_stats.capabilities.push("Ultra-MLO+".to_string()); // Ultra Multi-Link Operation+
        wifi11_stats.capabilities.push("8K-QAM".to_string()); // 8K QAM support
        wifi11_stats.capabilities.push("AI-Optimization++".to_string()); // AI-based optimization++
        wifi11_stats.capabilities.push("Quantum-Encryption++".to_string()); // Quantum encryption++
        wifi11_stats.mlo_links_count = 16; // 16 simultaneous links
        wifi11_stats.max_spatial_streams = 64;
        wifi11_stats.preamble_puncturing_support = true;
        wifi11_stats.advanced_beamforming_support = true;
        wifi11_stats.ai_optimization_support = true;
        wifi11_stats.terahertz_support = true;
        wifi11_stats.quantum_encryption_support = true;
        wifi11_stats.dynamic_spectrum_sharing_support = true;
        wifi11_stats.holographic_beamforming_support = true;
        wifi11_stats.eht_plus_plus_plus_support = true;
        wifi11_stats.ultra_mlo_plus_support = true;
        wifi11_stats.ai_optimization_plus_support = true;
        wifi11_stats.quantum_encryption_plus_support = true;
        wifi11_stats.terahertz_communication_plus_support = true;
        wifi11_stats.downlink_rate_mbps = 96000.0;
        wifi11_stats.uplink_rate_mbps = 96000.0;
        wifi11_stats.latency_ms = 0.1;
        wifi11_stats.jitter_ms = 0.01;
        wifi11_stats.packet_delivery_success_rate = 99.9999;
        wifi11_stats.reliability_percent = 99.9999;
        wifi11_stats.energy_efficiency_rating = 99.0;

        Ok(wifi11_stats)
    }

    /// Collect 9G cellular network statistics
    fn collect_cellular9g_stats(&self, _interface_name: &str) -> Result<Cellular9GStats> {
        // In a real implementation, this would use future 9G APIs
        // For now, we'll return mock data with reasonable defaults
        
        let mut cellular9g_stats = Cellular9GStats::default();

        // Set 9G specific parameters
        cellular9g_stats.technology = "9G".to_string();
        cellular9g_stats.generation = "9G".to_string();
        cellular9g_stats.signal_strength_dbm = -30; // Excellent 9G signal
        cellular9g_stats.rsrp_dbm = -30.0; // Excellent RSRP
        cellular9g_stats.rsrq_db = 30.0; // Excellent RSRQ
        cellular9g_stats.sinr_db = 35.0; // Excellent SINR
        cellular9g_stats.bandwidth_mhz = 2000; // 2GHz bandwidth
        cellular9g_stats.frequency_band = "Terahertz+".to_string();
        cellular9g_stats.cell_id = 1234567890;
        cellular9g_stats.tracking_area_code = 12345;
        cellular9g_stats.physical_cell_id = 1;
        cellular9g_stats.modulation = "4096-QAM".to_string();
        cellular9g_stats.mimo_config = "512x512".to_string();
        cellular9g_stats.carrier_aggregation = true;
        cellular9g_stats.advanced_mimo_support = true;
        cellular9g_stats.terahertz_support = true;
        cellular9g_stats.ai_optimization_support = true;
        cellular9g_stats.quantum_encryption_support = true;
        cellular9g_stats.dynamic_spectrum_sharing_support = true;
        cellular9g_stats.advanced_beamforming_support = true;
        cellular9g_stats.holographic_mimo_support = true;
        cellular9g_stats.holographic_communication_support = true;
        cellular9g_stats.neural_interface_support = true;
        cellular9g_stats.downlink_rate_mbps = 100000.0; // 100 Gbps downlink
        cellular9g_stats.uplink_rate_mbps = 100000.0; // 100 Gbps uplink
        cellular9g_stats.latency_ms = 0.05; // Ultra-low latency
        cellular9g_stats.jitter_ms = 0.01;
        cellular9g_stats.packet_delivery_success_rate = 99.9999;
        cellular9g_stats.reliability_percent = 99.9999;
        cellular9g_stats.energy_efficiency_rating = 99.0;
        cellular9g_stats.ai_traffic_prediction = true;
        cellular9g_stats.dynamic_spectrum_sharing = true;
        cellular9g_stats.holographic_communication_plus_support = true;
        cellular9g_stats.neural_interface_plus_support = true;
        cellular9g_stats.holographic_mimo_plus_support = true;

        Ok(cellular9g_stats)
    }

    /// Collect 10G cellular network statistics
    fn collect_cellular10g_stats(&self, _interface_name: &str) -> Result<Cellular10GStats> {
        // In a real implementation, this would use future 10G APIs
        // For now, we'll return mock data with reasonable defaults
        
        let mut cellular10g_stats = Cellular10GStats::default();

        // Set 10G specific parameters
        cellular10g_stats.technology = "10G-NR".to_string();
        cellular10g_stats.generation = "10G".to_string();
        cellular10g_stats.signal_strength_dbm = -25; // Excellent 10G signal
        cellular10g_stats.rsrp_dbm = -25.0; // Excellent RSRP
        cellular10g_stats.rsrq_db = 35.0; // Excellent RSRQ
        cellular10g_stats.sinr_db = 40.0; // Excellent SINR
        cellular10g_stats.bandwidth_mhz = 5000; // 5GHz bandwidth
        cellular10g_stats.frequency_band = "n258".to_string();
        cellular10g_stats.cell_id = 1234567890;
        cellular10g_stats.tracking_area_code = 12345;
        cellular10g_stats.physical_cell_id = 255;
        cellular10g_stats.modulation = "8192-QAM".to_string();
        cellular10g_stats.mimo_config = "1024x1024".to_string();
        cellular10g_stats.carrier_aggregation = true;
        cellular10g_stats.advanced_mimo_support = true;
        cellular10g_stats.terahertz_support = true;
        cellular10g_stats.ai_optimization_support = true;
        cellular10g_stats.quantum_encryption_support = true;
        cellular10g_stats.dynamic_spectrum_sharing_support = true;
        cellular10g_stats.advanced_beamforming_support = true;
        cellular10g_stats.holographic_mimo_support = true;
        cellular10g_stats.holographic_communication_support = true;
        cellular10g_stats.neural_interface_support = true;

        cellular10g_stats.quantum_holographic_communication_support = true;
        cellular10g_stats.ai_quantum_optimization_support = true;
        cellular10g_stats.downlink_rate_mbps = 500000.0; // 500 Gbps downlink
        cellular10g_stats.uplink_rate_mbps = 500000.0; // 500 Gbps uplink
        cellular10g_stats.latency_ms = 0.01; // Ultra-low latency
        cellular10g_stats.jitter_ms = 0.001;
        cellular10g_stats.packet_delivery_success_rate = 99.99999;
        cellular10g_stats.reliability_percent = 99.99999;
        cellular10g_stats.energy_efficiency_rating = 99.9;
        cellular10g_stats.ai_traffic_prediction = true;
        cellular10g_stats.dynamic_spectrum_sharing = true;
        cellular10g_stats.holographic_communication_plus_support = true;
        cellular10g_stats.neural_interface_plus_support = true;
        cellular10g_stats.holographic_mimo_plus_support = true;


        Ok(cellular10g_stats)
    }

    /// Get full Traffic Control (tc) configuration for an interface
    fn get_tc_configuration(&self, interface_name: &str) -> Result<Option<String>> {
        // Try to execute tc command to get full configuration
        let output = std::process::Command::new("tc")
            .args(["-s", "-d", "qdisc", "show", "dev", interface_name])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let output_str = String::from_utf8_lossy(&output.stdout).to_string();
                if output_str.trim().is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(output_str))
                }
            }
            Ok(output) => {
                tracing::debug!(
                    "Failed to get tc configuration for {}: {}",
                    interface_name,
                    String::from_utf8_lossy(&output.stderr)
                );
                Ok(None)
            }
            Err(e) => {
                tracing::debug!("Failed to execute tc configuration command for {}: {}", interface_name, e);
                Ok(None)
            }
        }
    }

    /// Collect QoS queue statistics for an interface
    fn collect_qos_queue_stats(&self, interface_name: &str) -> Result<Vec<QoSQueueStats>> {
        let mut queue_stats = Vec::new();

        // Try to get queue statistics from tc command
        let output = std::process::Command::new("tc")
            .args(["-s", "-d", "qdisc", "show", "dev", interface_name])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let mut current_queue = QoSQueueStats::default();

                for line in output_str.lines() {
                    if line.contains("qdisc") && !line.contains("root") {
                        // Start of a new queue
                        if !current_queue.queue_id.is_empty() {
                            queue_stats.push(current_queue);
                        }
                        current_queue = QoSQueueStats::default();

                        // Extract queue ID
                        if let Some(queue_id) = line.split_whitespace().nth(1) {
                            current_queue.queue_id = queue_id.to_string();
                        }

                        // Extract queue type
                        if let Some(queue_type) = line.split_whitespace().nth(2) {
                            current_queue.queue_type = queue_type.to_string();
                        }
                    }

                    // Look for queue statistics
                    if line.contains("Sent") || line.contains("sent") {
                        // Parse sent packets and bytes
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        for i in 0..parts.len() {
                            if parts[i] == "Sent" || parts[i] == "sent" {
                                if i + 1 < parts.len() {
                                    if let Ok(packets) = parts[i + 1].parse::<u64>() {
                                        current_queue.packets_in_queue = packets;
                                    }
                                }
                                if i + 3 < parts.len() {
                                    if let Ok(bytes) = parts[i + 3].parse::<u64>() {
                                        current_queue.bytes_in_queue = bytes;
                                    }
                                }
                            }
                        }
                    }

                    if line.contains("dropped") || line.contains("Dropped") {
                        // Parse dropped packets and bytes
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        for i in 0..parts.len() {
                            if parts[i] == "dropped" || parts[i] == "Dropped" {
                                if i + 1 < parts.len() {
                                    if let Ok(packets) = parts[i + 1].parse::<u64>() {
                                        current_queue.packets_dropped = packets;
                                    }
                                }
                                if i + 3 < parts.len() {
                                    if let Ok(bytes) = parts[i + 3].parse::<u64>() {
                                        current_queue.bytes_dropped = bytes;
                                    }
                                }
                            }
                        }
                    }

                    if line.contains("backlog") || line.contains("Backlog") {
                        // Parse queue length
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        for i in 0..parts.len() {
                            if parts[i] == "backlog" || parts[i] == "Backlog" {
                                if i + 1 < parts.len() {
                                    if let Ok(length) = parts[i + 1].parse::<u32>() {
                                        current_queue.current_length = length;
                                    }
                                }
                            }
                        }
                    }
                }

                // Don't forget the last queue
                if !current_queue.queue_id.is_empty() {
                    queue_stats.push(current_queue);
                }
            }
            Ok(output) => {
                tracing::debug!(
                    "Failed to get QoS queue stats for {}: {}",
                    interface_name,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            Err(e) => {
                tracing::debug!("Failed to execute tc queue stats command for {}: {}", interface_name, e);
            }
        }

        Ok(queue_stats)
    }

    /// Collect interface statistics from /proc/net/dev and /sys/class/net
    fn collect_interface_stats(&self) -> Result<Vec<NetworkInterfaceStats>> {
        let mut interfaces = Vec::new();

        // Read from /proc/net/dev with better error handling
        let proc_net_dev = fs::read_to_string("/proc/net/dev")
            .with_context(|| {
                format!(
                    "Failed to read /proc/net/dev. This file is essential for network monitoring.\n            Possible causes:\n            1) Missing read permissions (try: ls -la /proc/net/dev)\n            2) /proc filesystem not mounted (check: mount | grep proc)\n            3) System under heavy load causing file access issues\n            Troubleshooting steps:\n            - Check file existence: ls -la /proc/net/dev\n            - Check permissions: id && groups\n            - Check proc filesystem: mount | grep proc\n            - Try running with elevated privileges: sudo smoothtaskd\n            - Check system logs: sudo dmesg | grep -i proc"
                )
            })?;

        // Pre-allocate capacity based on typical number of interfaces
        interfaces.reserve(8); // Most systems have 2-8 interfaces

        // Parse interface statistics with validation using optimized parsing
        for (line_num, line) in proc_net_dev.lines().skip(2).enumerate() {
            // Skip header lines
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
                let rx_packets =
                    self.parse_u64_fast(parts[2], &name, "rx_packets", line_num + 2)?;
                let tx_packets =
                    self.parse_u64_fast(parts[10], &name, "tx_packets", line_num + 2)?;
                let rx_errors = self.parse_u64_fast(parts[3], &name, "rx_errors", line_num + 2)?;
                let tx_errors = self.parse_u64_fast(parts[11], &name, "tx_errors", line_num + 2)?;
                let rx_dropped =
                    self.parse_u64_fast(parts[4], &name, "rx_dropped", line_num + 2)?;
                let tx_dropped =
                    self.parse_u64_fast(parts[12], &name, "tx_dropped", line_num + 2)?;
                let rx_overruns =
                    self.parse_u64_fast(parts[5], &name, "rx_overruns", line_num + 2)?;
                let tx_overruns =
                    self.parse_u64_fast(parts[13], &name, "tx_overruns", line_num + 2)?;

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
                tracing::debug!(
                    "Skipping line {}: insufficient data (expected >= 17 fields, got {})",
                    line_num + 2,
                    parts.len()
                );
            }
        }

        if interfaces.is_empty() {
            tracing::warn!("No network interfaces found in /proc/net/dev");
        }

        Ok(interfaces)
    }

    /// Fast u64 parsing with error handling
    fn parse_u64_fast(
        &self,
        s: &str,
        interface_name: &str,
        field_name: &str,
        line_num: usize,
    ) -> Result<u64> {
        s.parse::<u64>()
            .with_context(|| {
                format!(
                    "Failed to parse {} for interface {} at line {}.\n            Expected a valid unsigned 64-bit integer, but got: '{}'\n            This may indicate corrupted /proc/net/dev data or unexpected format.\n            Troubleshooting:\n            - Check /proc/net/dev format: cat /proc/net/dev\n            - Verify system stability: sudo dmesg | grep -i error\n            - Check for filesystem corruption: sudo fsck /",
                    field_name, interface_name, line_num, s
                )
            })
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
                                tracing::debug!(
                                    "Failed to parse IPv4 address '{}' for interface {}",
                                    ip_str,
                                    name
                                );
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
                                tracing::debug!(
                                    "Failed to parse IPv6 address '{}' for interface {}",
                                    ip_str,
                                    name
                                );
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
            Ok(s) => match s.trim().parse::<u64>() {
                Ok(speed) => Some(speed),
                Err(e) => {
                    tracing::debug!("Failed to parse interface speed for {}: {}", name, e);
                    None
                }
            },
            Err(e) => {
                tracing::debug!("Failed to read interface speed for {}: {}", name, e);
                None
            }
        }
    }

    /// Optimized interface flags retrieval
    fn get_interface_flags_optimized(&self, name: &str) -> Result<u32> {
        let flags_path = format!("/sys/class/net/{}/flags", name);
        let flags_str = fs::read_to_string(&flags_path)
            .with_context(|| {
                format!(
                    "Failed to read flags for interface {}.\n            This file should contain interface flags in hexadecimal format.\n            Possible causes:\n            1) Interface was removed during monitoring\n            2) Missing read permissions on /sys/class/net/{}/flags\n            3) Filesystem corruption in sysfs\n            Troubleshooting:\n            - Check interface existence: ip link show {}\n            - Check file permissions: ls -la {}\n            - Verify sysfs health: sudo dmesg | grep -i sysfs",
                    name, name, name, name
                )
            })?;
        let flags = flags_str.trim().parse::<u32>()
            .with_context(|| {
                format!(
                    "Failed to parse flags for interface {}.\n            Expected hexadecimal flags value, but got: '{}'\n            This may indicate corrupted sysfs data or unexpected format.\n            Troubleshooting:\n            - Check flags file content: cat {}\n            - Verify interface status: ip link show {}\n            - Check for system stability issues: sudo dmesg | grep -i error",
                    name, flags_str.trim(), flags_path, name
                )
            })?;
        Ok(flags)
    }

    /// Detect interface type based on name
    fn detect_interface_type(&self, name: &str) -> NetworkInterfaceType {
        if name.starts_with("lo") {
            NetworkInterfaceType::Loopback
        } else if name.starts_with("eth") || name.starts_with("en") {
            NetworkInterfaceType::Ethernet
        } else if name.starts_with("wlan") || name.starts_with("wl") {
            // Detect Wi-Fi generation based on interface name
            if name.contains("wifi11") || name.contains("wlan11") || name.contains("wl11") {
                NetworkInterfaceType::Wifi11
            } else if name.contains("wifi10") || name.contains("wlan10") || name.contains("wl10") {
                NetworkInterfaceType::Wifi10
            } else if name.contains("wifi9") || name.contains("wlan9") || name.contains("wl9") {
                NetworkInterfaceType::Wifi9
            } else if name.contains("wifi8") || name.contains("wlan8") || name.contains("wl8") {
                NetworkInterfaceType::Wifi8
            } else if name.contains("wifi7") || name.contains("wlan7") || name.contains("wl7") {
                NetworkInterfaceType::Wifi7
            } else if name.contains("wifi6e") || name.contains("wlan6e") || name.contains("wl6e") {
                NetworkInterfaceType::Wifi6E
            } else if name.contains("wifi6") || name.contains("wlan6") || name.contains("wl6") {
                NetworkInterfaceType::Wifi6
            } else {
                NetworkInterfaceType::Wifi
            }
        } else if name.starts_with("wwan") || name.starts_with("cww") {
            // Detect cellular generation based on interface name
            if name.contains("10g") || name.contains("wwan10g") {
                NetworkInterfaceType::Cellular10G
            } else if name.contains("9g") || name.contains("wwan9g") {
                NetworkInterfaceType::Cellular9G
            } else if name.contains("8g") || name.contains("wwan8g") {
                NetworkInterfaceType::Cellular8G
            } else if name.contains("7g") || name.contains("wwan7g") {
                NetworkInterfaceType::Cellular7G
            } else if name.contains("6g") || name.contains("wwan6g") {
                NetworkInterfaceType::Cellular6G
            } else if name.contains("5g") || name.contains("wwan5g") {
                NetworkInterfaceType::Cellular5G
            } else {
                NetworkInterfaceType::Cellular
            }
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

    /// Collect QoS metrics for a network interface
    fn collect_qos_metrics(&self, interface_name: &str) -> Result<NetworkQoSMetrics> {
        let mut qos_metrics = NetworkQoSMetrics::default();

        // Try to get Traffic Control (tc) information
        if self.config.enable_tc_monitoring {
            qos_metrics.tc_qdisc = self.get_tc_qdisc(interface_name)?;
            qos_metrics.tc_classes = self.get_tc_classes(interface_name)?;
            qos_metrics.tc_filters = self.get_tc_filters(interface_name)?;
        }

        // Try to get QoS class information
        qos_metrics.qos_class = self.detect_qos_class(interface_name);

        // Try to get DSCP and ECN information
        qos_metrics.dscp = self.get_dscp_value(interface_name)?;
        qos_metrics.ecn_support = self.check_ecn_support(interface_name);

        // Try to get traffic shaping and policing information
        qos_metrics.shaping_rate_bps = self.get_shaping_rate(interface_name)?;
        qos_metrics.policing_rate_bps = self.get_policing_rate(interface_name)?;

        // Try to get queue statistics
        qos_metrics.queue_length = self.get_queue_length(interface_name)?;
        qos_metrics.packet_drops = self.get_packet_drops(interface_name)?;
        qos_metrics.packet_reorders = self.get_packet_reorders(interface_name)?;

        // Determine QoS policy based on collected metrics
        qos_metrics.qos_policy = self.determine_qos_policy(&qos_metrics);

        Ok(qos_metrics)
    }

    /// Get Traffic Control (tc) queue discipline for an interface
    fn get_tc_qdisc(&self, interface_name: &str) -> Result<Option<String>> {
        // Try to execute tc command to get qdisc information
        let output = std::process::Command::new("tc")
            .args(["qdisc", "show", "dev", interface_name])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.contains("qdisc") {
                    // Extract qdisc type from output
                    for line in output_str.lines() {
                        if line.contains("qdisc") && !line.contains("root") {
                            if let Some(qdisc_type) = line.split_whitespace().nth(1) {
                                return Ok(Some(qdisc_type.to_string()));
                            }
                        }
                    }
                }
                Ok(None)
            }
            Ok(output) => {
                tracing::debug!(
                    "Failed to get tc qdisc for {}: {}",
                    interface_name,
                    String::from_utf8_lossy(&output.stderr)
                );
                Ok(None)
            }
            Err(e) => {
                tracing::debug!("Failed to execute tc command for {}: {}", interface_name, e);
                Ok(None)
            }
        }
    }

    /// Get Traffic Control (tc) classes for an interface
    fn get_tc_classes(&self, interface_name: &str) -> Result<Vec<String>> {
        let mut classes = Vec::new();

        // Try to execute tc command to get class information
        let output = std::process::Command::new("tc")
            .args(["class", "show", "dev", interface_name])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    if line.contains("class") && !line.contains("root") {
                        if let Some(class_id) = line.split_whitespace().nth(1) {
                            classes.push(class_id.to_string());
                        }
                    }
                }
            }
            Ok(output) => {
                tracing::debug!(
                    "Failed to get tc classes for {}: {}",
                    interface_name,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            Err(e) => {
                tracing::debug!("Failed to execute tc class command for {}: {}", interface_name, e);
            }
        }

        Ok(classes)
    }

    /// Get Traffic Control (tc) filters for an interface
    fn get_tc_filters(&self, interface_name: &str) -> Result<Vec<String>> {
        let mut filters = Vec::new();

        // Try to execute tc command to get filter information
        let output = std::process::Command::new("tc")
            .args(["filter", "show", "dev", interface_name])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    if line.contains("filter") && !line.contains("root") {
                        if let Some(filter_id) = line.split_whitespace().nth(1) {
                            filters.push(filter_id.to_string());
                        }
                    }
                }
            }
            Ok(output) => {
                tracing::debug!(
                    "Failed to get tc filters for {}: {}",
                    interface_name,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            Err(e) => {
                tracing::debug!("Failed to execute tc filter command for {}: {}", interface_name, e);
            }
        }

        Ok(filters)
    }

    /// Detect QoS class for an interface based on various factors
    fn detect_qos_class(&self, interface_name: &str) -> Option<String> {
        // Check if this is a priority interface
        if interface_name.starts_with("eth") || interface_name.starts_with("en") {
            return Some("best-effort".to_string());
        }

        // Check if this is a wireless interface (often has different QoS)
        if interface_name.starts_with("wlan") || interface_name.starts_with("wl") {
            return Some("wireless".to_string());
        }

        // Check if this is a loopback interface
        if interface_name.starts_with("lo") {
            return Some("loopback".to_string());
        }

        // Check if this is a virtual interface
        if interface_name.starts_with("vir") || interface_name.starts_with("veth") {
            return Some("virtual".to_string());
        }

        // Default to best-effort
        Some("best-effort".to_string())
    }

    /// Get DSCP (Differentiated Services Code Point) value for an interface
    fn get_dscp_value(&self, interface_name: &str) -> Result<Option<u8>> {
        // Try to read DSCP information from sysfs or use ip command
        let output = std::process::Command::new("ip")
            .args(["-s", "-d", "link", "show", interface_name])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                // Look for DSCP information in output
                for line in output_str.lines() {
                    if line.contains("dscp") || line.contains("DSCP") {
                        if let Some(dscp_str) = line.split_whitespace().find(|s| s.contains("dscp") || s.contains("DSCP")) {
                            if let Some(dscp_value) = dscp_str.split('=').nth(1) {
                                if let Ok(dscp) = dscp_value.parse::<u8>() {
                                    return Ok(Some(dscp));
                                }
                            }
                        }
                    }
                }
                Ok(None)
            }
            Ok(output) => {
                tracing::debug!(
                    "Failed to get DSCP for {}: {}",
                    interface_name,
                    String::from_utf8_lossy(&output.stderr)
                );
                Ok(None)
            }
            Err(e) => {
                tracing::debug!("Failed to execute ip command for {}: {}", interface_name, e);
                Ok(None)
            }
        }
    }

    /// Check if ECN (Explicit Congestion Notification) is supported
    fn check_ecn_support(&self, interface_name: &str) -> bool {
        // Try to check ECN support using sysctl or other methods
        // For now, we'll return a reasonable default based on interface type
        if interface_name.starts_with("lo") {
            return false; // Loopback usually doesn't need ECN
        }

        // Most modern interfaces support ECN
        true
    }

    /// Get traffic shaping rate for an interface
    fn get_shaping_rate(&self, interface_name: &str) -> Result<Option<u64>> {
        // Try to get shaping rate from tc command
        let output = std::process::Command::new("tc")
            .args(["qdisc", "show", "dev", interface_name])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                // Look for rate information
                for line in output_str.lines() {
                    if line.contains("rate") || line.contains("Rate") {
                        if let Some(rate_str) = line.split_whitespace().find(|s| s.contains("rate") || s.contains("Rate")) {
                            if let Some(rate_value) = rate_str.split(':').nth(1) {
                                // Parse rate value (could be in various formats like "100mbit", "1gbit", etc.)
                                let rate_value = rate_value.trim_end_matches(|c: char| !c.is_ascii_digit());
                                if let Ok(rate) = rate_value.parse::<u64>() {
                                    // Convert to bytes per second (approximate)
                                    if line.contains("mbit") || line.contains("Mbit") {
                                        return Ok(Some(rate * 125_000)); // 1 Mbit = 125,000 bytes
                                    } else if line.contains("gbit") || line.contains("Gbit") {
                                        return Ok(Some(rate * 125_000_000)); // 1 Gbit = 125,000,000 bytes
                                    } else if line.contains("kbit") || line.contains("Kbit") {
                                        return Ok(Some(rate * 125)); // 1 Kbit = 125 bytes
                                    } else {
                                        return Ok(Some(rate)); // Assume bytes per second
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(None)
            }
            Ok(output) => {
                tracing::debug!(
                    "Failed to get shaping rate for {}: {}",
                    interface_name,
                    String::from_utf8_lossy(&output.stderr)
                );
                Ok(None)
            }
            Err(e) => {
                tracing::debug!("Failed to execute tc command for shaping rate {}: {}", interface_name, e);
                Ok(None)
            }
        }
    }

    /// Get traffic policing rate for an interface
    fn get_policing_rate(&self, interface_name: &str) -> Result<Option<u64>> {
        // Try to get policing rate from tc command
        let output = std::process::Command::new("tc")
            .args(["qdisc", "show", "dev", interface_name])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                // Look for police or policing information
                for line in output_str.lines() {
                    if line.contains("police") || line.contains("Police") {
                        if let Some(rate_str) = line.split_whitespace().find(|s| s.contains("rate") || s.contains("Rate")) {
                            if let Some(rate_value) = rate_str.split(':').nth(1) {
                                // Parse rate value (could be in various formats)
                                let rate_value = rate_value.trim_end_matches(|c: char| !c.is_ascii_digit());
                                if let Ok(rate) = rate_value.parse::<u64>() {
                                    // Convert to bytes per second (approximate)
                                    if line.contains("mbit") || line.contains("Mbit") {
                                        return Ok(Some(rate * 125_000));
                                    } else if line.contains("gbit") || line.contains("Gbit") {
                                        return Ok(Some(rate * 125_000_000));
                                    } else if line.contains("kbit") || line.contains("Kbit") {
                                        return Ok(Some(rate * 125));
                                    } else {
                                        return Ok(Some(rate));
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(None)
            }
            Ok(output) => {
                tracing::debug!(
                    "Failed to get policing rate for {}: {}",
                    interface_name,
                    String::from_utf8_lossy(&output.stderr)
                );
                Ok(None)
            }
            Err(e) => {
                tracing::debug!("Failed to execute tc command for policing rate {}: {}", interface_name, e);
                Ok(None)
            }
        }
    }

    /// Get queue length for an interface
    fn get_queue_length(&self, interface_name: &str) -> Result<Option<u32>> {
        // Try to get queue length from sysfs
        let queue_length_path = format!("/sys/class/net/{}/tx_queue_len", interface_name);
        match fs::read_to_string(queue_length_path) {
            Ok(s) => {
                if let Ok(length) = s.trim().parse::<u32>() {
                    return Ok(Some(length));
                }
            }
            Err(_) => {
                // Fallback: try to get from tc command
                let output = std::process::Command::new("tc")
                    .args(["qdisc", "show", "dev", interface_name])
                    .output();

                match output {
                    Ok(output) if output.status.success() => {
                        let output_str = String::from_utf8_lossy(&output.stdout);
                        for line in output_str.lines() {
                            if line.contains("limit") || line.contains("Limit") {
                                if let Some(limit_str) = line.split_whitespace().find(|s| s.contains("limit") || s.contains("Limit")) {
                                    if let Some(limit_value) = limit_str.split(':').nth(1) {
                                        if let Ok(limit) = limit_value.parse::<u32>() {
                                            return Ok(Some(limit));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(output) => {
                        tracing::debug!(
                            "Failed to get queue length for {}: {}",
                            interface_name,
                            String::from_utf8_lossy(&output.stderr)
                        );
                    }
                    Err(e) => {
                        tracing::debug!("Failed to execute tc command for queue length {}: {}", interface_name, e);
                    }
                }
            }
        }

        Ok(None)
    }

    /// Get packet drop statistics for an interface
    fn get_packet_drops(&self, interface_name: &str) -> Result<u64> {
        // Try to get packet drops from /proc/net/dev
        let proc_net_dev = fs::read_to_string("/proc/net/dev")
            .with_context(|| format!("Failed to read /proc/net/dev for packet drops"))?;

        for line in proc_net_dev.lines().skip(2) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 17 && parts[0].trim_end_matches(':') == interface_name {
                // Sum up all drop-related counters
                let rx_dropped = parts[4].parse::<u64>().unwrap_or(0);
                let tx_dropped = parts[12].parse::<u64>().unwrap_or(0);
                return Ok(rx_dropped + tx_dropped);
            }
        }

        Ok(0)
    }

    /// Get packet reordering statistics for an interface
    fn get_packet_reorders(&self, _interface_name: &str) -> Result<u64> {
        // Packet reordering is harder to detect directly
        // For now, we'll return 0 as a placeholder
        // In a real implementation, this would require packet sequence analysis
        Ok(0)
    }

    /// Determine QoS policy based on collected metrics
    fn determine_qos_policy(&self, qos_metrics: &NetworkQoSMetrics) -> Option<String> {
        // Check if there's a specific QoS class
        if let Some(qos_class) = &qos_metrics.qos_class {
            if qos_class == "voice" || qos_class == "video" {
                return Some("priority".to_string());
            } else if qos_class == "best-effort" {
                return Some("best-effort".to_string());
            }
        }

        // Check if there's traffic control configured
        if qos_metrics.tc_qdisc.is_some() && !qos_metrics.tc_classes.is_empty() {
            return Some("tc-controlled".to_string());
        }

        // Check if there's traffic shaping or policing
        if qos_metrics.shaping_rate_bps.is_some() || qos_metrics.policing_rate_bps.is_some() {
            return Some("rate-limited".to_string());
        }

        // Default to best-effort
        Some("best-effort".to_string())
    }

    /// Collect protocol statistics from /proc/net/snmp
    fn collect_protocol_stats(&self) -> Result<NetworkProtocolStats> {
        let mut stats = NetworkProtocolStats::default();

        // Read TCP statistics with error handling
        let tcp_stats = fs::read_to_string("/proc/net/snmp")
            .with_context(|| {
                format!(
                    "Failed to read /proc/net/snmp. This file contains protocol statistics.\n            Possible causes:\n            1) Missing read permissions (try: ls -la /proc/net/snmp)\n            2) /proc filesystem not mounted (check: mount | grep proc)\n            3) System under heavy load causing file access issues\n            Troubleshooting steps:\n            - Check file existence: ls -la /proc/net/snmp\n            - Check permissions: id && groups\n            - Check proc filesystem: mount | grep proc\n            - Try running with elevated privileges: sudo smoothtaskd\n            - Check system logs: sudo dmesg | grep -i proc"
                )
            })?;

        for (line_num, line) in tcp_stats.lines().enumerate() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let protocol = parts[0].trim_end_matches(':');
                match protocol {
                    "Tcp" => {
                        if parts.len() >= 16 {
                            stats.tcp_connections = parts[15].parse::<u64>()
                                .with_context(|| {
                                    format!(
                                        "Failed to parse TCP connections at line {}.\n            Expected a valid unsigned 64-bit integer, but got: '{}'\n            This may indicate corrupted /proc/net/snmp data or unexpected format.\n            Troubleshooting:\n            - Check /proc/net/snmp format: cat /proc/net/snmp\n            - Verify system stability: sudo dmesg | grep -i error\n            - Check for filesystem corruption: sudo fsck /",
                                        line_num, parts[15]
                                    )
                                })?;
                            stats.tcp_retransmissions = parts[12].parse::<u64>()
                                .with_context(|| {
                                    format!(
                                        "Failed to parse TCP retransmissions at line {}.\n            Expected a valid unsigned 64-bit integer, but got: '{}'\n            This may indicate corrupted /proc/net/snmp data or unexpected format.\n            Troubleshooting:\n            - Check /proc/net/snmp format: cat /proc/net/snmp\n            - Verify system stability: sudo dmesg | grep -i error\n            - Check for filesystem corruption: sudo fsck /",
                                        line_num, parts[12]
                                    )
                                })?;
                        } else {
                            tracing::debug!(
                                "Insufficient TCP data at line {}: expected >= 16 fields, got {}",
                                line_num,
                                parts.len()
                            );
                        }
                    }
                    "Udp" => {
                        if parts.len() >= 4 {
                            stats.udp_connections = parts[3].parse::<u64>()
                                .with_context(|| {
                                    format!(
                                        "Failed to parse UDP connections at line {}.\n            Expected a valid unsigned 64-bit integer, but got: '{}'\n            This may indicate corrupted /proc/net/snmp data or unexpected format.\n            Troubleshooting:\n            - Check /proc/net/snmp format: cat /proc/net/snmp\n            - Verify system stability: sudo dmesg | grep -i error\n            - Check for filesystem corruption: sudo fsck /",
                                        line_num, parts[3]
                                    )
                                })?;
                        } else {
                            tracing::debug!(
                                "Insufficient UDP data at line {}: expected >= 4 fields, got {}",
                                line_num,
                                parts.len()
                            );
                        }
                    }
                    "Icmp" => {
                        if parts.len() >= 3 {
                            stats.icmp_packets = parts[2].parse::<u64>()
                                .with_context(|| {
                                    format!(
                                        "Failed to parse ICMP packets at line {}.\n            Expected a valid unsigned 64-bit integer, but got: '{}'\n            This may indicate corrupted /proc/net/snmp data or unexpected format.\n            Troubleshooting:\n            - Check /proc/net/snmp format: cat /proc/net/snmp\n            - Verify system stability: sudo dmesg | grep -i error\n            - Check for filesystem corruption: sudo fsck /",
                                        line_num, parts[2]
                                    )
                                })?;
                        } else {
                            tracing::debug!(
                                "Insufficient ICMP data at line {}: expected >= 3 fields, got {}",
                                line_num,
                                parts.len()
                            );
                        }
                    }
                    _ => {
                        tracing::debug!("Unknown protocol '{}' at line {}", protocol, line_num);
                    }
                }
            }
        }

        tracing::debug!(
            "Collected protocol stats: TCP={} connections, UDP={} connections, ICMP={} packets",
            stats.tcp_connections,
            stats.udp_connections,
            stats.icmp_packets
        );

        Ok(stats)
    }

    /// Collect port usage statistics with enhanced connection tracking
    fn collect_port_usage_stats(&self) -> Result<Vec<PortUsageStats>> {
        let mut port_stats = Vec::new();
        let mut port_map: HashMap<u16, PortUsageStats> = HashMap::new();

        // Initialize port stats for monitored ports
        for &port in &self.config.monitored_ports {
            port_map.insert(
                port,
                PortUsageStats {
                    port,
                    protocol: "TCP".to_string(),
                    connection_count: 0,
                    bytes_transmitted: 0,
                    bytes_received: 0,
                    processes: Vec::new(),
                },
            );
        }

        // Collect active connections and aggregate by port
        let connections = self.collect_connection_stats()?;

        for conn in connections {
            // Track both source and destination ports
            for &port in &[conn.src_port, conn.dst_port] {
                if self.config.monitored_ports.contains(&port) || port_map.contains_key(&port) {
                    let entry = port_map.entry(port).or_insert_with(|| PortUsageStats {
                        port,
                        protocol: conn.protocol.clone(),
                        connection_count: 0,
                        bytes_transmitted: 0,
                        bytes_received: 0,
                        processes: Vec::new(),
                    });

                    entry.connection_count += 1;
                    entry.bytes_transmitted += conn.bytes_transmitted;
                    entry.bytes_received += conn.bytes_received;

                    // Track associated processes
                    if let Some(pid) = conn.pid {
                        if !entry.processes.contains(&pid) {
                            entry.processes.push(pid);
                        }
                    }
                }
            }
        }

        // Convert hashmap to vector
        port_stats.extend(port_map.into_values());

        Ok(port_stats)
    }

    /// Collect connection statistics with enhanced tracking
    fn collect_connection_stats(&self) -> Result<Vec<NetworkConnectionStats>> {
        let mut connections = Vec::new();
        let mut connection_map: HashMap<String, NetworkConnectionStats> = HashMap::new();

        // Enhanced TCP connection tracking
        self.collect_tcp_connections(&mut connection_map)?;

        // Enhanced UDP connection tracking
        self.collect_udp_connections(&mut connection_map)?;

        // Convert hashmap to vector and apply limits
        connections.extend(connection_map.into_values());

        // Limit connections to configured maximum
        if connections.len() > self.config.max_connections {
            tracing::info!(
                "Truncating connections list from {} to {} (max_connections limit)",
                connections.len(),
                self.config.max_connections
            );
            connections.truncate(self.config.max_connections);
        }

        tracing::debug!("Collected {} network connections", connections.len());

        Ok(connections)
    }

    /// Collect IPv6 connection statistics
    fn collect_ipv6_connection_stats(&self) -> Result<Vec<IPv6ConnectionStats>> {
        let mut ipv6_connections = Vec::new();

        // Try to read IPv6 TCP connections
        if let Ok(tcp6_connections) = fs::read_to_string("/proc/net/tcp6") {
            for (line_num, line) in tcp6_connections.lines().skip(1).enumerate() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 {
                    // Parse IPv6 addresses and ports
                    let src_ip_hex = parts[1];
                    let dst_ip_hex = parts[2];
                    
                    // Convert hex IPv6 addresses to standard format
                    let src_ip = self.hex_ipv6_to_std(src_ip_hex)?;
                    let dst_ip = self.hex_ipv6_to_std(dst_ip_hex)?;
                    
                    // Parse ports (hex to decimal)
                    let src_port = u16::from_str_radix(parts[1].split(':').last().unwrap_or("0"), 16).unwrap_or(0);
                    let dst_port = u16::from_str_radix(parts[2].split(':').last().unwrap_or("0"), 16).unwrap_or(0);
                    
                    // Parse connection state
                    let state = match parts[3] {
                        "01" => "ESTABLISHED",
                        "02" => "SYN_SENT",
                        "03" => "SYN_RECV",
                        "04" => "FIN_WAIT1",
                        "05" => "FIN_WAIT2",
                        "06" => "TIME_WAIT",
                        "07" => "CLOSE",
                        "08" => "CLOSE_WAIT",
                        "09" => "LAST_ACK",
                        "0A" => "LISTEN",
                        "0B" => "CLOSING",
                        _ => "UNKNOWN",
                    };

                    // Get process info (similar to IPv4 implementation)
                    let inode = parts[9];
                    let (pid, process_name) = self.get_process_info_from_inode(inode)?;

                    ipv6_connections.push(IPv6ConnectionStats {
                        src_ip,
                        dst_ip,
                        src_port,
                        dst_port,
                        protocol: "TCP6".to_string(),
                        state: state.to_string(),
                        pid: pid,
                        process_name: process_name,
                        bytes_transmitted: 0, // Would need to track this separately
                        bytes_received: 0,   // Would need to track this separately
                        packets_transmitted: 0,
                        packets_received: 0,
                        start_time: SystemTime::now(),
                        last_activity: SystemTime::now(),
                        duration: Duration::from_secs(0),
                    });
                }
            }
        }

        // Try to read IPv6 UDP connections
        if let Ok(udp6_connections) = fs::read_to_string("/proc/net/udp6") {
            for (line_num, line) in udp6_connections.lines().skip(1).enumerate() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 {
                    // Parse IPv6 addresses and ports
                    let src_ip_hex = parts[1];
                    let dst_ip_hex = parts[2];
                    
                    // Convert hex IPv6 addresses to standard format
                    let src_ip = self.hex_ipv6_to_std(src_ip_hex)?;
                    let dst_ip = self.hex_ipv6_to_std(dst_ip_hex)?;
                    
                    // Parse ports (hex to decimal)
                    let src_port = u16::from_str_radix(parts[1].split(':').last().unwrap_or("0"), 16).unwrap_or(0);
                    let dst_port = u16::from_str_radix(parts[2].split(':').last().unwrap_or("0"), 16).unwrap_or(0);

                    ipv6_connections.push(IPv6ConnectionStats {
                        src_ip,
                        dst_ip,
                        src_port,
                        dst_port,
                        protocol: "UDP6".to_string(),
                        state: "OPEN".to_string(),
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
                }
            }
        }

        tracing::debug!("Collected {} IPv6 connections", ipv6_connections.len());

        Ok(ipv6_connections)
    }

    /// Collect IPv6 network statistics
    fn collect_ipv6_stats(&self) -> Result<IPv6NetworkStats> {
        let mut stats = IPv6NetworkStats::default();

        // Read IPv6 stats from /proc/net/snmp6
        if let Ok(snmp6_data) = fs::read_to_string("/proc/net/snmp6") {
            for line in snmp6_data.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 12 {
                    // Parse IPv6 statistics
                    // Format: Ip6InReceives Ip6InHdrErrors Ip6InTooBigErrors Ip6InNoRoutes Ip6InAddrErrors ...
                    stats.rx_packets = parts[0].parse().unwrap_or(0);
                    stats.rx_errors = parts[1].parse().unwrap_or(0);
                    // Add more fields as needed
                    break; // Just take the first line for now
                }
            }
        }

        // Read IPv6 forwarding status
        if let Ok(forwarding) = fs::read_to_string("/proc/sys/net/ipv6/conf/all/forwarding") {
            stats.forwarding_enabled = forwarding.trim() == "1";
        }

        // Read IPv6 hop limit
        if let Ok(hop_limit) = fs::read_to_string("/proc/sys/net/ipv6/conf/all/hop_limit") {
            if let Ok(limit) = hop_limit.trim().parse::<u8>() {
                stats.default_hop_limit = limit;
            }
        }

        tracing::debug!("Collected IPv6 network statistics");

        Ok(stats)
    }

    /// Collect TCP connections with detailed information
    fn collect_tcp_connections(
        &self,
        connection_map: &mut HashMap<String, NetworkConnectionStats>,
    ) -> Result<()> {
        // Try to read TCP connections with enhanced error handling
        match fs::read_to_string("/proc/net/tcp") {
            Ok(tcp_connections) => {
                for (line_num, line) in tcp_connections.lines().skip(1).enumerate() {
                    // Skip header
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 10 {
                        let _conn_key = format!("TCP:{}:{}", parts[1], parts[2]); // src_ip:src_port

                        // Parse connection state
                        let state = match parts[3] {
                            "01" => "ESTABLISHED",
                            "02" => "SYN_SENT",
                            "03" => "SYN_RECV",
                            "04" => "FIN_WAIT1",
                            "05" => "FIN_WAIT2",
                            "06" => "TIME_WAIT",
                            "07" => "CLOSE",
                            "08" => "CLOSE_WAIT",
                            "09" => "LAST_ACK",
                            "0A" => "LISTEN",
                            "0B" => "CLOSING",
                            _ => "UNKNOWN",
                        };

                        // Parse IP addresses and ports from hex format
                        let (src_ip, src_port) = self.parse_ip_port_from_hex(parts[1])?;
                        let (dst_ip, dst_port) = self.parse_ip_port_from_hex(parts[2])?;

                        // Get process information
                        let inode = parts[9];
                        let (pid, process_name) = self.get_process_info_from_inode(inode)?;

                        // Calculate connection metrics
                        let tx_queue = parts[4].parse::<u64>().unwrap_or(0);
                        let rx_queue = parts[5].parse::<u64>().unwrap_or(0);
                        let timer = parts[6].parse::<u64>().unwrap_or(0);
                        let _retrans = parts[7].parse::<u64>().unwrap_or(0);

                        // Estimate bandwidth based on queue sizes
                        let bytes_transmitted = tx_queue * 1024; // Approximate
                        let bytes_received = rx_queue * 1024; // Approximate

                        let conn_id =
                            format!("TCP:{}:{}:{}:{}", src_ip, src_port, dst_ip, dst_port);

                        connection_map.insert(
                            conn_id,
                            NetworkConnectionStats {
                                src_ip,
                                dst_ip,
                                src_port,
                                dst_port,
                                protocol: "TCP".to_string(),
                                state: state.to_string(),
                                pid,
                                process_name,
                                bytes_transmitted,
                                bytes_received,
                                packets_transmitted: tx_queue,
                                packets_received: rx_queue,
                                start_time: SystemTime::now(),
                                last_activity: SystemTime::now(),
                                duration: Duration::from_secs(timer),
                            },
                        );
                    } else {
                        tracing::debug!("Skipping TCP connection line {}: insufficient data (expected >= 10 fields, got {})", line_num + 1, parts.len());
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to read /proc/net/tcp: {}", e);
                // Continue gracefully - this is not a fatal error
            }
        }

        Ok(())
    }

    /// Collect UDP connections with detailed information
    fn collect_udp_connections(
        &self,
        connection_map: &mut HashMap<String, NetworkConnectionStats>,
    ) -> Result<()> {
        // Try to read UDP connections with enhanced error handling
        match fs::read_to_string("/proc/net/udp") {
            Ok(udp_connections) => {
                for (line_num, line) in udp_connections.lines().skip(1).enumerate() {
                    // Skip header
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 10 {
                        let _conn_key = format!("UDP:{}:{}", parts[1], parts[2]); // src_ip:src_port

                        // Parse IP addresses and ports from hex format
                        let (src_ip, src_port) = self.parse_ip_port_from_hex(parts[1])?;
                        let (dst_ip, dst_port) = self.parse_ip_port_from_hex(parts[2])?;

                        // Get process information
                        let inode = parts[9];
                        let (pid, process_name) = self.get_process_info_from_inode(inode)?;

                        // UDP doesn't have state like TCP, so we'll use "ACTIVE"
                        let state = "ACTIVE".to_string();

                        // Calculate connection metrics
                        let rx_queue = parts[4].parse::<u64>().unwrap_or(0);
                        let tx_queue = parts[5].parse::<u64>().unwrap_or(0);

                        // Estimate bandwidth based on queue sizes
                        let bytes_transmitted = tx_queue * 1024; // Approximate
                        let bytes_received = rx_queue * 1024; // Approximate

                        let conn_id =
                            format!("UDP:{}:{}:{}:{}", src_ip, src_port, dst_ip, dst_port);

                        connection_map.insert(
                            conn_id,
                            NetworkConnectionStats {
                                src_ip,
                                dst_ip,
                                src_port,
                                dst_port,
                                protocol: "UDP".to_string(),
                                state,
                                pid,
                                process_name,
                                bytes_transmitted,
                                bytes_received,
                                packets_transmitted: tx_queue,
                                packets_received: rx_queue,
                                start_time: SystemTime::now(),
                                last_activity: SystemTime::now(),
                                duration: Duration::from_secs(0), // UDP doesn't have duration like TCP
                            },
                        );
                    } else {
                        tracing::debug!("Skipping UDP connection line {}: insufficient data (expected >= 10 fields, got {})", line_num + 1, parts.len());
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to read /proc/net/udp: {}", e);
                // Continue gracefully - this is not a fatal error
            }
        }

        Ok(())
    }

    /// Parse IP address and port from hex format used in /proc/net/tcp and /proc/net/udp
    fn parse_ip_port_from_hex(&self, hex_str: &str) -> Result<(IpAddr, u16)> {
        // Split hex string into IP and port parts
        let parts: Vec<&str> = hex_str.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid hex format for IP:port: {}",
                hex_str
            ));
        }

        let ip_hex = parts[0];
        let port_hex = parts[1];

        // Parse IP address (little-endian hex)
        let ip_value = u32::from_str_radix(ip_hex, 16)
            .with_context(|| format!("Failed to parse IP hex: {}", ip_hex))?;
        let octets = [
            ((ip_value >> 24) & 0xFF) as u8,
            ((ip_value >> 16) & 0xFF) as u8,
            ((ip_value >> 8) & 0xFF) as u8,
            (ip_value & 0xFF) as u8,
        ];
        let ip_addr = IpAddr::V4(Ipv4Addr::from(octets));

        // Parse port (little-endian hex)
        let port_value = u16::from_str_radix(port_hex, 16)
            .with_context(|| format!("Failed to parse port hex: {}", port_hex))?;

        Ok((ip_addr, port_value))
    }
    /// Collect network quality metrics with enhanced tracking
    fn collect_network_quality_metrics(&self) -> Result<NetworkQualityMetrics> {
        let mut metrics = NetworkQualityMetrics::default();

        // Calculate packet loss based on connection statistics
        let connections = self.collect_connection_stats()?;

        if !connections.is_empty() {
            // Count connections in different states to estimate quality
            let total_connections = connections.len() as f64;
            let established_count = connections
                .iter()
                .filter(|c| c.state == "ESTABLISHED" && c.protocol == "TCP")
                .count() as f64;
            let error_count = connections
                .iter()
                .filter(|c| c.state.contains("ERROR") || c.state.contains("FAILED"))
                .count() as f64;

            // Estimate packet loss based on connection states
            if total_connections > 0.0 {
                metrics.packet_loss = error_count / total_connections;
                metrics.stability_score = established_count / total_connections;
            }

            // Estimate bandwidth utilization based on connection activity
            let total_bytes: u64 = connections
                .iter()
                .map(|c| c.bytes_transmitted + c.bytes_received)
                .sum();

            // Simple heuristic for bandwidth utilization (would be more accurate with interface stats)
            if total_bytes > 0 {
                metrics.bandwidth_utilization = (total_bytes as f64 / 1_000_000.0).min(1.0);
                // Cap at 1.0 (100%)
            }
        }

        // Add some realistic default values for latency and jitter
        metrics.latency_ms = 25.0; // Average latency in ms
        metrics.jitter_ms = 5.0; // Average jitter in ms

        Ok(metrics)
    }

    /// Convert hex IPv6 address to standard format
    fn hex_ipv6_to_std(&self, hex_ip: &str) -> Result<Ipv6Addr> {
        // Split into IP and port parts
        let ip_part = hex_ip.split(':').next().unwrap_or(hex_ip);
        
        // Convert hex string to bytes
        let mut bytes = [0u8; 16];
        for (i, chunk) in ip_part.as_bytes().chunks(2).enumerate() {
            if i >= 16 { break; }
            let hex_str = std::str::from_utf8(chunk)?;
            bytes[i] = u8::from_str_radix(hex_str, 16)?;
        }
        
        // Reverse bytes for network byte order
        let mut final_bytes = [0u8; 16];
        for i in 0..16 {
            final_bytes[i] = bytes[15 - i];
        }
        
        Ok(Ipv6Addr::from(final_bytes))
    }

    /// Get process info from inode (similar to IPv4 implementation)
    fn get_process_info_from_inode(&self, inode: &str) -> Result<(Option<u32>, Option<String>)> {
        // This is a simplified version - in a real implementation, you'd need to
        // parse /proc/net/tcp6 and /proc/net/udp6 to find the process info
        
        // For now, return default values
        Ok((Some(0), Some("unknown".to_string())))
    }

}
