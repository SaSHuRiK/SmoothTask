//! Модуль системы уведомлений.
//!
//! Предоставляет инфраструктуру для отправки уведомлений пользователю о важных событиях
//! в работе демона. Поддерживает различные бэкенды (заглушки, desktop уведомления и т.д.).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;

// Conditional import for libnotify
// libnotify support is temporarily disabled due to crate availability issues
// #[cfg(feature = "libnotify")]
// use libnotify::Notification as LibnotifyNotification;

#[cfg(feature = "dbus")]
use zbus::Connection;

#[cfg(feature = "dbus")]
use zbus::zvariant::Value;

/// Тип уведомления, определяющий его важность и визуальное представление.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NotificationType {
    /// Критическое уведомление - требует немедленного внимания.
    /// Используется для фатальных ошибок, которые могут повлиять на работу системы.
    Critical,
    
    /// Предупреждение - некритическая проблема, требующая внимания.
    /// Используется для предупреждений о потенциальных проблемах или неоптимальных состояниях.
    Warning,
    
    /// Информационное уведомление - общая информация о работе системы.
    /// Используется для уведомлений о нормальной работе, успешных операциях и т.д.
    Info,
}

impl fmt::Display for NotificationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotificationType::Critical => write!(f, "CRITICAL"),
            NotificationType::Warning => write!(f, "WARNING"),
            NotificationType::Info => write!(f, "INFO"),
        }
    }
}

/// Структура, представляющая уведомление.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// Тип уведомления (критическое, предупреждение, информационное).
    pub notification_type: NotificationType,
    
    /// Заголовок уведомления.
    pub title: String,
    
    /// Основное сообщение уведомления.
    pub message: String,
    
    /// Дополнительные детали (опционально).
    /// Может содержать техническую информацию, трассировку стека и т.д.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    
    /// Временная метка создания уведомления.
    #[serde(with = "chrono::serde::ts_seconds")]
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Notification {
    /// Создаёт новое уведомление с текущей временной меткой.
    pub fn new(
        notification_type: NotificationType,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            notification_type,
            title: title.into(),
            message: message.into(),
            details: None,
            timestamp: chrono::Utc::now(),
        }
    }
    
    /// Добавляет дополнительные детали к уведомлению.
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

/// Трейт для отправки уведомлений.
/// Реализации этого трейта могут отправлять уведомления через различные бэкенды.
#[async_trait::async_trait]
pub trait Notifier: Send + Sync + 'static {
    /// Отправляет уведомление.
    /// 
    /// # Аргументы
    /// * `notification` - Уведомление для отправки.
    /// 
    /// # Возвращает
    /// `Result<()>` - Ok, если уведомление успешно отправлено, иначе ошибка.
    async fn send_notification(&self, notification: &Notification) -> Result<()>;
    
    /// Возвращает имя бэкенда уведомлений (для логирования и отладки).
    fn backend_name(&self) -> &str;
}

/// Заглушка для уведомлений, используемая для тестирования и когда реальные уведомления не нужны.
/// Просто логирует уведомления через tracing, но не отправляет их.
#[derive(Debug, Default)]
pub struct StubNotifier;

#[async_trait::async_trait]
impl Notifier for StubNotifier {
    async fn send_notification(&self, notification: &Notification) -> Result<()> {
        match notification.notification_type {
            NotificationType::Critical => {
                tracing::error!(
                    "[NOTIFICATION] {}: {}",
                    notification.title,
                    notification.message
                );
            }
            NotificationType::Warning => {
                tracing::warn!(
                    "[NOTIFICATION] {}: {}",
                    notification.title,
                    notification.message
                );
            }
            NotificationType::Info => {
                tracing::info!(
                    "[NOTIFICATION] {}: {}",
                    notification.title,
                    notification.message
                );
            }
        }
        
        if let Some(details) = &notification.details {
            tracing::debug!("Notification details: {}", details);
        }
        
        Ok(())
    }
    
    fn backend_name(&self) -> &str {
        "stub"
    }
}

/// Реализация Notifier на основе libnotify для отправки desktop уведомлений.
/// Использует системную библиотеку libnotify для отображения уведомлений в desktop окружении.
/// 
/// Доступно только при включении фичи `libnotify`.
// #[cfg(feature = "libnotify")]
// #[derive(Debug, Default)]
// libnotify support is temporarily disabled due to crate availability issues
// #[cfg(feature = "libnotify")]
// pub struct LibnotifyNotifier {
//     /// Имя приложения для уведомлений.
//     app_name: String,
// }

// #[cfg(feature = "libnotify")]
// impl LibnotifyNotifier {
//     /// Создаёт новый LibnotifyNotifier с указанным именем приложения.
//     /// 
//     /// # Аргументы
//     /// * `app_name` - Имя приложения, которое будет отображаться в уведомлениях.
//     /// 
//     /// # Возвращает
//     /// Новый экземпляр LibnotifyNotifier.
//     pub fn new(app_name: impl Into<String>) -> Self {
//         Self {
//             app_name: app_name.into(),
//         }
//     }
//     
//     /// Инициализирует библиотеку libnotify.
//     /// 
//     /// # Возвращает
//     /// `Result<()>` - Ok, если инициализация прошла успешно, иначе ошибка.
//     pub fn init() -> Result<()> {
//         libnotify::init("SmoothTask")?;
//         Ok(())
//     }
// 
// // #[cfg(feature = "libnotify")]
// #[async_trait::async_trait]
// impl Notifier for LibnotifyNotifier {
//     async fn send_notification(&self, notification: &Notification) -> Result<()> {
//         // Создаём уведомление libnotify
//         let mut libnotify_notification = LibnotifyNotification::new(
//             &notification.title,
//             &notification.message,
//             None, // Иконка не указана
//         );
//         
//         // Устанавливаем имя приложения
//         libnotify_notification.set_app_name(&self.app_name);
//         
//         // Устанавливаем уровень срочности в зависимости от типа уведомления
//         let urgency = match notification.notification_type {
//             NotificationType::Critical => libnotify::Urgency::Critical,
//             NotificationType::Warning => libnotify::Urgency::Normal,
//             NotificationType::Info => libnotify::Urgency::Low,
//         };
//         libnotify_notification.set_urgency(urgency);
//         
//         // Добавляем дополнительные детали в тело уведомления, если они есть
//         if let Some(details) = &notification.details {
//             let mut body = notification.message.clone();
//             body.push_str("\n");
//             body.push_str(details);
//             libnotify_notification.set_body(&body);
// //
//         }
//         
//         // Отправляем уведомление
//         libnotify_notification.show()?;
//         
//         // Логируем отправку уведомления
//         tracing::info!(
//             "Sent desktop notification via libnotify: {} - {}",
//             notification.title,
//             notification.message
//         );
//         
//         Ok(())
//     }
//     
//     fn backend_name(&self) -> &str {
//         "libnotify"

/// Notifier на основе D-Bus для отправки уведомлений через системный D-Bus.
/// Использует стандартный протокол org.freedesktop.Notifications.
#[cfg(feature = "dbus")]
pub struct DBusNotifier {
    /// Имя приложения для уведомлений.
    app_name: String,
    /// Идентификатор соединения D-Bus.
    connection: Option<Connection>,
}

#[cfg(feature = "dbus")]
impl DBusNotifier {
    /// Создаёт новый DBusNotifier с указанным именем приложения.
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            connection: None,
        }
    }

    /// Устанавливает соединение с системным D-Bus.
    pub async fn connect(&mut self) -> Result<()> {
        self.connection = Some(Connection::system().await?);
        Ok(())
    }

    /// Проверяет, установлено ли соединение с D-Bus.
    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }
}

#[cfg(feature = "dbus")]
#[async_trait::async_trait]
impl Notifier for DBusNotifier {
    async fn send_notification(&self, notification: &Notification) -> Result<()> {
        // Проверяем, что соединение установлено
        let connection = match &self.connection {
            Some(conn) => conn,
            None => {
                tracing::warn!("D-Bus connection not established, cannot send notification");
                return Ok(());
            }
        };

        // Преобразуем тип уведомления в уровень срочности
        let urgency = match notification.notification_type {
            NotificationType::Critical => "critical",
            NotificationType::Warning => "normal",
            NotificationType::Info => "low",
        };

        // Формируем сообщение уведомления
        let mut body = notification.message.clone();
        if let Some(details) = &notification.details {
            body.push_str("\n");
            body.push_str(details);
        }

        // Реальная отправка уведомления через D-Bus
        // Используем стандартный интерфейс org.freedesktop.Notifications
        let proxy = zbus::Proxy::new(
            connection,
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
        )?;

        // Подготавливаем параметры для вызова метода Notify
        let app_name: &str = &self.app_name;
        let replaces_id: u32 = 0; // 0 означает новое уведомление
        let app_icon: &str = match notification.notification_type {
            NotificationType::Critical => "dialog-error",
            NotificationType::Warning => "dialog-warning",
            NotificationType::Info => "dialog-information",
        };
        let summary: &str = &notification.title;
        let body_str: &str = &body;
        let actions: Vec<&str> = vec![]; // Нет действий
        let hints: std::collections::HashMap<&str, zbus::zvariant::Value> = {
            let mut hints_map = std::collections::HashMap::new();
            // Устанавливаем уровень срочности
            hints_map.insert("urgency", zbus::zvariant::Value::new(urgency));
            hints_map
        };
        let expire_timeout: i32 = 5000; // 5 секунд

        // Отправляем уведомление через D-Bus
        let result: zbus::Result<u32> = proxy
            .call_method(
                "Notify",
                &(
                    app_name,
                    replaces_id,
                    app_icon,
                    summary,
                    body_str,
                    actions,
                    hints,
                    expire_timeout,
                ),
            )
            .await;

        match result {
            Ok(notification_id) => {
                tracing::info!(
                    "Successfully sent D-Bus notification (ID: {}): {} - {}",
                    notification_id,
                    notification.title,
                    notification.message
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    "Failed to send D-Bus notification: {}. Falling back to logging.",
                    e
                );
                // В случае ошибки, логируем уведомление как заглушка
                match notification.notification_type {
                    NotificationType::Critical => {
                        tracing::error!(
                            "[NOTIFICATION] {}: {}",
                            notification.title,
                            notification.message
                        );
                    }
                    NotificationType::Warning => {
                        tracing::warn!(
                            "[NOTIFICATION] {}: {}",
                            notification.title,
                            notification.message
                        );
                    }
                    NotificationType::Info => {
                        tracing::info!(
                            "[NOTIFICATION] {}: {}",
                            notification.title,
                            notification.message
                        );
                    }
                }
                if let Some(details) = &notification.details {
                    tracing::debug!("Notification details: {}", details);
                }
                Ok(())
            }
        }
    }

    fn backend_name(&self) -> &str {
        "dbus"
    }
}

/// Основной менеджер уведомлений, управляющий отправкой уведомлений через различные бэкенды.
pub struct NotificationManager {
    /// Основной бэкенд для отправки уведомлений.
    primary_notifier: Box<dyn Notifier>,
    
    /// Флаг, разрешающий отправку уведомлений.
    /// Если false, уведомления не отправляются (полезно для тестирования или тихого режима).
    enabled: bool,
    
    /// Опциональное хранилище логов для интеграции с системой логирования.
    /// Если указано, уведомления будут также логироваться в хранилище.
    pub log_storage: Option<std::sync::Arc<crate::logging::log_storage::SharedLogStorage>>,
}

impl NotificationManager {
    /// Создаёт новый NotificationManager с указанным бэкендом.
    pub fn new(notifier: impl Notifier) -> Self {
        Self {
            primary_notifier: Box::new(notifier),
            enabled: true,
            log_storage: None,
        }
    }
    
    /// Создаёт новый NotificationManager с заглушкой (для тестирования).
    pub fn new_stub() -> Self {
        Self::new(StubNotifier)
    }
    
    /// Создаёт новый NotificationManager с заглушкой и интеграцией с хранилищем логов.
    pub fn new_stub_with_logging(log_storage: std::sync::Arc<crate::logging::log_storage::SharedLogStorage>) -> Self {
        Self {
            primary_notifier: Box::new(StubNotifier),
            enabled: true,
            log_storage: Some(log_storage),
        }
    }
    
    /// Создаёт новый NotificationManager с libnotify бэкендом.
    /// 
    /// # Аргументы
    /// * `app_name` - Имя приложения для уведомлений.
    /// 
    /// # Возвращает
    /// Новый экземпляр NotificationManager с libnotify бэкендом.
    /// 
    /// # Примечания
    /// Доступно только при включении фичи `libnotify`.

    // libnotify support is temporarily disabled
    // pub fn new_libnotify(app_name: impl Into<String>) -> Self {
    //     Self::new(LibnotifyNotifier::new(app_name))
    // }

    /// Создаёт новый NotificationManager с D-Bus бэкендом.
    /// 
    /// # Примечания
    /// Доступно только при включении фичи `dbus`.
    #[cfg(feature = "dbus")]
    pub fn new_dbus(app_name: impl Into<String>) -> Self {
        Self::new(DBusNotifier::new(app_name))
    }
    
    /// Включает или отключает отправку уведомлений.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    /// Возвращает true, если отправка уведомлений включена.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// Отправляет уведомление через основной бэкенд.
    /// 
    /// # Аргументы
    /// * `notification` - Уведомление для отправки.
    /// 
    /// # Возвращает
    /// `Result<()>` - Ok, если уведомление успешно отправлено, иначе ошибка.
    /// Если отправка уведомлений отключена, возвращает Ok(()).
    pub async fn send(&self, notification: &Notification) -> Result<()> {
        if !self.enabled {
            tracing::debug!("Notifications are disabled, skipping notification");
            return Ok(());
        }
        
        // Логируем уведомление в хранилище логов, если оно доступно
        if let Some(ref log_storage_arc) = self.log_storage {
            let log_level = match notification.notification_type {
                NotificationType::Critical => crate::logging::log_storage::LogLevel::Error,
                NotificationType::Warning => crate::logging::log_storage::LogLevel::Warn,
                NotificationType::Info => crate::logging::log_storage::LogLevel::Info,
            };
            
            let mut log_entry = crate::logging::log_storage::LogEntry::new(
                log_level,
                "notifications",
                format!("{} - {}", notification.title, notification.message),
            );
            
            if let Some(details) = &notification.details {
                let fields = serde_json::json!({
                    "notification_type": format!("{}", notification.notification_type),
                    "timestamp": notification.timestamp.to_rfc3339(),
                    "details": details,
                });
                log_entry = log_entry.with_fields(fields);
            }
            
            log_storage_arc.add_entry(log_entry).await;
        }
        
        self.primary_notifier.send_notification(notification).await
    }
    
    /// Возвращает имя текущего бэкенда уведомлений.
    pub fn backend_name(&self) -> &str {
        self.primary_notifier.backend_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_notification_creation() {
        let notification = Notification::new(
            NotificationType::Info,
            "Test Title",
            "Test Message",
        );
        
        assert_eq!(notification.notification_type, NotificationType::Info);
        assert_eq!(notification.title, "Test Title");
        assert_eq!(notification.message, "Test Message");
        assert!(notification.details.is_none());
        assert!(notification.timestamp <= Utc::now());
    }
    
    #[tokio::test]
    async fn test_notification_with_details() {
        let notification = Notification::new(
            NotificationType::Warning,
            "Test Title",
            "Test Message",
        ).with_details("Additional details");
        
        assert_eq!(notification.notification_type, NotificationType::Warning);
        assert_eq!(notification.title, "Test Title");
        assert_eq!(notification.message, "Test Message");
        assert_eq!(notification.details, Some("Additional details".to_string()));
    }
    
    #[tokio::test]
    async fn test_stub_notifier() {
        let notifier = StubNotifier;
        let notification = Notification::new(
            NotificationType::Info,
            "Test Title",
            "Test Message",
        );
        
        let result = notifier.send_notification(&notification).await;
        assert!(result.is_ok());
        assert_eq!(notifier.backend_name(), "stub");
    }
    
    #[tokio::test]
    async fn test_notification_manager_enabled() {
        let manager = NotificationManager::new_stub();
        let notification = Notification::new(
            NotificationType::Info,
            "Test Title",
            "Test Message",
        );
        
        assert!(manager.is_enabled());
        let result = manager.send(&notification).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_notification_manager_disabled() {
        let mut manager = NotificationManager::new_stub();
        manager.set_enabled(false);
        let notification = Notification::new(
            NotificationType::Info,
            "Test Title",
            "Test Message",
        );
        
        assert!(!manager.is_enabled());
        let result = manager.send(&notification).await;
        assert!(result.is_ok()); // Должно возвращать Ok, даже если отключено
    }
    
    #[test]
    fn test_notification_type_display() {
        assert_eq!(format!("{}", NotificationType::Critical), "CRITICAL");
        assert_eq!(format!("{}", NotificationType::Warning), "WARNING");
        assert_eq!(format!("{}", NotificationType::Info), "INFO");
    }
    
    #[test]
    fn test_notification_type_serde() {
        let critical = NotificationType::Critical;
        let serialized = serde_yaml::to_string(&critical).unwrap();
        assert!(serialized.contains("critical"));
        
        let warning = NotificationType::Warning;
        let serialized = serde_yaml::to_string(&warning).unwrap();
        assert!(serialized.contains("warning"));
        
        let info = NotificationType::Info;
        let serialized = serde_yaml::to_string(&info).unwrap();
        assert!(serialized.contains("info"));
    }
    

//     #[test]
//     fn test_libnotify_notifier_creation() {
//         let notifier = LibnotifyNotifier::new("TestApp");
//         assert_eq!(notifier.backend_name(), "libnotify");
//     }
//     
// 
//     #[test]
//     fn test_notification_manager_libnotify() {
//         let manager = NotificationManager::new_libnotify("TestApp");
//         assert_eq!(manager.backend_name(), "libnotify");
//         assert!(manager.is_enabled());
//     }

    #[cfg(feature = "dbus")]
    #[test]
    fn test_dbus_notifier_creation() {
        let notifier = DBusNotifier::new("TestApp");
        assert_eq!(notifier.backend_name(), "dbus");
        assert!(!notifier.is_connected());
    }

    #[cfg(feature = "dbus")]
    #[test]
    fn test_notification_manager_dbus() {
        let manager = NotificationManager::new_dbus("TestApp");
        assert_eq!(manager.backend_name(), "dbus");
        assert!(manager.is_enabled());
    }

    #[tokio::test]
    async fn test_notification_manager_with_logging() {
        use crate::logging::log_storage::SharedLogStorage;
        use std::sync::Arc;
        
        let log_storage = Arc::new(SharedLogStorage::new(10));
        let manager = NotificationManager::new_stub_with_logging(Arc::clone(&log_storage));
        
        assert!(manager.is_enabled());
        assert!(manager.log_storage.is_some());
        
        // Отправляем уведомление
        let notification = Notification::new(
            NotificationType::Info,
            "Test Title",
            "Test Message",
        ).with_details("Test details");
        
        let result = manager.send(&notification).await;
        assert!(result.is_ok());
        
        // Проверяем, что уведомление было залоггировано
        let entries = log_storage.get_entries_by_level(crate::logging::log_storage::LogLevel::Info).await;
        assert_eq!(entries.len(), 1);
        
        let entry = &entries[0];
        assert_eq!(entry.target, "notifications");
        assert!(entry.message.contains("Test Title - Test Message"));
        assert!(entry.fields.is_some());
        
        if let Some(fields) = &entry.fields {
            assert!(fields.get("notification_type").is_some());
            assert!(fields.get("details").is_some());
        }
    }

    #[tokio::test]
    async fn test_notification_manager_logging_levels() {
        use crate::logging::log_storage::SharedLogStorage;
        use std::sync::Arc;
        
        let log_storage = Arc::new(SharedLogStorage::new(20));
        let manager = NotificationManager::new_stub_with_logging(Arc::clone(&log_storage));
        
        // Отправляем уведомления разных уровней
        let critical_notification = Notification::new(
            NotificationType::Critical,
            "Critical Title",
            "Critical Message",
        );
        
        let warning_notification = Notification::new(
            NotificationType::Warning,
            "Warning Title",
            "Warning Message",
        );
        
        let info_notification = Notification::new(
            NotificationType::Info,
            "Info Title",
            "Info Message",
        );
        
        // Отправляем уведомления
        manager.send(&critical_notification).await.unwrap();
        manager.send(&warning_notification).await.unwrap();
        manager.send(&info_notification).await.unwrap();
        
        // Проверяем, что уведомления были залоггированы с правильными уровнями
        // Используем get_all_entries и фильтруем по уровню, чтобы избежать проблем с кэшированием
        let all_entries = log_storage.get_all_entries().await;
        let error_entries: Vec<_> = all_entries.iter().filter(|e| e.level == crate::logging::log_storage::LogLevel::Error).collect();
        let warn_entries: Vec<_> = all_entries.iter().filter(|e| e.level == crate::logging::log_storage::LogLevel::Warn).collect();
        let info_entries: Vec<_> = all_entries.iter().filter(|e| e.level == crate::logging::log_storage::LogLevel::Info).collect();
        
        assert_eq!(error_entries.len(), 1); // Critical -> Error
        assert_eq!(warn_entries.len(), 1);  // Warning -> Warn
        assert_eq!(info_entries.len(), 1);  // Info -> Info
    }

    #[tokio::test]
    async fn test_notification_manager_disabled_with_logging() {
        use crate::logging::log_storage::SharedLogStorage;
        use std::sync::Arc;
        
        let log_storage = Arc::new(SharedLogStorage::new(10));
        let mut manager = NotificationManager::new_stub_with_logging(Arc::clone(&log_storage));
        manager.set_enabled(false);
        
        // Отправляем уведомление (должно быть проигнорировано)
        let notification = Notification::new(
            NotificationType::Info,
            "Test Title",
            "Test Message",
        );
        
        let result = manager.send(&notification).await;
        assert!(result.is_ok());
        
        // Проверяем, что уведомление НЕ было залоггировано
        let entries = log_storage.get_entries_by_level(crate::logging::log_storage::LogLevel::Info).await;
        assert_eq!(entries.len(), 0);
    }
}