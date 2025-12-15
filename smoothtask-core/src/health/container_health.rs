//! Модуль мониторинга здоровья контейнеров.
//!
//! Этот модуль предоставляет функции для мониторинга здоровья контейнеров Docker и Podman,
//! включая проверку статуса, мониторинг ресурсов и обнаружение проблем.

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Информация о здоровье контейнера.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainerHealthInfo {
    /// Уникальный идентификатор контейнера
    pub container_id: String,
    /// Имя контейнера
    pub container_name: String,
    /// Состояние здоровья контейнера
    pub health_status: ContainerHealthStatus,
    /// Время последней проверки
    pub last_check_time: DateTime<Utc>,
    /// Сообщение о состоянии
    pub status_message: Option<String>,
    /// Детали ошибки (если есть)
    pub error_details: Option<String>,
    /// Использование CPU (в процентах)
    pub cpu_usage_percent: Option<f32>,
    /// Использование памяти (в байтах)
    pub memory_usage_bytes: Option<u64>,
    /// Использование памяти (в процентах)
    pub memory_usage_percent: Option<f32>,
    /// Состояние сети
    pub network_status: Option<ContainerNetworkStatus>,
    /// Состояние диска
    pub disk_status: Option<ContainerDiskStatus>,
}

impl Default for ContainerHealthInfo {
    fn default() -> Self {
        Self {
            container_id: String::new(),
            container_name: String::new(),
            health_status: ContainerHealthStatus::Unknown,
            last_check_time: Utc::now(),
            status_message: None,
            error_details: None,
            cpu_usage_percent: None,
            memory_usage_bytes: None,
            memory_usage_percent: None,
            network_status: None,
            disk_status: None,
        }
    }
}

/// Состояние здоровья контейнера.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerHealthStatus {
    /// Контейнер работает нормально
    #[serde(rename = "healthy")]
    Healthy,
    /// Контейнер работает, но есть предупреждения
    #[serde(rename = "warning")]
    Warning,
    /// Контейнер нездоров
    #[serde(rename = "unhealthy")]
    Unhealthy,
    /// Контейнер остановлен
    #[serde(rename = "stopped")]
    Stopped,
    /// Состояние неизвестно
    #[serde(rename = "unknown")]
    Unknown,
}

impl Default for ContainerHealthStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Состояние сети контейнера.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainerNetworkStatus {
    /// Количество активных сетевых соединений
    pub active_connections: usize,
    /// Входящий трафик (байт/с)
    pub incoming_traffic_bps: Option<u64>,
    /// Исходящий трафик (байт/с)
    pub outgoing_traffic_bps: Option<u64>,
    /// Состояние сети
    pub network_health: ContainerNetworkHealth,
}

/// Состояние сети контейнера.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerNetworkHealth {
    /// Сеть работает нормально
    #[serde(rename = "healthy")]
    Healthy,
    /// Проблемы с сетью
    #[serde(rename = "degraded")]
    Degraded,
    /// Сеть недоступна
    #[serde(rename = "unavailable")]
    Unavailable,
    /// Состояние неизвестно
    #[serde(rename = "unknown")]
    Unknown,
}

/// Состояние диска контейнера.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainerDiskStatus {
    /// Использование диска (байт)
    pub disk_usage_bytes: Option<u64>,
    /// Использование диска (процент)
    pub disk_usage_percent: Option<f32>,
    /// Скорость чтения (байт/с)
    pub read_speed_bps: Option<u64>,
    /// Скорость записи (байт/с)
    pub write_speed_bps: Option<u64>,
    /// Состояние диска
    pub disk_health: ContainerDiskHealth,
}

/// Состояние диска контейнера.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerDiskHealth {
    /// Диск работает нормально
    #[serde(rename = "healthy")]
    Healthy,
    /// Диск почти полон
    #[serde(rename = "warning")]
    Warning,
    /// Проблемы с диском
    #[serde(rename = "degraded")]
    Degraded,
    /// Диск недоступен
    #[serde(rename = "unavailable")]
    Unavailable,
    /// Состояние неизвестно
    #[serde(rename = "unknown")]
    Unknown,
}

/// Конфигурация мониторинга здоровья контейнеров.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainerHealthConfig {
    /// Включить мониторинг контейнеров
    pub enable_container_monitoring: bool,
    /// Интервал проверки контейнеров
    pub check_interval: Duration,
    /// Пороги для определения критических состояний
    pub critical_thresholds: ContainerCriticalThresholds,
    /// Список контейнеров для мониторинга (пустой список = все контейнеры)
    pub monitored_containers: Vec<String>,
    /// Список игнорируемых контейнеров
    pub ignored_containers: Vec<String>,
}

impl Default for ContainerHealthConfig {
    fn default() -> Self {
        Self {
            enable_container_monitoring: true,
            check_interval: Duration::from_secs(60),
            critical_thresholds: ContainerCriticalThresholds::default(),
            monitored_containers: Vec::new(),
            ignored_containers: Vec::new(),
        }
    }
}

/// Пороги для определения критических состояний контейнеров.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainerCriticalThresholds {
    /// Максимальное использование CPU (в процентах)
    pub max_cpu_usage_percent: f32,
    /// Максимальное использование памяти (в процентах)
    pub max_memory_usage_percent: f32,
    /// Максимальное использование диска (в процентах)
    pub max_disk_usage_percent: f32,
    /// Максимальное количество последовательных ошибок
    pub max_consecutive_errors: usize,
    /// Максимальное время бездействия контейнера (в секундах)
    pub max_idle_time_seconds: u64,
}

impl Default for ContainerCriticalThresholds {
    fn default() -> Self {
        Self {
            max_cpu_usage_percent: 90.0,
            max_memory_usage_percent: 85.0,
            max_disk_usage_percent: 90.0,
            max_consecutive_errors: 5,
            max_idle_time_seconds: 300, // 5 минут
        }
    }
}

/// Основной интерфейс для мониторинга здоровья контейнеров.
#[async_trait::async_trait]
pub trait ContainerHealthMonitorTrait: Send + Sync {
    /// Выполнить проверку здоровья контейнеров.
    async fn check_container_health(&self) -> Result<HashMap<String, ContainerHealthInfo>>;

    /// Получить информацию о здоровье конкретного контейнера.
    async fn get_container_health(&self, container_id: &str) -> Result<Option<ContainerHealthInfo>>;

    /// Обновить конфигурацию мониторинга контейнеров.
    async fn update_container_health_config(&self, config: ContainerHealthConfig) -> Result<()>;

    /// Получить текущую конфигурацию.
    async fn get_container_health_config(&self) -> Result<ContainerHealthConfig>;

    /// Добавить контейнер в список мониторинга.
    async fn add_monitored_container(&self, container_id: &str) -> Result<()>;

    /// Удалить контейнер из списка мониторинга.
    async fn remove_monitored_container(&self, container_id: &str) -> Result<()>;

    /// Добавить контейнер в список игнорируемых.
    async fn add_ignored_container(&self, container_id: &str) -> Result<()>;

    /// Удалить контейнер из списка игнорируемых.
    async fn remove_ignored_container(&self, container_id: &str) -> Result<()>;
}

/// Реализация ContainerHealthMonitorTrait.
#[derive(Debug, Clone)]
pub struct ContainerHealthMonitorImpl {
    config: ContainerHealthConfig,
    container_health_cache: Arc<tokio::sync::RwLock<HashMap<String, ContainerHealthInfo>>>, 
}

use std::sync::Arc;

#[async_trait::async_trait]
impl ContainerHealthMonitorTrait for ContainerHealthMonitorImpl {
    async fn check_container_health(&self) -> Result<HashMap<String, ContainerHealthInfo>> {
        let mut health_info_map = HashMap::new();

        // Получаем список всех контейнеров
        let containers = self.get_container_list().await?;

        // Проверяем здоровье каждого контейнера
        for container in containers {
            let container_id = container.id.clone();
            let container_name = container.name.clone();

            // Проверяем, нужно ли мониторить этот контейнер
            if self.should_monitor_container(&container_id, &container_name) {
                let health_info = self.check_single_container_health(&container).await?;
                health_info_map.insert(container_id.clone(), health_info);
            }
        }

        // Обновляем кэш
        let mut cache = self.container_health_cache.write().await;
        *cache = health_info_map.clone();

        Ok(health_info_map)
    }

    async fn get_container_health(&self, container_id: &str) -> Result<Option<ContainerHealthInfo>> {
        // Сначала пытаемся получить из кэша
        let cache = self.container_health_cache.read().await;
        if let Some(health_info) = cache.get(container_id) {
            return Ok(Some(health_info.clone()));
        }

        // Если нет в кэше, получаем свежую информацию
        let containers = self.get_container_list().await?;
        for container in containers {
            if container.id == container_id {
                let health_info = self.check_single_container_health(&container).await?;
                return Ok(Some(health_info));
            }
        }

        Ok(None)
    }

    async fn update_container_health_config(&self, config: ContainerHealthConfig) -> Result<()> {
        // В реальной реализации нужно обновить конфигурацию
        // Для простоты просто возвращаем Ok
        Ok(())
    }

    async fn get_container_health_config(&self) -> Result<ContainerHealthConfig> {
        Ok(self.config.clone())
    }

    async fn add_monitored_container(&self, container_id: &str) -> Result<()> {
        // В реальной реализации нужно добавить контейнер в список мониторинга
        Ok(())
    }

    async fn remove_monitored_container(&self, container_id: &str) -> Result<()> {
        // В реальной реализации нужно удалить контейнер из списка мониторинга
        Ok(())
    }

    async fn add_ignored_container(&self, container_id: &str) -> Result<()> {
        // В реальной реализации нужно добавить контейнер в список игнорируемых
        Ok(())
    }

    async fn remove_ignored_container(&self, container_id: &str) -> Result<()> {
        // В реальной реализации нужно удалить контейнер из списка игнорируемых
        Ok(())
    }
}

impl ContainerHealthMonitorImpl {
    /// Создать новый ContainerHealthMonitorImpl.
    pub fn new(config: ContainerHealthConfig) -> Self {
        Self {
            config,
            container_health_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Создать новый ContainerHealthMonitorImpl с конфигурацией по умолчанию.
    pub fn new_default() -> Self {
        Self::new(ContainerHealthConfig::default())
    }

    /// Получить список всех контейнеров.
    async fn get_container_list(&self) -> Result<Vec<ContainerInfo>> {
        let mut containers = Vec::new();

        // Пробуем получить информацию о контейнерах с помощью Docker
        if let Ok(docker_containers) = self.get_docker_containers().await {
            containers.extend(docker_containers);
        }

        // Пробуем получить информацию о контейнерах с помощью Podman
        if let Ok(podman_containers) = self.get_podman_containers().await {
            containers.extend(podman_containers);
        }

        Ok(containers)
    }

    /// Получить информацию о контейнерах Docker.
    async fn get_docker_containers(&self) -> Result<Vec<ContainerInfo>> {
        let output = Command::new("docker")
            .args(["ps", "--format", "{{.ID}}:{{.Names}}:{{.Status}}"])
            .output()
            .context("Failed to execute docker command")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Docker command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let stdout = String::from_utf8(output.stdout)?;
        let mut containers = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                containers.push(ContainerInfo {
                    id: parts[0].to_string(),
                    name: parts[1].to_string(),
                    status: parts[2].to_string(),
                });
            }
        }

        Ok(containers)
    }

    /// Получить информацию о контейнерах Podman.
    async fn get_podman_containers(&self) -> Result<Vec<ContainerInfo>> {
        let output = Command::new("podman")
            .args(["ps", "--format", "{{.ID}}:{{.Names}}:{{.Status}}"])
            .output()
            .context("Failed to execute podman command")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Podman command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let stdout = String::from_utf8(output.stdout)?;
        let mut containers = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                containers.push(ContainerInfo {
                    id: parts[0].to_string(),
                    name: parts[1].to_string(),
                    status: parts[2].to_string(),
                });
            }
        }

        Ok(containers)
    }

    /// Проверить, нужно ли мониторить контейнер.
    fn should_monitor_container(&self, container_id: &str, container_name: &str) -> bool {
        // Проверяем, включен ли мониторинг контейнеров
        if !self.config.enable_container_monitoring {
            return false;
        }

        // Проверяем, не игнорируется ли контейнер
        if self.config.ignored_containers.contains(&container_id.to_string())
            || self.config.ignored_containers.contains(&container_name.to_string())
        {
            return false;
        }

        // Проверяем, входит ли контейнер в список мониторинга
        if !self.config.monitored_containers.is_empty()
            && !self.config.monitored_containers.contains(&container_id.to_string())
            && !self.config.monitored_containers.contains(&container_name.to_string())
        {
            return false;
        }

        true
    }

    /// Проверить здоровье одного контейнера.
    async fn check_single_container_health(&self, container: &ContainerInfo) -> Result<ContainerHealthInfo> {
        let mut health_info = ContainerHealthInfo {
            container_id: container.id.clone(),
            container_name: container.name.clone(),
            health_status: ContainerHealthStatus::Unknown,
            last_check_time: Utc::now(),
            status_message: Some(format!("Container status: {}", container.status)),
            error_details: None,
            cpu_usage_percent: None,
            memory_usage_bytes: None,
            memory_usage_percent: None,
            network_status: None,
            disk_status: None,
        };

        // Определяем состояние контейнера на основе статуса
        health_info.health_status = self.determine_container_status(&container.status);

        // Получаем статистику контейнера
        if let Ok(stats) = self.get_container_stats(&container.id).await {
            health_info.cpu_usage_percent = stats.cpu_usage_percent;
            health_info.memory_usage_bytes = stats.memory_usage_bytes;
            health_info.memory_usage_percent = stats.memory_usage_percent;

            // Проверяем пороги
            self.check_thresholds(&mut health_info);
        }

        Ok(health_info)
    }

    /// Определить состояние контейнера на основе статуса.
    fn determine_container_status(&self, status: &str) -> ContainerHealthStatus {
        let status_lower = status.to_lowercase();

        if status_lower.contains("up") || status_lower.contains("running") {
            ContainerHealthStatus::Healthy
        } else if status_lower.contains("exited") || status_lower.contains("stopped") {
            ContainerHealthStatus::Stopped
        } else if status_lower.contains("unhealthy") || status_lower.contains("dead") {
            ContainerHealthStatus::Unhealthy
        } else {
            ContainerHealthStatus::Unknown
        }
    }

    /// Получить статистику контейнера.
    async fn get_container_stats(&self, container_id: &str) -> Result<ContainerStats> {
        // Пробуем получить статистику с помощью Docker
        if let Ok(stats) = self.get_docker_container_stats(container_id).await {
            return Ok(stats);
        }

        // Пробуем получить статистику с помощью Podman
        if let Ok(stats) = self.get_podman_container_stats(container_id).await {
            return Ok(stats);
        }

        Err(anyhow::anyhow!("Failed to get container stats for {}", container_id))
    }

    /// Получить статистику контейнера Docker.
    async fn get_docker_container_stats(&self, container_id: &str) -> Result<ContainerStats> {
        let output = Command::new("docker")
            .args(["stats", "--no-stream", "--format", "{{.CPUPerc}}:{{.MemUsage}}:{{.MemPerc}}", container_id])
            .output()
            .context("Failed to execute docker stats command")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Docker stats command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let stdout = String::from_utf8(output.stdout)?;
        let parts: Vec<&str> = stdout.trim().split(':').collect();

        if parts.len() >= 3 {
            let cpu_usage = parts[0].trim_end_matches('%').parse::<f32>().ok();
            let mem_usage = parts[1].split('/').next().and_then(|s| {
                s.trim().parse::<u64>().ok()
            });
            let mem_percent = parts[2].trim_end_matches('%').parse::<f32>().ok();

            return Ok(ContainerStats {
                cpu_usage_percent: cpu_usage,
                memory_usage_bytes: mem_usage,
                memory_usage_percent: mem_percent,
            });
        }

        Err(anyhow::anyhow!("Failed to parse docker stats output"))
    }

    /// Получить статистику контейнера Podman.
    async fn get_podman_container_stats(&self, container_id: &str) -> Result<ContainerStats> {
        let output = Command::new("podman")
            .args(["stats", "--no-stream", "--format", "{{.CPUPerc}}:{{.MemUsage}}:{{.MemPerc}}", container_id])
            .output()
            .context("Failed to execute podman stats command")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Podman stats command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let stdout = String::from_utf8(output.stdout)?;
        let parts: Vec<&str> = stdout.trim().split(':').collect();

        if parts.len() >= 3 {
            let cpu_usage = parts[0].trim_end_matches('%').parse::<f32>().ok();
            let mem_usage = parts[1].split('/').next().and_then(|s| {
                s.trim().parse::<u64>().ok()
            });
            let mem_percent = parts[2].trim_end_matches('%').parse::<f32>().ok();

            return Ok(ContainerStats {
                cpu_usage_percent: cpu_usage,
                memory_usage_bytes: mem_usage,
                memory_usage_percent: mem_percent,
            });
        }

        Err(anyhow::anyhow!("Failed to parse podman stats output"))
    }

    /// Проверить пороги и обновить состояние здоровья.
    fn check_thresholds(&self, health_info: &mut ContainerHealthInfo) {
        // Проверяем использование CPU
        if let Some(cpu_usage) = health_info.cpu_usage_percent {
            if cpu_usage > self.config.critical_thresholds.max_cpu_usage_percent {
                health_info.health_status = ContainerHealthStatus::Unhealthy;
                health_info.error_details = Some(format!(
                    "High CPU usage: {:.1}% (threshold: {:.1}%)",
                    cpu_usage, self.config.critical_thresholds.max_cpu_usage_percent
                ));
            } else if cpu_usage > 80.0 && health_info.health_status == ContainerHealthStatus::Healthy {
                health_info.health_status = ContainerHealthStatus::Warning;
            }
        }

        // Проверяем использование памяти
        if let Some(mem_usage) = health_info.memory_usage_percent {
            if mem_usage > self.config.critical_thresholds.max_memory_usage_percent {
                health_info.health_status = ContainerHealthStatus::Unhealthy;
                health_info.error_details = Some(format!(
                    "High memory usage: {:.1}% (threshold: {:.1}%)",
                    mem_usage, self.config.critical_thresholds.max_memory_usage_percent
                ));
            } else if mem_usage > 75.0 && health_info.health_status == ContainerHealthStatus::Healthy {
                health_info.health_status = ContainerHealthStatus::Warning;
            }
        }
    }
}

/// Информация о контейнере.
#[derive(Debug, Clone)]
struct ContainerInfo {
    id: String,
    name: String,
    status: String,
}

/// Статистика контейнера.
#[derive(Debug, Clone)]
struct ContainerStats {
    cpu_usage_percent: Option<f32>,
    memory_usage_bytes: Option<u64>,
    memory_usage_percent: Option<f32>,
}

/// Вспомогательная функция для создания ContainerHealthMonitor.
pub fn create_container_health_monitor(config: ContainerHealthConfig) -> ContainerHealthMonitorImpl {
    ContainerHealthMonitorImpl::new(config)
}

/// Вспомогательная функция для создания ContainerHealthMonitor с конфигурацией по умолчанию.
pub fn create_default_container_health_monitor() -> ContainerHealthMonitorImpl {
    ContainerHealthMonitorImpl::new_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    #[tokio::test]
    async fn test_container_health_monitor_creation() {
        let config = ContainerHealthConfig::default();
        let monitor = ContainerHealthMonitorImpl::new(config);
        assert_eq!(monitor.config.enable_container_monitoring, true);
    }

    #[tokio::test]
    async fn test_container_health_monitor_default() {
        let monitor = ContainerHealthMonitorImpl::new_default();
        assert_eq!(monitor.config.enable_container_monitoring, true);
        assert_eq!(monitor.config.check_interval, Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_container_status_determination() {
        let config = ContainerHealthConfig::default();
        let monitor = ContainerHealthMonitorImpl::new(config);

        assert_eq!(
            monitor.determine_container_status("Up 2 hours"),
            ContainerHealthStatus::Healthy
        );
        assert_eq!(
            monitor.determine_container_status("Exited (0) 5 minutes ago"),
            ContainerHealthStatus::Stopped
        );
        assert_eq!(
            monitor.determine_container_status("Unhealthy"),
            ContainerHealthStatus::Unhealthy
        );
        assert_eq!(
            monitor.determine_container_status("Unknown status"),
            ContainerHealthStatus::Unknown
        );
    }

    #[tokio::test]
    async fn test_container_monitoring_decision() {
        let mut config = ContainerHealthConfig::default();
        config.enable_container_monitoring = false;
        let monitor = ContainerHealthMonitorImpl::new(config);

        assert_eq!(
            monitor.should_monitor_container("test123", "test_container"),
            false
        );

        config.enable_container_monitoring = true;
        config.ignored_containers = vec!["test123".to_string()];
        let monitor = ContainerHealthMonitorImpl::new(config);

        assert_eq!(
            monitor.should_monitor_container("test123", "test_container"),
            false
        );

        config.ignored_containers = vec![];
        config.monitored_containers = vec!["test123".to_string()];
        let monitor = ContainerHealthMonitorImpl::new(config);

        assert_eq!(
            monitor.should_monitor_container("test123", "test_container"),
            true
        );
        assert_eq!(
            monitor.should_monitor_container("other123", "other_container"),
            false
        );
    }

    #[tokio::test]
    async fn test_threshold_checking() {
        let config = ContainerHealthConfig::default();
        let monitor = ContainerHealthMonitorImpl::new(config);

        let mut health_info = ContainerHealthInfo {
            container_id: "test123".to_string(),
            container_name: "test_container".to_string(),
            health_status: ContainerHealthStatus::Healthy,
            last_check_time: Utc::now(),
            status_message: None,
            error_details: None,
            cpu_usage_percent: Some(95.0),
            memory_usage_bytes: None,
            memory_usage_percent: None,
            network_status: None,
            disk_status: None,
        };

        monitor.check_thresholds(&mut health_info);
        assert_eq!(health_info.health_status, ContainerHealthStatus::Unhealthy);
        assert!(health_info.error_details.is_some());

        let mut health_info = ContainerHealthInfo {
            cpu_usage_percent: None,
            memory_usage_percent: Some(95.0),
            ..health_info
        };

        monitor.check_thresholds(&mut health_info);
        assert_eq!(health_info.health_status, ContainerHealthStatus::Unhealthy);
        assert!(health_info.error_details.is_some());
    }
}

