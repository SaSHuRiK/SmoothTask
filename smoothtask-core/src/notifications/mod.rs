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

    /// Уведомление о изменении приоритета - специальный тип для уведомлений
    /// о изменении приоритетов процессов.
    PriorityChange,

    /// Уведомление о изменении конфигурации - специальный тип для уведомлений
    /// о перезагрузке конфигурации или изменении настроек.
    ConfigChange,

    /// Уведомление о системном событии - специальный тип для уведомлений
    /// о системных событиях (запуск, остановка, ошибки системы и т.д.).
    SystemEvent,
}

impl fmt::Display for NotificationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotificationType::Critical => write!(f, "CRITICAL"),
            NotificationType::Warning => write!(f, "WARNING"),
            NotificationType::Info => write!(f, "INFO"),
            NotificationType::PriorityChange => write!(f, "PRIORITY_CHANGE"),
            NotificationType::ConfigChange => write!(f, "CONFIG_CHANGE"),
            NotificationType::SystemEvent => write!(f, "SYSTEM_EVENT"),
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

    /// Создаёт уведомление о изменении приоритета.
    pub fn priority_change(
        process_name: impl Into<String>,
        old_priority: impl Into<String>,
        new_priority: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            notification_type: NotificationType::PriorityChange,
            title: format!("Priority Changed: {}", process_name.into()),
            message: format!(
                "Priority changed from {} to {} - {}",
                old_priority.into(),
                new_priority.into(),
                reason.into()
            ),
            details: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Создаёт уведомление о изменении конфигурации.
    pub fn config_change(
        config_file: impl Into<String>,
        changes_summary: impl Into<String>,
    ) -> Self {
        Self {
            notification_type: NotificationType::ConfigChange,
            title: format!("Configuration Reloaded: {}", config_file.into()),
            message: format!("Configuration changes applied: {}", changes_summary.into()),
            details: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Создаёт уведомление о системном событии.
    pub fn system_event(
        event_type: impl Into<String>,
        event_description: impl Into<String>,
    ) -> Self {
        Self {
            notification_type: NotificationType::SystemEvent,
            title: format!("System Event: {}", event_type.into()),
            message: event_description.into(),
            details: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Создаёт уведомление о ресурсном событии.
    /// Используется для уведомлений о высоком использовании ресурсов (CPU, память, GPU и т.д.).
    pub fn resource_event(
        resource_type: impl Into<String> + Clone + std::fmt::Display,
        usage_value: impl Into<String>,
        threshold: impl Into<String>,
    ) -> Self {
        let resource_type_str = resource_type.clone();
        Self {
            notification_type: NotificationType::Warning,
            title: format!("High {} Usage", resource_type.into()),
            message: format!(
                "{} usage is at {} (threshold: {})",
                resource_type_str,
                usage_value.into(),
                threshold.into()
            ),
            details: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Создаёт уведомление о температурном событии.
    /// Используется для уведомлений о высокой температуре компонентов.
    pub fn temperature_event(
        component: impl Into<String> + Clone + std::fmt::Display,
        temperature: impl Into<String>,
        threshold: impl Into<String>,
    ) -> Self {
        let component_str = component.clone();
        Self {
            notification_type: NotificationType::Warning,
            title: format!("High {} Temperature", component.into()),
            message: format!(
                "{} temperature is at {}°C (threshold: {}°C)",
                component_str,
                temperature.into(),
                threshold.into()
            ),
            details: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Создаёт уведомление о сетевом событии.
    /// Используется для уведомлений о сетевой активности.
    pub fn network_event(
        event_type: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self {
            notification_type: NotificationType::Info,
            title: format!("Network Event: {}", event_type.into()),
            message: details.into(),
            details: None,
            timestamp: chrono::Utc::now(),
        }
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
            NotificationType::PriorityChange => {
                tracing::info!(
                    "[NOTIFICATION] {}: {}",
                    notification.title,
                    notification.message
                );
            }
            NotificationType::ConfigChange => {
                tracing::info!(
                    "[NOTIFICATION] {}: {}",
                    notification.title,
                    notification.message
                );
            }
            NotificationType::SystemEvent => {
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

    /// Устанавливает соединение с сессионным D-Bus (для пользовательских уведомлений).
    pub async fn connect_session(&mut self) -> Result<()> {
        self.connection = Some(Connection::session().await?);
        Ok(())
    }

    /// Проверяет доступность D-Bus сервиса уведомлений.
    pub async fn check_notification_service_available(&self) -> bool {
        if let Some(conn) = &self.connection {
            let proxy = zbus::Proxy::new(
                conn,
                "org.freedesktop.Notifications",
                "/org/freedesktop/Notifications",
                "org.freedesktop.Notifications",
            );
            
            // Пробуем вызвать метод GetServerInformation для проверки доступности
            let result: zbus::Result<(String, String, String, String)> = proxy
                .call_method("GetServerInformation", &())
                .await;
            
            result.is_ok()
        } else {
            false
        }
    }

    /// Получает информацию о сервере уведомлений.
    pub async fn get_server_information(&self) -> Result<(String, String, String, String)> {
        if let Some(conn) = &self.connection {
            let proxy = zbus::Proxy::new(
                conn,
                "org.freedesktop.Notifications",
                "/org/freedesktop/Notifications",
                "org.freedesktop.Notifications",
            );
            
            proxy.call_method("GetServerInformation", &()).await
        } else {
            Err(anyhow::anyhow!("D-Bus connection not established"))
        }
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

        // Проверяем доступность сервиса уведомлений
        let service_available = self.check_notification_service_available().await;
        if !service_available {
            tracing::warn!("D-Bus notification service not available, falling back to logging");
            // В случае отсутствия сервиса, логируем уведомление как заглушка
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
            return Ok(());
        }

        // Преобразуем тип уведомления в уровень срочности
        let urgency = match notification.notification_type {
            NotificationType::Critical => "critical",
            NotificationType::Warning => "normal",
            NotificationType::Info => "low",
            NotificationType::PriorityChange => "normal",
            NotificationType::ConfigChange => "low",
            NotificationType::SystemEvent => "normal",
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
            NotificationType::PriorityChange => "preferences-system-performance",
            NotificationType::ConfigChange => "preferences-system",
            NotificationType::SystemEvent => "computer",
        };
        let summary: &str = &notification.title;
        let body_str: &str = &body;
        let actions: Vec<&str> = vec![]; // Нет действий
        let hints: std::collections::HashMap<&str, zbus::zvariant::Value> = {
            let mut hints_map = std::collections::HashMap::new();
            // Устанавливаем уровень срочности
            hints_map.insert("urgency", zbus::zvariant::Value::new(urgency));
            // Добавляем временную метку
            hints_map.insert("timestamp", zbus::zvariant::Value::new(notification.timestamp.timestamp()));
            // Добавляем категорию уведомления
            let category = match notification.notification_type {
                NotificationType::Critical => "device.error",
                NotificationType::Warning => "device.warning",
                NotificationType::Info => "device.info",
                NotificationType::PriorityChange => "system.performance",
                NotificationType::ConfigChange => "system.config",
                NotificationType::SystemEvent => "system.event",
            };
            hints_map.insert("category", zbus::zvariant::Value::new(category));
            hints_map
        };
        let expire_timeout: i32 = match notification.notification_type {
            NotificationType::Critical => 10000, // 10 секунд для критических уведомлений
            NotificationType::Warning => 7000,  // 7 секунд для предупреждений
            _ => 5000, // 5 секунд для остальных
        };

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

/// Структура, представляющая текущее состояние системы уведомлений.
#[derive(Debug, Clone, Serialize)]
pub struct NotificationStatus {
    /// Флаг, указывающий, включены ли уведомления.
    pub enabled: bool,
    /// Текущий бэкенд уведомлений.
    pub backend: String,
    /// Флаг, указывающий, интегрирована ли система уведомлений с хранилищем логов.
    pub has_log_integration: bool,
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
    pub fn new_stub_with_logging(
        log_storage: std::sync::Arc<crate::logging::log_storage::SharedLogStorage>,
    ) -> Self {
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

    /// Создаёт новый NotificationManager с D-Bus бэкендом и интеграцией с хранилищем логов.
    ///
    /// # Примечания
    /// Доступно только при включении фичи `dbus`.
    #[cfg(feature = "dbus")]
    pub fn new_dbus_with_logging(
        notifier: DBusNotifier,
        log_storage: std::sync::Arc<crate::logging::log_storage::SharedLogStorage>,
    ) -> Self {
        Self {
            primary_notifier: Box::new(notifier),
            enabled: true,
            log_storage: Some(log_storage),
        }
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
                NotificationType::PriorityChange => crate::logging::log_storage::LogLevel::Info,
                NotificationType::ConfigChange => crate::logging::log_storage::LogLevel::Info,
                NotificationType::SystemEvent => crate::logging::log_storage::LogLevel::Info,
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

    /// Создаёт запись в логе на основе уведомления без отправки уведомления.
    ///
    /// # Аргументы
    /// * `notification` - Уведомление для логирования.
    ///
    /// # Возвращает
    /// `Result<()>` - Ok, если запись успешно добавлена в лог, иначе ошибка.
    pub async fn log_only(&self, notification: &Notification) -> Result<()> {
        // Логируем уведомление в хранилище логов, если оно доступно
        if let Some(ref log_storage_arc) = self.log_storage {
            let log_level = match notification.notification_type {
                NotificationType::Critical => crate::logging::log_storage::LogLevel::Error,
                NotificationType::Warning => crate::logging::log_storage::LogLevel::Warn,
                NotificationType::Info => crate::logging::log_storage::LogLevel::Info,
                NotificationType::PriorityChange => crate::logging::log_storage::LogLevel::Info,
                NotificationType::ConfigChange => crate::logging::log_storage::LogLevel::Info,
                NotificationType::SystemEvent => crate::logging::log_storage::LogLevel::Info,
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

        Ok(())
    }

    /// Возвращает текущее состояние системы уведомлений.
    pub fn get_status(&self) -> NotificationStatus {
        NotificationStatus {
            enabled: self.enabled,
            backend: self.backend_name().to_string(),
            has_log_integration: self.log_storage.is_some(),
        }
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
        let notification = Notification::new(NotificationType::Info, "Test Title", "Test Message");

        assert_eq!(notification.notification_type, NotificationType::Info);
        assert_eq!(notification.title, "Test Title");
        assert_eq!(notification.message, "Test Message");
        assert!(notification.details.is_none());
        assert!(notification.timestamp <= Utc::now());
    }

    #[tokio::test]
    async fn test_notification_with_details() {
        let notification =
            Notification::new(NotificationType::Warning, "Test Title", "Test Message")
                .with_details("Additional details");

        assert_eq!(notification.notification_type, NotificationType::Warning);
        assert_eq!(notification.title, "Test Title");
        assert_eq!(notification.message, "Test Message");
        assert_eq!(notification.details, Some("Additional details".to_string()));
    }

    #[tokio::test]
    async fn test_stub_notifier() {
        let notifier = StubNotifier;
        let notification = Notification::new(NotificationType::Info, "Test Title", "Test Message");

        let result = notifier.send_notification(&notification).await;
        assert!(result.is_ok());
        assert_eq!(notifier.backend_name(), "stub");
    }

    #[tokio::test]
    async fn test_notification_manager_enabled() {
        let manager = NotificationManager::new_stub();
        let notification = Notification::new(NotificationType::Info, "Test Title", "Test Message");

        assert!(manager.is_enabled());
        let result = manager.send(&notification).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notification_manager_disabled() {
        let mut manager = NotificationManager::new_stub();
        manager.set_enabled(false);
        let notification = Notification::new(NotificationType::Info, "Test Title", "Test Message");

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
        let notification = Notification::new(NotificationType::Info, "Test Title", "Test Message")
            .with_details("Test details");

        let result = manager.send(&notification).await;
        assert!(result.is_ok());

        // Проверяем, что уведомление было залоггировано
        let entries = log_storage
            .get_entries_by_level(crate::logging::log_storage::LogLevel::Info)
            .await;
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

        let info_notification =
            Notification::new(NotificationType::Info, "Info Title", "Info Message");

        // Отправляем уведомления
        manager.send(&critical_notification).await.unwrap();
        manager.send(&warning_notification).await.unwrap();
        manager.send(&info_notification).await.unwrap();

        // Проверяем, что уведомления были залоггированы с правильными уровнями
        // Используем get_all_entries и фильтруем по уровню, чтобы избежать проблем с кэшированием
        let all_entries = log_storage.get_all_entries().await;
        let error_entries: Vec<_> = all_entries
            .iter()
            .filter(|e| e.level == crate::logging::log_storage::LogLevel::Error)
            .collect();
        let warn_entries: Vec<_> = all_entries
            .iter()
            .filter(|e| e.level == crate::logging::log_storage::LogLevel::Warn)
            .collect();
        let info_entries: Vec<_> = all_entries
            .iter()
            .filter(|e| e.level == crate::logging::log_storage::LogLevel::Info)
            .collect();

        assert_eq!(error_entries.len(), 1); // Critical -> Error
        assert_eq!(warn_entries.len(), 1); // Warning -> Warn
        assert_eq!(info_entries.len(), 1); // Info -> Info
    }

    #[tokio::test]
    async fn test_notification_manager_disabled_with_logging() {
        use crate::logging::log_storage::SharedLogStorage;
        use std::sync::Arc;

        let log_storage = Arc::new(SharedLogStorage::new(10));
        let mut manager = NotificationManager::new_stub_with_logging(Arc::clone(&log_storage));
        manager.set_enabled(false);

        // Отправляем уведомление (должно быть проигнорировано)
        let notification = Notification::new(NotificationType::Info, "Test Title", "Test Message");

        let result = manager.send(&notification).await;
        assert!(result.is_ok());

        // Проверяем, что уведомление НЕ было залоггировано
        let entries = log_storage
            .get_entries_by_level(crate::logging::log_storage::LogLevel::Info)
            .await;
        assert_eq!(entries.len(), 0);
    }

    #[tokio::test]
    async fn test_new_notification_types() {
        // Тестируем новые типы уведомлений
        let priority_notification =
            Notification::priority_change("test_process", "low", "high", "user request");
        assert_eq!(
            priority_notification.notification_type,
            NotificationType::PriorityChange
        );
        assert!(priority_notification
            .title
            .contains("Priority Changed: test_process"));
        assert!(priority_notification
            .message
            .contains("Priority changed from low to high - user request"));

        let config_notification =
            Notification::config_change("/etc/smoothtask/config.yml", "updated qos settings");
        assert_eq!(
            config_notification.notification_type,
            NotificationType::ConfigChange
        );
        assert!(config_notification
            .title
            .contains("Configuration Reloaded: /etc/smoothtask/config.yml"));
        assert!(config_notification
            .message
            .contains("Configuration changes applied: updated qos settings"));

        let system_notification =
            Notification::system_event("startup", "SmoothTask daemon started successfully");
        assert_eq!(
            system_notification.notification_type,
            NotificationType::SystemEvent
        );
        assert!(system_notification.title.contains("System Event: startup"));
        assert_eq!(
            system_notification.message,
            "SmoothTask daemon started successfully"
        );
    }

    #[tokio::test]
    async fn test_notification_type_display_new_types() {
        assert_eq!(
            format!("{}", NotificationType::PriorityChange),
            "PRIORITY_CHANGE"
        );
        assert_eq!(
            format!("{}", NotificationType::ConfigChange),
            "CONFIG_CHANGE"
        );
        assert_eq!(format!("{}", NotificationType::SystemEvent), "SYSTEM_EVENT");
    }

    #[tokio::test]
    async fn test_notification_type_serde_new_types() {
        let priority_change = NotificationType::PriorityChange;
        let serialized = serde_yaml::to_string(&priority_change).unwrap();
        assert!(serialized.contains("priority-change"));

        let config_change = NotificationType::ConfigChange;
        let serialized = serde_yaml::to_string(&config_change).unwrap();
        assert!(serialized.contains("config-change"));

        let system_event = NotificationType::SystemEvent;
        let serialized = serde_yaml::to_string(&system_event).unwrap();
        assert!(serialized.contains("system-event"));
    }

    #[tokio::test]
    async fn test_stub_notifier_new_types() {
        let notifier = StubNotifier;

        let priority_notification =
            Notification::priority_change("test_app", "normal", "high", "policy change");
        let result = notifier.send_notification(&priority_notification).await;
        assert!(result.is_ok());

        let config_notification = Notification::config_change("config.yml", "updated settings");
        let result = notifier.send_notification(&config_notification).await;
        assert!(result.is_ok());

        let system_notification = Notification::system_event("shutdown", "System shutting down");
        let result = notifier.send_notification(&system_notification).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notification_manager_log_only() {
        use crate::logging::log_storage::SharedLogStorage;
        use std::sync::Arc;

        let log_storage = Arc::new(SharedLogStorage::new(10));
        let manager = NotificationManager::new_stub_with_logging(Arc::clone(&log_storage));

        // Создаём уведомление и логируем его без отправки
        let notification =
            Notification::priority_change("test_process", "low", "high", "test reason");

        let result = manager.log_only(&notification).await;
        assert!(result.is_ok());

        // Проверяем, что уведомление было залоггировано
        let entries = log_storage
            .get_entries_by_level(crate::logging::log_storage::LogLevel::Info)
            .await;
        assert_eq!(entries.len(), 1);

        let entry = &entries[0];
        assert_eq!(entry.target, "notifications");
        assert!(entry.message.contains(
            "Priority Changed: test_process - Priority changed from low to high - test reason"
        ));
    }

    #[tokio::test]
    async fn test_notification_manager_get_status() {
        let manager = NotificationManager::new_stub();
        let status = manager.get_status();

        assert!(status.enabled);
        assert_eq!(status.backend, "stub");
        assert!(!status.has_log_integration);
    }

    #[tokio::test]
    async fn test_notification_manager_get_status_with_logging() {
        use crate::logging::log_storage::SharedLogStorage;
        use std::sync::Arc;

        let log_storage = Arc::new(SharedLogStorage::new(10));
        let manager = NotificationManager::new_stub_with_logging(Arc::clone(&log_storage));
        let status = manager.get_status();

        assert!(status.enabled);
        assert_eq!(status.backend, "stub");
        assert!(status.has_log_integration);
    }

    #[tokio::test]
    async fn test_notification_manager_get_status_disabled() {
        let mut manager = NotificationManager::new_stub();
        manager.set_enabled(false);
        let status = manager.get_status();

        assert!(!status.enabled);
        assert_eq!(status.backend, "stub");
        assert!(!status.has_log_integration);
    }

    #[tokio::test]
    async fn test_notification_serialization_with_new_types() {
        let notification =
            Notification::priority_change("firefox", "normal", "high", "interactive application")
                .with_details("Process ID: 1234, User: testuser");

        let serialized = serde_json::to_string(&notification).unwrap();
        let deserialized: Notification = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            deserialized.notification_type,
            NotificationType::PriorityChange
        );
        assert_eq!(deserialized.title, "Priority Changed: firefox");
        assert!(deserialized
            .message
            .contains("Priority changed from normal to high - interactive application"));
        assert_eq!(
            deserialized.details,
            Some("Process ID: 1234, User: testuser".to_string())
        );
    }

    #[tokio::test]
    async fn test_notification_manager_comprehensive() {
        use crate::logging::log_storage::SharedLogStorage;
        use std::sync::Arc;

        let log_storage = Arc::new(SharedLogStorage::new(20));
        let manager = NotificationManager::new_stub_with_logging(Arc::clone(&log_storage));

        // Тестируем все типы уведомлений
        let notifications = vec![
            Notification::new(
                NotificationType::Critical,
                "Critical Test",
                "Critical message",
            ),
            Notification::new(NotificationType::Warning, "Warning Test", "Warning message"),
            Notification::new(NotificationType::Info, "Info Test", "Info message"),
            Notification::priority_change("app1", "low", "high", "reason1"),
            Notification::config_change("config.yml", "changes applied"),
            Notification::system_event("startup", "system started"),
        ];

        // Отправляем все уведомления
        for notification in &notifications {
            let result = manager.send(notification).await;
            assert!(
                result.is_ok(),
                "Failed to send notification: {:?}",
                notification
            );
        }

        // Проверяем, что все уведомления были залоггированы
        let all_entries = log_storage.get_all_entries().await;
        assert_eq!(
            all_entries.len(),
            6,
            "Expected 6 log entries, got {}",
            all_entries.len()
        );

        // Проверяем, что разные типы уведомлений имеют правильные уровни логирования
        let info_entries: Vec<_> = all_entries
            .iter()
            .filter(|e| e.level == crate::logging::log_storage::LogLevel::Info)
            .collect();

        let warn_entries: Vec<_> = all_entries
            .iter()
            .filter(|e| e.level == crate::logging::log_storage::LogLevel::Warn)
            .collect();

        let error_entries: Vec<_> = all_entries
            .iter()
            .filter(|e| e.level == crate::logging::log_storage::LogLevel::Error)
            .collect();

        assert_eq!(error_entries.len(), 1, "Expected 1 error entry");
        assert_eq!(warn_entries.len(), 1, "Expected 1 warning entry");
        assert_eq!(info_entries.len(), 4, "Expected 4 info entries");
    }



    #[tokio::test]
    async fn test_notification_manager_new_types() {
        use crate::logging::log_storage::SharedLogStorage;
        use std::sync::Arc;

        let log_storage = Arc::new(SharedLogStorage::new(15));
        let manager = NotificationManager::new_stub_with_logging(Arc::clone(&log_storage));

        // Тестируем новые типы уведомлений
        let notifications = vec![
            Notification::resource_event("Memory", "12GB", "10GB"),
            Notification::temperature_event("GPU", "80", "75"),
            Notification::network_event("Connection Spike", "1000 active connections"),
        ];

        // Отправляем все уведомления
        for notification in &notifications {
            let result = manager.send(notification).await;
            assert!(result.is_ok(), "Failed to send notification: {:?}", notification);
        }

        // Проверяем, что все уведомления были залоггированы
        let all_entries = log_storage.get_all_entries().await;
        assert_eq!(all_entries.len(), 3, "Expected 3 log entries, got {}", all_entries.len());

        // Проверяем уровни логирования
        let warn_entries: Vec<_> = all_entries
            .iter()
            .filter(|e| e.level == crate::logging::log_storage::LogLevel::Warn)
            .collect();

        let info_entries: Vec<_> = all_entries
            .iter()
            .filter(|e| e.level == crate::logging::log_storage::LogLevel::Info)
            .collect();

        assert_eq!(warn_entries.len(), 2, "Expected 2 warning entries");
        assert_eq!(info_entries.len(), 1, "Expected 1 info entry");
    }

    #[cfg(feature = "dbus")]
    #[tokio::test]
    async fn test_dbus_notifier_enhanced_features() {
        let mut notifier = DBusNotifier::new("TestApp");
        
        // Проверяем, что соединение не установлено изначально
        assert!(!notifier.is_connected());
        
        // Проверяем, что сервис уведомлений недоступен без соединения
        assert!(!notifier.check_notification_service_available().await);
        
        // Проверяем, что получение информации о сервере возвращает ошибку без соединения
        let result = notifier.get_server_information().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_notification_serialization_new_types() {
        let resource_notification = Notification::resource_event("GPU", "90%", "85%");
        let serialized = serde_json::to_string(&resource_notification).unwrap();
        let deserialized: Notification = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.notification_type, NotificationType::Warning);
        assert_eq!(deserialized.title, "High GPU Usage");
        assert!(deserialized.message.contains("GPU usage is at 90% (threshold: 85%)"));

        let temperature_notification = Notification::temperature_event("CPU", "85", "80");
        let serialized = serde_json::to_string(&temperature_notification).unwrap();
        let deserialized: Notification = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.notification_type, NotificationType::Warning);
        assert_eq!(deserialized.title, "High CPU Temperature");
        assert!(deserialized.message.contains("CPU temperature is at 85°C (threshold: 80°C)"));
    }
}
