//! Модуль уведомлений о проблемах здоровья.
//!
//! Этот модуль предоставляет функции для отправки уведомлений
//! о проблемах здоровья демона.

use super::*;
use anyhow::{Context, Result};
use std::sync::Arc;

/// Интерфейс для отправки уведомлений о здоровье.
#[async_trait::async_trait]
pub trait HealthNotificationService: Send + Sync {
    /// Отправить уведомление о проблеме здоровья.
    async fn send_health_notification(&self, issue: &HealthIssue) -> Result<()>;
    
    /// Отправить уведомление о изменении состояния здоровья.
    async fn send_health_status_change_notification(&self, old_status: HealthStatus, new_status: HealthStatus) -> Result<()>;
    
    /// Отправить уведомление о критическом состоянии.
    async fn send_critical_health_notification(&self, issue: &HealthIssue) -> Result<()>;
    
    /// Настроить параметры уведомлений.
    async fn configure_notifications(&self, config: NotificationSettings) -> Result<()>;
}

/// Реализация HealthNotificationService.
#[derive(Debug, Clone)]
pub struct HealthNotificationServiceImpl {
    notification_settings: Arc<tokio::sync::RwLock<NotificationSettings>>,
    last_notification_time: Arc<tokio::sync::RwLock<Option<DateTime<Utc>>>>,
}

#[async_trait::async_trait]
impl HealthNotificationService for HealthNotificationServiceImpl {
    async fn send_health_notification(&self, issue: &HealthIssue) -> Result<()> {
        let settings = self.notification_settings.read().await;
        let mut last_time = self.last_notification_time.write().await;
        
        // Проверяем частоту уведомлений
        if let Some(last_time_value) = *last_time {
            let duration_since_last = Utc::now().signed_duration_since(last_time_value);
            if (duration_since_last.num_seconds() as u64) < settings.max_notification_frequency_seconds {
                info!("Skipping notification due to frequency limit");
                return Ok(());
            }
        }
        
        // Проверяем тип уведомления
        match issue.severity {
            HealthIssueSeverity::Critical => {
                if !settings.enable_critical_notifications {
                    return Ok(());
                }
            }
            HealthIssueSeverity::Warning => {
                if !settings.enable_warning_notifications {
                    return Ok(());
                }
            }
            _ => {}
        }
        
        // Отправляем уведомление
        info!("Sending health notification: {} - {}", issue.severity, issue.description);
        
        // Обновляем время последнего уведомления
        *last_time = Some(Utc::now());
        
        Ok(())
    }
    
    async fn send_health_status_change_notification(&self, old_status: HealthStatus, new_status: HealthStatus) -> Result<()> {
        let settings = self.notification_settings.read().await;
        let mut last_time = self.last_notification_time.write().await;
        
        // Проверяем частоту уведомлений
        if let Some(last_time_value) = *last_time {
            let duration_since_last = Utc::now().signed_duration_since(last_time_value);
            if (duration_since_last.num_seconds() as u64) < settings.max_notification_frequency_seconds {
                info!("Skipping status change notification due to frequency limit");
                return Ok(());
            }
        }
        
        // Отправляем уведомление
        info!("Health status changed from {:?} to {:?}", old_status, new_status);
        
        // Обновляем время последнего уведомления
        *last_time = Some(Utc::now());
        
        Ok(())
    }
    
    async fn send_critical_health_notification(&self, issue: &HealthIssue) -> Result<()> {
        let settings = self.notification_settings.read().await;
        let mut last_time = self.last_notification_time.write().await;
        
        // Проверяем частоту уведомлений
        if let Some(last_time_value) = *last_time {
            let duration_since_last = Utc::now().signed_duration_since(last_time_value);
            if (duration_since_last.num_seconds() as u64) < settings.max_notification_frequency_seconds {
                info!("Skipping critical notification due to frequency limit");
                return Ok(());
            }
        }
        
        // Отправляем уведомление
        error!("CRITICAL HEALTH ISSUE: {} - {}", issue.issue_type, issue.description);
        
        // Обновляем время последнего уведомления
        *last_time = Some(Utc::now());
        
        Ok(())
    }
    
    async fn configure_notifications(&self, config: NotificationSettings) -> Result<()> {
        let mut settings = self.notification_settings.write().await;
        *settings = config;
        Ok(())
    }
}

impl HealthNotificationServiceImpl {
    /// Создать новый HealthNotificationServiceImpl.
    pub fn new(settings: NotificationSettings) -> Self {
        Self {
            notification_settings: Arc::new(tokio::sync::RwLock::new(settings)),
            last_notification_time: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }
    
    /// Создать новый HealthNotificationServiceImpl с настройками по умолчанию.
    pub fn new_default() -> Self {
        Self::new(NotificationSettings::default())
    }
}

/// Вспомогательная функция для создания HealthNotificationService.
pub fn create_health_notification_service(settings: NotificationSettings) -> HealthNotificationServiceImpl {
    HealthNotificationServiceImpl::new(settings)
}

/// Вспомогательная функция для создания HealthNotificationService с настройками по умолчанию.
pub fn create_default_health_notification_service() -> HealthNotificationServiceImpl {
    HealthNotificationServiceImpl::new_default()
}

/// Обработчик событий здоровья для уведомлений.
#[derive(Debug, Clone)]
pub struct NotificationHealthEventHandler {
    notification_service: HealthNotificationServiceImpl,
}

#[async_trait::async_trait]
impl HealthEventHandler for NotificationHealthEventHandler {
    async fn handle_health_event(&self, event: HealthEvent) -> Result<()> {
        match event {
            HealthEvent::HealthStatusChanged { old_status, new_status, .. } => {
                self.notification_service.send_health_status_change_notification(old_status, new_status).await?;
            }
            HealthEvent::NewHealthIssue { issue, .. } => {
                self.notification_service.send_health_notification(&issue).await?;
            }
            HealthEvent::HealthIssueResolved { .. } => {
                info!("Health issue resolved");
            }
            HealthEvent::CriticalHealthDetected { issue, .. } => {
                self.notification_service.send_critical_health_notification(&issue).await?;
            }
        }
        
        Ok(())
    }
}

impl NotificationHealthEventHandler {
    /// Создать новый NotificationHealthEventHandler.
    pub fn new(notification_service: HealthNotificationServiceImpl) -> Self {
        Self { notification_service }
    }
    
    /// Создать новый NotificationHealthEventHandler с настройками по умолчанию.
    pub fn new_default() -> Self {
        Self::new(create_default_health_notification_service())
    }
}

/// Вспомогательная функция для создания NotificationHealthEventHandler.
pub fn create_notification_health_event_handler(notification_service: HealthNotificationServiceImpl) -> NotificationHealthEventHandler {
    NotificationHealthEventHandler::new(notification_service)
}

/// Вспомогательная функция для создания NotificationHealthEventHandler с настройками по умолчанию.
pub fn create_default_notification_health_event_handler() -> NotificationHealthEventHandler {
    NotificationHealthEventHandler::new_default()
}