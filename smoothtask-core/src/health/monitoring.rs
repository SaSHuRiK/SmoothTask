//! Модуль мониторинга здоровья в реальном времени.
//!
//! Этот модуль предоставляет функции для непрерывного мониторинга
//! здоровья демона и автоматического обнаружения проблем.

use super::*;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Структура для хранения состояния мониторинга в реальном времени.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HealthMonitoringState {
    /// Текущее состояние здоровья
    pub current_health: HealthMonitor,
    /// История состояний здоровья
    pub health_history: Vec<HealthMonitor>,
    /// Максимальное количество хранимых состояний
    pub max_history_size: usize,
    /// Время последнего уведомления
    pub last_notification_time: Option<DateTime<Utc>>,
}

/// Интерфейс для мониторинга здоровья в реальном времени.
#[async_trait::async_trait]
pub trait HealthMonitoringService: Send + Sync {
    /// Запустить службу мониторинга здоровья.
    async fn start_monitoring(&self) -> Result<()>;
    
    /// Остановить службу мониторинга здоровья.
    async fn stop_monitoring(&self) -> Result<()>;
    
    /// Получить текущее состояние мониторинга.
    async fn get_monitoring_state(&self) -> Result<HealthMonitoringState>;
    
    /// Обновить конфигурацию мониторинга.
    async fn update_monitoring_config(&self, config: HealthMonitorConfig) -> Result<()>;
    
    /// Добавить обработчик событий здоровья.
    async fn add_health_event_handler(&self, handler: Box<dyn HealthEventHandler>) -> Result<()>;
}

/// Обработчик событий здоровья.
#[async_trait::async_trait]
pub trait HealthEventHandler: Send + Sync {
    /// Обработать событие изменения состояния здоровья.
    async fn handle_health_event(&self, event: HealthEvent) -> Result<()>;
}

/// Событие здоровья.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthEvent {
    /// Состояние здоровья изменилось
    HealthStatusChanged {
        old_status: HealthStatus,
        new_status: HealthStatus,
        timestamp: DateTime<Utc>,
    },
    /// Добавлена новая проблема
    NewHealthIssue {
        issue: HealthIssue,
        timestamp: DateTime<Utc>,
    },
    /// Проблема решена
    HealthIssueResolved {
        issue_id: String,
        timestamp: DateTime<Utc>,
    },
    /// Критическое состояние обнаружено
    CriticalHealthDetected {
        issue: HealthIssue,
        timestamp: DateTime<Utc>,
    },
}

/// Реализация HealthMonitoringService.
#[derive(Clone)]
pub struct HealthMonitoringServiceImpl {
    health_monitor: HealthMonitorImpl,
    monitoring_state: Arc<RwLock<HealthMonitoringState>>,
    event_handlers: Arc<RwLock<Vec<Box<dyn HealthEventHandler>>>>,
    is_running: Arc<RwLock<bool>>,
}

#[async_trait::async_trait]
impl HealthMonitoringService for HealthMonitoringServiceImpl {
    async fn start_monitoring(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Ok(()); // Уже запущено
        }
        
        *is_running = true;
        drop(is_running);
        
        // Запускаем цикл мониторинга в фоне
        let health_monitor = self.health_monitor.clone();
        let monitoring_state = self.monitoring_state.clone();
        let event_handlers = self.event_handlers.clone();
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            let config = health_monitor.health_state.read().await.config.clone();
            
            while *is_running.read().await {
                // Выполняем проверку здоровья
                match health_monitor.check_health().await {
                    Ok(health_status) => {
                        // Обновляем состояние мониторинга
                        let mut state = monitoring_state.write().await;
                        
                        // Сохраняем текущее состояние для сравнения
                        let old_status = state.current_health.overall_status;
                        
                        // Обновляем текущее состояние
                        state.current_health = health_status.clone();
                        
                        // Добавляем в историю
                        state.health_history.push(health_status.clone());
                        if state.health_history.len() > state.max_history_size {
                            state.health_history.remove(0);
                        }
                        
                        // Обрабатываем события
                        if old_status != state.current_health.overall_status {
                            let event = HealthEvent::HealthStatusChanged {
                                old_status,
                                new_status: state.current_health.overall_status,
                                timestamp: Utc::now(),
                            };
                            
                            // Логируем изменение состояния
                            match state.current_health.overall_status {
                                HealthStatus::Critical => {
                                    error!("Health status changed to CRITICAL: {:?} -> {:?}", old_status, state.current_health.overall_status);
                                }
                                HealthStatus::Degraded => {
                                    warn!("Health status changed to DEGRADED: {:?} -> {:?}", old_status, state.current_health.overall_status);
                                }
                                HealthStatus::Warning => {
                                    warn!("Health status changed to WARNING: {:?} -> {:?}", old_status, state.current_health.overall_status);
                                }
                                HealthStatus::Healthy => {
                                    info!("Health status changed to HEALTHY: {:?} -> {:?}", old_status, state.current_health.overall_status);
                                }
                                HealthStatus::Unknown => {
                                    warn!("Health status changed to UNKNOWN: {:?} -> {:?}", old_status, state.current_health.overall_status);
                                }
                            }
                            
                            // Уведомляем обработчики
                            for handler in event_handlers.read().await.iter() {
                                if let Err(e) = handler.handle_health_event(event.clone()).await {
                                    error!("Failed to handle health event: {}", e);
                                }
                            }
                        }
                        
                        // Проверяем на критическое состояние
                        if state.current_health.overall_status == HealthStatus::Critical {
                            let issue = HealthIssue {
                                issue_id: uuid::Uuid::new_v4().to_string(),
                                timestamp: Utc::now(),
                                issue_type: HealthIssueType::ComponentFailure,
                                severity: HealthIssueSeverity::Critical,
                                component: None,
                                description: "Critical health status detected".to_string(),
                                error_details: None,
                                status: HealthIssueStatus::Open,
                                resolved_time: None,
                            };
                            
                            let event = HealthEvent::CriticalHealthDetected {
                                issue: issue.clone(),
                                timestamp: Utc::now(),
                            };
                            
                            // Логируем критическое состояние
                            error!("CRITICAL HEALTH ISSUE DETECTED: {}", issue.description);
                            
                            // Уведомляем обработчики
                            for handler in event_handlers.read().await.iter() {
                                if let Err(e) = handler.handle_health_event(event.clone()).await {
                                    error!("Failed to handle critical health event: {}", e);
                                }
                            }
                            
                            // Добавляем проблему в историю
                            health_monitor.add_health_issue(issue).await.ok();
                        }
                    }
                    Err(e) => {
                        error!("Failed to check health: {}", e);
                    }
                }
                
                // Ждем перед следующей проверкой
                tokio::time::sleep(config.check_interval).await;
            }
        });
        
        Ok(())
    }
    
    async fn stop_monitoring(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        Ok(())
    }
    
    async fn get_monitoring_state(&self) -> Result<HealthMonitoringState> {
        Ok(self.monitoring_state.read().await.clone())
    }
    
    async fn update_monitoring_config(&self, config: HealthMonitorConfig) -> Result<()> {
        let mut health_state = self.health_monitor.health_state.write().await;
        health_state.config = config;
        Ok(())
    }
    
    async fn add_health_event_handler(&self, handler: Box<dyn HealthEventHandler>) -> Result<()> {
        let mut handlers = self.event_handlers.write().await;
        handlers.push(handler);
        Ok(())
    }
}

impl HealthMonitoringServiceImpl {
    /// Создать новый HealthMonitoringServiceImpl.
    pub fn new(health_monitor: HealthMonitorImpl) -> Self {
        Self {
            health_monitor,
            monitoring_state: Arc::new(RwLock::new(HealthMonitoringState {
                current_health: HealthMonitor::default(),
                health_history: Vec::new(),
                max_history_size: 10,
                last_notification_time: None,
            })),
            event_handlers: Arc::new(RwLock::new(Vec::new())),
            is_running: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Создать новый HealthMonitoringServiceImpl с HealthMonitor по умолчанию.
    pub fn new_default() -> Self {
        Self::new(create_default_health_monitor())
    }
}

/// Вспомогательная функция для создания HealthMonitoringService.
pub fn create_health_monitoring_service(health_monitor: HealthMonitorImpl) -> HealthMonitoringServiceImpl {
    HealthMonitoringServiceImpl::new(health_monitor)
}

/// Вспомогательная функция для создания HealthMonitoringService с HealthMonitor по умолчанию.
pub fn create_default_health_monitoring_service() -> HealthMonitoringServiceImpl {
    HealthMonitoringServiceImpl::new_default()
}