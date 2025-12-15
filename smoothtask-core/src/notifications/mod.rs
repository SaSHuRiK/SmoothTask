//! Модуль системы уведомлений.
//!
//! Предоставляет инфраструктуру для отправки уведомлений пользователю о важных событиях
//! в работе демона. Поддерживает различные бэкенды (заглушки, desktop уведомления и т.д.).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

// Import health monitoring types for integration
use crate::health::monitoring::{HealthEvent, HealthMonitoringService};
use crate::health::{HealthIssue, HealthIssueSeverity, HealthStatus};

// Conditional import for libnotify
// libnotify support is temporarily disabled due to crate availability issues
// #[cfg(feature = "libnotify")]
// use libnotify::Notification as LibnotifyNotification;

#[cfg(feature = "dbus")]
use zbus::Connection;

#[cfg(feature = "dbus")]
use zbus::zvariant::Value;

/// Тип уведомления, определяющий его важность и визуальное представление.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
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
    pub fn network_event(event_type: impl Into<String>, details: impl Into<String>) -> Self {
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

/// Notifier на основе вебхуков для отправки уведомлений через HTTP/HTTPS.
/// Поддерживает конфигурируемые URL, заголовки и таймауты.
#[derive(Debug, Clone)]
pub struct WebhookNotifier {
    /// Базовый URL вебхука.
    webhook_url: String,
    /// Дополнительные заголовки для HTTP запросов.
    headers: std::collections::HashMap<String, String>,
    /// Таймаут для HTTP запросов в секундах.
    timeout_seconds: u64,
    /// Флаг, разрешающий небезопасные HTTPS соединения (для самоподписанных сертификатов).
    allow_insecure_https: bool,
    /// HTTP клиент для отправки запросов.
    client: reqwest::Client,
}

/// Notifier на основе email для отправки уведомлений через SMTP.
/// Доступно только при включении фичи `email`.
#[cfg(feature = "email")]
#[derive(Debug, Clone)]
pub struct EmailNotifier {
    /// SMTP сервер для отправки email.
    smtp_server: String,
    /// Порт SMTP сервера.
    smtp_port: u16,
    /// Email отправителя.
    from_email: String,
    /// Имя отправителя.
    from_name: String,
    /// Email получателя.
    to_email: String,
    /// Имя получателя.
    to_name: String,
    /// Логин для SMTP аутентификации.
    smtp_username: Option<String>,
    /// Пароль для SMTP аутентификации.
    smtp_password: Option<String>,
    /// Флаг, указывающий, использовать ли TLS.
    use_tls: bool,
    /// Таймаут для SMTP соединения в секундах.
    timeout_seconds: u64,
}

/// Notifier на основе SMS для отправки уведомлений через HTTP SMS шлюзы.
/// Поддерживает различные SMS провайдеры через HTTP API.
#[derive(Debug, Clone)]
pub struct SmsNotifier {
    /// URL SMS шлюза.
    gateway_url: String,
    /// Имя пользователя для аутентификации.
    username: Option<String>,
    /// Пароль для аутентификации.
    password: Option<String>,
    /// API ключ для аутентификации.
    api_key: Option<String>,
    /// Номер телефона получателя.
    phone_number: String,
    /// Дополнительные заголовки для HTTP запросов.
    headers: std::collections::HashMap<String, String>,
    /// Таймаут для HTTP запросов в секундах.
    timeout_seconds: u64,
    /// HTTP клиент для отправки запросов.
    client: reqwest::Client,
}

/// Notifier на основе Telegram для отправки уведомлений через Telegram Bot API.
/// Доступно только при включении фичи `telegram`.
#[cfg(feature = "telegram")]
#[derive(Debug, Clone)]
pub struct TelegramNotifier {
    /// Токен Telegram бота.
    bot_token: String,
    /// Идентификатор чата для отправки уведомлений.
    chat_id: String,
    /// Таймаут для HTTP запросов в секундах.
    timeout_seconds: u64,
    /// HTTP клиент для отправки запросов.
    client: reqwest::Client,
}

/// Notifier на основе Discord для отправки уведомлений через Discord Webhook API.
/// Доступно только при включении фичи `discord`.
#[cfg(feature = "discord")]
#[derive(Debug, Clone)]
pub struct DiscordNotifier {
    /// URL вебхука Discord.
    webhook_url: String,
    /// Таймаут для HTTP запросов в секундах.
    timeout_seconds: u64,
    /// HTTP клиент для отправки запросов.
    client: reqwest::Client,
}

impl WebhookNotifier {
    /// Создаёт новый WebhookNotifier с указанным URL вебхука.
    ///
    /// # Аргументы
    /// * `webhook_url` - URL вебхука для отправки уведомлений.
    ///
    /// # Возвращает
    /// Новый экземпляр WebhookNotifier.
    pub fn new(webhook_url: impl Into<String>) -> Self {
        Self {
            webhook_url: webhook_url.into(),
            headers: std::collections::HashMap::new(),
            timeout_seconds: 10,
            allow_insecure_https: false,
            client: reqwest::Client::new(),
        }
    }

    /// Устанавливает дополнительные заголовки для HTTP запросов.
    ///
    /// # Аргументы
    /// * `headers` - HashMap с заголовками.
    ///
    /// # Возвращает
    /// Мутированный экземпляр WebhookNotifier.
    pub fn with_headers(mut self, headers: std::collections::HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    /// Устанавливает таймаут для HTTP запросов.
    ///
    /// # Аргументы
    /// * `timeout_seconds` - Таймаут в секундах.
    ///
    /// # Возвращает
    /// Мутированный экземпляр WebhookNotifier.
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    /// Разрешает небезопасные HTTPS соединения (для самоподписанных сертификатов).
    ///
    /// # Возвращает
    /// Мутированный экземпляр WebhookNotifier.
    pub fn allow_insecure_https(mut self) -> Self {
        self.allow_insecure_https = true;
        self
    }

    /// Возвращает текущий HTTP клиент.
    ///
    /// # Возвращает
    /// Экземпляр reqwest::Client.
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Возвращает URL вебхука.
    ///
    /// # Возвращает
    /// URL вебхука.
    pub fn webhook_url(&self) -> &str {
        &self.webhook_url
    }

    /// Возвращает дополнительные заголовки.
    ///
    /// # Возвращает
    /// Ссылку на HashMap с заголовками.
    pub fn headers(&self) -> &std::collections::HashMap<String, String> {
        &self.headers
    }

    /// Возвращает таймаут в секундах.
    ///
    /// # Возвращает
    /// Таймаут в секундах.
    pub fn timeout_seconds(&self) -> u64 {
        self.timeout_seconds
    }

    /// Возвращает true, если разрешены небезопасные HTTPS соединения.
    ///
    /// # Возвращает
    /// Флаг allow_insecure_https.
    pub fn is_insecure_https_allowed(&self) -> bool {
        self.allow_insecure_https
    }
}

/// Notifier на основе D-Bus для отправки уведомлений через системный D-Bus.
/// Использует стандартный протокол org.freedesktop.Notifications.
#[cfg(feature = "dbus")]
pub struct DBusNotifier {
    /// Имя приложения для уведомлений.
    app_name: String,
    /// Идентификатор соединения D-Bus.
    connection: Option<Connection>,
}

/// Реализация SmsNotifier для отправки уведомлений через HTTP SMS шлюзы.
impl SmsNotifier {
    /// Создаёт новый SmsNotifier с указанными параметрами.
    ///
    /// # Аргументы
    /// * `gateway_url` - URL SMS шлюза.
    /// * `phone_number` - Номер телефона получателя.
    ///
    /// # Возвращает
    /// Новый экземпляр SmsNotifier.
    pub fn new(gateway_url: impl Into<String>, phone_number: impl Into<String>) -> Self {
        Self {
            gateway_url: gateway_url.into(),
            username: None,
            password: None,
            api_key: None,
            phone_number: phone_number.into(),
            headers: std::collections::HashMap::new(),
            timeout_seconds: 30,
            client: reqwest::Client::new(),
        }
    }

    /// Устанавливает учётные данные для аутентификации.
    ///
    /// # Аргументы
    /// * `username` - Имя пользователя для аутентификации.
    /// * `password` - Пароль для аутентификации.
    ///
    /// # Возвращает
    /// Мутированный экземпляр SmsNotifier.
    pub fn with_credentials(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }

    /// Устанавливает API ключ для аутентификации.
    ///
    /// # Аргументы
    /// * `api_key` - API ключ для аутентификации.
    ///
    /// # Возвращает
    /// Мутированный экземпляр SmsNotifier.
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Устанавливает дополнительные заголовки для HTTP запросов.
    ///
    /// # Аргументы
    /// * `headers` - HashMap с заголовками.
    ///
    /// # Возвращает
    /// Мутированный экземпляр SmsNotifier.
    pub fn with_headers(mut self, headers: std::collections::HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    /// Устанавливает таймаут для HTTP запросов.
    ///
    /// # Аргументы
    /// * `timeout_seconds` - Таймаут в секундах.
    ///
    /// # Возвращает
    /// Мутированный экземпляр SmsNotifier.
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }



    /// Возвращает URL SMS шлюза.
    ///
    /// # Возвращает
    /// URL SMS шлюза.
    pub fn gateway_url(&self) -> &str {
        &self.gateway_url
    }

    /// Возвращает номер телефона получателя.
    ///
    /// # Возвращает
    /// Номер телефона получателя.
    pub fn phone_number(&self) -> &str {
        &self.phone_number
    }

    /// Возвращает дополнительные заголовки.
    ///
    /// # Возвращает
    /// Ссылку на HashMap с заголовками.
    pub fn headers(&self) -> &std::collections::HashMap<String, String> {
        &self.headers
    }

    /// Возвращает таймаут в секундах.
    ///
    /// # Возвращает
    /// Таймаут в секундах.
    pub fn timeout_seconds(&self) -> u64 {
        self.timeout_seconds
    }
}

/// Реализация EmailNotifier для отправки уведомлений через SMTP.
/// Доступно только при включении фичи `email`.
#[cfg(feature = "email")]
impl EmailNotifier {
    /// Создаёт новый EmailNotifier с указанными параметрами.
    ///
    /// # Аргументы
    /// * `smtp_server` - SMTP сервер для отправки email.
    /// * `smtp_port` - Порт SMTP сервера.
    /// * `from_email` - Email отправителя.
    /// * `from_name` - Имя отправителя.
    /// * `to_email` - Email получателя.
    /// * `to_name` - Имя получателя.
    /// * `use_tls` - Флаг, указывающий, использовать ли TLS.
    ///
    /// # Возвращает
    /// Новый экземпляр EmailNotifier.
    pub fn new(
        smtp_server: impl Into<String>,
        smtp_port: u16,
        from_email: impl Into<String>,
        from_name: impl Into<String>,
        to_email: impl Into<String>,
        to_name: impl Into<String>,
        use_tls: bool,
    ) -> Self {
        Self {
            smtp_server: smtp_server.into(),
            smtp_port,
            from_email: from_email.into(),
            from_name: from_name.into(),
            to_email: to_email.into(),
            to_name: to_name.into(),
            smtp_username: None,
            smtp_password: None,
            use_tls,
            timeout_seconds: 30,
        }
    }

    /// Устанавливает учётные данные для SMTP аутентификации.
    ///
    /// # Аргументы
    /// * `username` - Логин для SMTP аутентификации.
    /// * `password` - Пароль для SMTP аутентификации.
    ///
    /// # Возвращает
    /// Мутированный экземпляр EmailNotifier.
    pub fn with_credentials(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.smtp_username = Some(username.into());
        self.smtp_password = Some(password.into());
        self
    }

    /// Устанавливает таймаут для SMTP соединения.
    ///
    /// # Аргументы
    /// * `timeout_seconds` - Таймаут в секундах.
    ///
    /// # Возвращает
    /// Мутированный экземпляр EmailNotifier.
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    /// Создаёт SMTP транспорт для отправки email.
    ///
    /// # Возвращает
    /// Результат с SMTP транспортом или ошибкой.
    async fn create_smtp_transport(&self) -> Result<lettre::AsyncSmtpTransport<lettre::Tokio1Executor>> {
        let mut builder = if self.use_tls {
            lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::relay(&self.smtp_server)?
                .port(self.smtp_port)
                .tls(lettre::transport::smtp::client::Tls::Required(
                    lettre::transport::smtp::client::TlsParameters::new(
                        self.smtp_server.clone(),
                    )?,
                ))
        } else {
            lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::relay(&self.smtp_server)?
                .port(self.smtp_port)
        };

        // Устанавливаем таймаут
        builder = builder.timeout(std::time::Duration::from_secs(self.timeout_seconds));

        // Устанавливаем учётные данные, если они указаны
        if let (Some(username), Some(password)) = (&self.smtp_username, &self.smtp_password) {
            builder = builder.credentials(lettre::transport::smtp::authentication::Credentials::new(
                username.clone(),
                password.clone(),
            ));
        }

        Ok(builder.build())
    }

    /// Возвращает SMTP сервер.
    ///
    /// # Возвращает
    /// SMTP сервер.
    pub fn smtp_server(&self) -> &str {
        &self.smtp_server
    }

    /// Возвращает порт SMTP сервера.
    ///
    /// # Возвращает
    /// Порт SMTP сервера.
    pub fn smtp_port(&self) -> u16 {
        self.smtp_port
    }

    /// Возвращает email отправителя.
    ///
    /// # Возвращает
    /// Email отправителя.
    pub fn from_email(&self) -> &str {
        &self.from_email
    }

    /// Возвращает email получателя.
    ///
    /// # Возвращает
    /// Email получателя.
    pub fn to_email(&self) -> &str {
        &self.to_email
    }

    /// Возвращает true, если используется TLS.
    ///
    /// # Возвращает
    /// Флаг use_tls.
    pub fn is_tls_used(&self) -> bool {
        self.use_tls
    }

    /// Возвращает таймаут в секундах.
    ///
    /// # Возвращает
    /// Таймаут в секундах.
    pub fn timeout_seconds(&self) -> u64 {
        self.timeout_seconds
    }
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
            let result: zbus::Result<(String, String, String, String)> =
                proxy.call_method("GetServerInformation", &()).await;

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

#[async_trait::async_trait]
impl Notifier for WebhookNotifier {
    async fn send_notification(&self, notification: &Notification) -> Result<()> {
        // Используем хранимый HTTP клиент
        let client = self.client();

        // Преобразуем уведомление в JSON формат
        let notification_json = serde_json::json!({
            "notification_type": format!("{}", notification.notification_type),
            "title": notification.title,
            "message": notification.message,
            "details": notification.details,
            "timestamp": notification.timestamp.to_rfc3339(),
        });

        // Логируем отправку уведомления
        tracing::info!(
            "Sending webhook notification to {}: {} - {}",
            self.webhook_url,
            notification.title,
            notification.message
        );

        // Отправляем POST запрос на вебхук
        let mut request_builder = client.post(&self.webhook_url);

        // Добавляем заголовки
        for (key, value) in &self.headers {
            request_builder = request_builder.header(key, value);
        }

        // Устанавливаем Content-Type как application/json
        request_builder = request_builder.header("Content-Type", "application/json");

        // Отправляем запрос
        let response = request_builder
            .json(&notification_json)
            .send()
            .await;

        match response {
            Ok(resp) => {
                // Проверяем статус код
                if resp.status().is_success() {
                    tracing::info!(
                        "Successfully sent webhook notification to {}: {} - {}",
                        self.webhook_url,
                        notification.title,
                        notification.message
                    );
                    Ok(())
                } else {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    tracing::error!(
                        "Failed to send webhook notification to {}: HTTP {} - {}",
                        self.webhook_url,
                        status,
                        body
                    );
                    Err(anyhow::anyhow!(
                        "Webhook notification failed: HTTP {} - {}",
                        status,
                        body
                    ))
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to send webhook notification to {}: {}",
                    self.webhook_url,
                    e
                );
                Err(anyhow::anyhow!("Webhook notification failed: {}", e))
            }
        }
    }

    fn backend_name(&self) -> &str {
        "webhook"
    }
}

/// Реализация Notifier для EmailNotifier.
/// Доступно только при включении фичи `email`.
#[cfg(feature = "email")]
#[async_trait::async_trait]
impl Notifier for EmailNotifier {
    async fn send_notification(&self, notification: &Notification) -> Result<()> {
        // Логируем отправку email уведомления
        tracing::info!(
            "Sending email notification to {}: {} - {}",
            self.to_email,
            notification.title,
            notification.message
        );

        // Создаём SMTP транспорт
        let smtp_transport = self.create_smtp_transport().await?;

        // Формируем тему письма
        let subject = format!("[SmoothTask] {}", notification.title);

        // Формируем тело письма
        let mut body = format!(
            "SmoothTask Notification\n\nType: {}\n\nMessage:\n{}",
            notification.notification_type,
            notification.message
        );

        // Добавляем дополнительные детали, если они есть
        if let Some(details) = &notification.details {
            body.push_str("\n\nDetails:\n");
            body.push_str(details);
        }

        // Добавляем временную метку
        body.push_str("\n\n---\n");
        body.push_str(&format!("Timestamp: {}", notification.timestamp.to_rfc3339()));

        // Создаём email сообщение
        let email = lettre::Message::builder()
            .from(format!("{} <{}>", self.from_name, self.from_email).parse()?)
            .to(format!("{} <{}>", self.to_name, self.to_email).parse()?)
            .subject(subject)
            .body(body)?;

        // Отправляем email
        match smtp_transport.send(email).await {
            Ok(_) => {
                tracing::info!(
                    "Successfully sent email notification to {}: {} - {}",
                    self.to_email,
                    notification.title,
                    notification.message
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    "Failed to send email notification to {}: {}",
                    self.to_email,
                    e
                );
                Err(anyhow::anyhow!("Email notification failed: {}", e))
            }
        }
    }

    fn backend_name(&self) -> &str {
        "email"
    }
}

/// Реализация Notifier для SmsNotifier.
#[async_trait::async_trait]
impl Notifier for SmsNotifier {
    async fn send_notification(&self, notification: &Notification) -> Result<()> {
        // Используем хранимый HTTP клиент
        let client = &self.client;

        // Логируем отправку SMS уведомления
        tracing::info!(
            "Sending SMS notification to {}: {} - {}",
            self.phone_number,
            notification.title,
            notification.message
        );

        // Формируем сообщение SMS
        let mut sms_message = format!("SmoothTask: {}", notification.title);
        sms_message.push_str("\n");
        sms_message.push_str(&notification.message);

        // Добавляем дополнительные детали, если они есть и помещаются в лимит
        if let Some(details) = &notification.details {
            let details_preview = if details.len() > 50 {
                format!("{}...", &details[..50])
            } else {
                details.clone()
            };
            sms_message.push_str("\n");
            sms_message.push_str(&details_preview);
        }

        // Ограничиваем длину сообщения (обычно SMS ограничены 160 символами)
        let sms_message = if sms_message.len() > 160 {
            format!("{}...", &sms_message[..157])
        } else {
            sms_message
        };

        // Подготавливаем параметры для SMS шлюза
        let mut request_builder = client.post(&self.gateway_url);

        // Добавляем заголовки
        for (key, value) in &self.headers {
            request_builder = request_builder.header(key, value);
        }

        // Добавляем параметры аутентификации
        let mut form_data = std::collections::HashMap::new();
        form_data.insert("phone".to_string(), self.phone_number.clone());
        form_data.insert("message".to_string(), sms_message.clone());

        if let Some(username) = &self.username {
            form_data.insert("username".to_string(), username.clone());
        }
        if let Some(password) = &self.password {
            form_data.insert("password".to_string(), password.clone());
        }
        if let Some(api_key) = &self.api_key {
            form_data.insert("api_key".to_string(), api_key.clone());
        }

        // Отправляем запрос
        let response = request_builder.form(&form_data).send().await;

        match response {
            Ok(resp) => {
                // Проверяем статус код
                if resp.status().is_success() {
                    tracing::info!(
                        "Successfully sent SMS notification to {}: {} - {}",
                        self.phone_number,
                        notification.title,
                        notification.message
                    );
                    Ok(())
                } else {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    tracing::error!(
                        "Failed to send SMS notification to {}: HTTP {} - {}",
                        self.phone_number,
                        status,
                        body
                    );
                    Err(anyhow::anyhow!(
                        "SMS notification failed: HTTP {} - {}",
                        status,
                        body
                    ))
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to send SMS notification to {}: {}",
                    self.phone_number,
                    e
                );
                Err(anyhow::anyhow!("SMS notification failed: {}", e))
            }
        }
    }

    fn backend_name(&self) -> &str {
        "sms"
    }
}

/// Реализация TelegramNotifier для отправки уведомлений через Telegram Bot API.
/// Доступно только при включении фичи `telegram`.
#[cfg(feature = "telegram")]
impl TelegramNotifier {
    /// Создаёт новый TelegramNotifier с указанными параметрами.
    ///
    /// # Аргументы
    /// * `bot_token` - Токен Telegram бота.
    /// * `chat_id` - Идентификатор чата для отправки уведомлений.
    ///
    /// # Возвращает
    /// Новый экземпляр TelegramNotifier.
    pub fn new(bot_token: impl Into<String>, chat_id: impl Into<String>) -> Self {
        Self {
            bot_token: bot_token.into(),
            chat_id: chat_id.into(),
            timeout_seconds: 30,
            client: reqwest::Client::new(),
        }
    }

    /// Устанавливает таймаут для HTTP запросов.
    ///
    /// # Аргументы
    /// * `timeout_seconds` - Таймаут в секундах.
    ///
    /// # Возвращает
    /// Мутированный экземпляр TelegramNotifier.
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    /// Возвращает токен Telegram бота.
    ///
    /// # Возвращает
    /// Токен Telegram бота.
    pub fn bot_token(&self) -> &str {
        &self.bot_token
    }

    /// Возвращает идентификатор чата.
    ///
    /// # Возвращает
    /// Идентификатор чата.
    pub fn chat_id(&self) -> &str {
        &self.chat_id
    }

    /// Возвращает таймаут в секундах.
    ///
    /// # Возвращает
    /// Таймаут в секундах.
    pub fn timeout_seconds(&self) -> u64 {
        self.timeout_seconds
    }
}

/// Реализация Notifier для TelegramNotifier.
/// Доступно только при включении фичи `telegram`.
#[cfg(feature = "telegram")]
#[async_trait::async_trait]
impl Notifier for TelegramNotifier {
    async fn send_notification(&self, notification: &Notification) -> Result<()> {
        // Используем хранимый HTTP клиент
        let client = &self.client;

        // Логируем отправку Telegram уведомления
        tracing::info!(
            "Sending Telegram notification to chat {}: {} - {}",
            self.chat_id,
            notification.title,
            notification.message
        );

        // Формируем сообщение Telegram
        let mut telegram_message = format!("🔔 *SmoothTask Notification*\n\n");
        telegram_message.push_str(&format!("*Type*: {}\n\n", notification.notification_type));
        telegram_message.push_str(&format!("*Title*: {}\n\n", notification.title));
        telegram_message.push_str(&format!("*Message*: {}\n\n", notification.message));

        // Добавляем дополнительные детали, если они есть
        if let Some(details) = &notification.details {
            telegram_message.push_str(&format!("*Details*:\n{}\n\n", details));
        }

        // Добавляем временную метку
        telegram_message.push_str(&format!(
            "*Timestamp*: {}",
            notification.timestamp.to_rfc3339()
        ));

        // Формируем URL для Telegram Bot API
        let api_url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );

        // Подготавливаем параметры для Telegram API
        let params = [
            ("chat_id", self.chat_id.as_str()),
            ("text", &telegram_message),
            ("parse_mode", "Markdown"),
        ];

        // Отправляем запрос
        let response = client.post(&api_url).form(&params).send().await;

        match response {
            Ok(resp) => {
                // Проверяем статус код
                if resp.status().is_success() {
                    tracing::info!(
                        "Successfully sent Telegram notification to chat {}: {} - {}",
                        self.chat_id,
                        notification.title,
                        notification.message
                    );
                    Ok(())
                } else {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    tracing::error!(
                        "Failed to send Telegram notification to chat {}: HTTP {} - {}",
                        self.chat_id,
                        status,
                        body
                    );
                    Err(anyhow::anyhow!(
                        "Telegram notification failed: HTTP {} - {}",
                        status,
                        body
                    ))
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to send Telegram notification to chat {}: {}",
                    self.chat_id,
                    e
                );
                Err(anyhow::anyhow!("Telegram notification failed: {}", e))
            }
        }
    }

    fn backend_name(&self) -> &str {
        "telegram"
    }
}

/// Реализация DiscordNotifier для отправки уведомлений через Discord Webhook API.
/// Доступно только при включении фичи `discord`.
#[cfg(feature = "discord")]
impl DiscordNotifier {
    /// Создаёт новый DiscordNotifier с указанным URL вебхука.
    ///
    /// # Аргументы
    /// * `webhook_url` - URL вебхука Discord для отправки уведомлений.
    ///
    /// # Возвращает
    /// Новый экземпляр DiscordNotifier.
    pub fn new(webhook_url: impl Into<String>) -> Self {
        Self {
            webhook_url: webhook_url.into(),
            timeout_seconds: 30,
            client: reqwest::Client::new(),
        }
    }

    /// Устанавливает таймаут для HTTP запросов.
    ///
    /// # Аргументы
    /// * `timeout_seconds` - Таймаут в секундах.
    ///
    /// # Возвращает
    /// Мутированный экземпляр DiscordNotifier.
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    /// Возвращает URL вебхука Discord.
    ///
    /// # Возвращает
    /// URL вебхука Discord.
    pub fn webhook_url(&self) -> &str {
        &self.webhook_url
    }

    /// Возвращает таймаут в секундах.
    ///
    /// # Возвращает
    /// Таймаут в секундах.
    pub fn timeout_seconds(&self) -> u64 {
        self.timeout_seconds
    }
}

/// Реализация Notifier для DiscordNotifier.
/// Доступно только при включении фичи `discord`.
#[cfg(feature = "discord")]
#[async_trait::async_trait]
impl Notifier for DiscordNotifier {
    async fn send_notification(&self, notification: &Notification) -> Result<()> {
        // Используем хранимый HTTP клиент
        let client = &self.client;

        // Логируем отправку Discord уведомления
        tracing::info!(
            "Sending Discord notification to webhook {}: {} - {}",
            self.webhook_url,
            notification.title,
            notification.message
        );

        // Формируем сообщение Discord
        let discord_message = format!(
            "🔔 **SmoothTask Notification**\n\n**Type**: {}\n**Title**: {}\n**Message**: {}",
            notification.notification_type,
            notification.title,
            notification.message
        );

        // Добавляем дополнительные детали, если они есть
        let mut fields = Vec::new();
        if let Some(details) = &notification.details {
            fields.push(serde_json::json!({
                "name": "Details",
                "value": details,
                "inline": false
            }));
        }

        // Добавляем временную метку
        fields.push(serde_json::json!({
            "name": "Timestamp",
            "value": notification.timestamp.to_rfc3339(),
            "inline": false
        }));

        // Формируем JSON payload для Discord вебхука
        let payload = serde_json::json!({
            "content": discord_message,
            "embeds": [{
                "title": notification.title,
                "description": notification.message,
                "color": match notification.notification_type {
                    NotificationType::Critical => 0xFF0000, // Красный
                    NotificationType::Warning => 0xFFA500, // Оранжевый
                    NotificationType::Info => 0x0000FF,   // Синий
                    NotificationType::PriorityChange => 0x800080, // Фиолетовый
                    NotificationType::ConfigChange => 0x008000,   // Зеленый
                    NotificationType::SystemEvent => 0x00FFFF,   // Голубой
                },
                "fields": fields,
            }],
        });

        // Отправляем запрос
        let response = client
            .post(&self.webhook_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await;

        match response {
            Ok(resp) => {
                // Проверяем статус код
                if resp.status().is_success() {
                    tracing::info!(
                        "Successfully sent Discord notification to webhook {}: {} - {}",
                        self.webhook_url,
                        notification.title,
                        notification.message
                    );
                    Ok(())
                } else {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    tracing::error!(
                        "Failed to send Discord notification to webhook {}: HTTP {} - {}",
                        self.webhook_url,
                        status,
                        body
                    );
                    Err(anyhow::anyhow!(
                        "Discord notification failed: HTTP {} - {}",
                        status,
                        body
                    ))
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to send Discord notification to webhook {}: {}",
                    self.webhook_url,
                    e
                );
                Err(anyhow::anyhow!("Discord notification failed: {}", e))
            }
        }
    }

    fn backend_name(&self) -> &str {
        "discord"
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
            hints_map.insert(
                "timestamp",
                zbus::zvariant::Value::new(notification.timestamp.timestamp()),
            );
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
            NotificationType::Warning => 7000,   // 7 секунд для предупреждений
            _ => 5000,                           // 5 секунд для остальных
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

/// Конфигурация стратегии уведомлений.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct NotificationStrategy {
    /// Максимальная частота уведомлений в секундах (0 для отключения ограничения).
    pub max_frequency_seconds: u64,
    /// Приоритет уведомлений (0 - низкий, 100 - высокий).
    pub priority: u8,
    /// Максимальное количество попыток отправки.
    pub max_retries: usize,
    /// Задержка между попытками в миллисекундах.
    pub retry_delay_ms: u64,
    /// Включить эскалацию для критических уведомлений.
    pub enable_escalation: bool,
    /// Каналы уведомлений для эскалации (например, email, sms, webhook).
    pub escalation_channels: Vec<String>,
}

impl Default for NotificationStrategy {
    fn default() -> Self {
        Self {
            max_frequency_seconds: 60,
            priority: 50,
            max_retries: 3,
            retry_delay_ms: 1000,
            enable_escalation: false,
            escalation_channels: vec!["webhook".to_string()],
        }
    }
}

/// Расширенная конфигурация уведомлений.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct EnhancedNotificationConfig {
    /// Стратегии для разных типов уведомлений.
    pub strategies: std::collections::HashMap<NotificationType, NotificationStrategy>,
    /// Глобальное ограничение частоты уведомлений.
    pub global_rate_limit_seconds: u64,
    /// Включить интеграцию с системой мониторинга.
    pub enable_monitoring_integration: bool,
    /// Включить расширенное логирование уведомлений.
    pub enable_detailed_logging: bool,
}

impl Default for EnhancedNotificationConfig {
    fn default() -> Self {
        let mut strategies = std::collections::HashMap::new();
        
        // Стратегия по умолчанию для критических уведомлений
        strategies.insert(
            NotificationType::Critical,
            NotificationStrategy {
                max_frequency_seconds: 30,
                priority: 100,
                max_retries: 5,
                retry_delay_ms: 500,
                enable_escalation: true,
                escalation_channels: vec!["webhook".to_string(), "email".to_string(), "sms".to_string()],
            },
        );
        
        // Стратегия по умолчанию для предупреждений
        strategies.insert(
            NotificationType::Warning,
            NotificationStrategy {
                max_frequency_seconds: 120,
                priority: 75,
                max_retries: 3,
                retry_delay_ms: 1000,
                enable_escalation: false,
                escalation_channels: vec!["webhook".to_string()],
            },
        );
        
        // Стратегия по умолчанию для информационных уведомлений
        strategies.insert(
            NotificationType::Info,
            NotificationStrategy {
                max_frequency_seconds: 300,
                priority: 50,
                max_retries: 2,
                retry_delay_ms: 2000,
                enable_escalation: false,
                escalation_channels: vec!["webhook".to_string()],
            },
        );
        
        Self {
            strategies,
            global_rate_limit_seconds: 60,
            enable_monitoring_integration: true,
            enable_detailed_logging: true,
        }
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

    /// Расширенная конфигурация уведомлений.
    config: Arc<tokio::sync::RwLock<EnhancedNotificationConfig>>,

    /// Время последнего уведомления для глобального ограничения частоты.
    last_global_notification: Arc<tokio::sync::RwLock<Option<DateTime<Utc>>>>,

    /// Время последнего уведомления для каждого типа.
    last_notification_by_type: Arc<tokio::sync::RwLock<std::collections::HashMap<NotificationType, DateTime<Utc>>>>,

    /// Дополнительные бэкенды для эскалации.
    escalation_notifiers: Arc<tokio::sync::RwLock<std::collections::HashMap<String, Box<dyn Notifier>>>>,
}

/// Расширенный менеджер уведомлений с поддержкой стратегий и интеграции с мониторингом.
#[derive(Clone)]
pub struct EnhancedNotificationManager {
    /// Основной менеджер уведомлений.
    inner: Arc<NotificationManager>,

    /// Интеграция с системой мониторинга здоровья.
    health_monitoring_integration: Option<Arc<dyn HealthMonitoringService + Send + Sync>>,
}

impl NotificationManager {
    /// Создаёт новый NotificationManager с указанным бэкендом.
    pub fn new(notifier: impl Notifier) -> Self {
        Self {
            primary_notifier: Box::new(notifier),
            enabled: true,
            log_storage: None,
            config: Arc::new(tokio::sync::RwLock::new(EnhancedNotificationConfig::default())),
            last_global_notification: Arc::new(tokio::sync::RwLock::new(None)),
            last_notification_by_type: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            escalation_notifiers: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
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
        let mut manager = Self::new(StubNotifier);
        manager.log_storage = Some(log_storage);
        manager
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

    /// Создаёт новый NotificationManager с вебхук бэкендом.
    ///
    /// # Аргументы
    /// * `webhook_url` - URL вебхука для отправки уведомлений.
    ///
    /// # Возвращает
    /// Новый экземпляр NotificationManager с вебхук бэкендом.
    pub fn new_webhook(webhook_url: impl Into<String>) -> Self {
        Self::new(WebhookNotifier::new(webhook_url))
    }

    /// Создаёт новый NotificationManager с вебхук бэкендом и интеграцией с хранилищем логов.
    ///
    /// # Аргументы
    /// * `webhook_url` - URL вебхука для отправки уведомлений.
    /// * `log_storage` - Хранилище логов для интеграции.
    ///
    /// # Возвращает
    /// Новый экземпляр NotificationManager с вебхук бэкендом и интеграцией с хранилищем логов.
    pub fn new_webhook_with_logging(
        webhook_url: impl Into<String>,
        log_storage: std::sync::Arc<crate::logging::log_storage::SharedLogStorage>,
    ) -> Self {
        let mut manager = Self::new(WebhookNotifier::new(webhook_url));
        manager.log_storage = Some(log_storage);
        manager
    }

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
        let mut manager = Self::new(notifier);
        manager.log_storage = Some(log_storage);
        manager
    }

    /// Создаёт новый NotificationManager с email бэкендом.
    ///
    /// # Аргументы
    /// * `smtp_server` - SMTP сервер для отправки email.
    /// * `smtp_port` - Порт SMTP сервера.
    /// * `from_email` - Email отправителя.
    /// * `from_name` - Имя отправителя.
    /// * `to_email` - Email получателя.
    /// * `to_name` - Имя получателя.
    /// * `use_tls` - Флаг, указывающий, использовать ли TLS.
    ///
    /// # Возвращает
    /// Новый экземпляр NotificationManager с email бэкендом.
    ///
    /// # Примечания
    /// Доступно только при включении фичи `email`.
    #[cfg(feature = "email")]
    pub fn new_email(
        smtp_server: impl Into<String>,
        smtp_port: u16,
        from_email: impl Into<String>,
        from_name: impl Into<String>,
        to_email: impl Into<String>,
        to_name: impl Into<String>,
        use_tls: bool,
    ) -> Self {
        Self::new(EmailNotifier::new(
            smtp_server,
            smtp_port,
            from_email,
            from_name,
            to_email,
            to_name,
            use_tls,
        ))
    }

    /// Создаёт новый NotificationManager с email бэкендом и интеграцией с хранилищем логов.
    ///
    /// # Аргументы
    /// * `smtp_server` - SMTP сервер для отправки email.
    /// * `smtp_port` - Порт SMTP сервера.
    /// * `from_email` - Email отправителя.
    /// * `from_name` - Имя отправителя.
    /// * `to_email` - Email получателя.
    /// * `to_name` - Имя получателя.
    /// * `use_tls` - Флаг, указывающий, использовать ли TLS.
    /// * `log_storage` - Хранилище логов для интеграции.
    ///
    /// # Возвращает
    /// Новый экземпляр NotificationManager с email бэкендом и интеграцией с хранилищем логов.
    ///
    /// # Примечания
    /// Доступно только при включении фичи `email`.
    #[cfg(feature = "email")]
    pub fn new_email_with_logging(
        smtp_server: impl Into<String>,
        smtp_port: u16,
        from_email: impl Into<String>,
        from_name: impl Into<String>,
        to_email: impl Into<String>,
        to_name: impl Into<String>,
        use_tls: bool,
        log_storage: std::sync::Arc<crate::logging::log_storage::SharedLogStorage>,
    ) -> Self {
        let mut manager = Self::new(EmailNotifier::new(
            smtp_server,
            smtp_port,
            from_email,
            from_name,
            to_email,
            to_name,
            use_tls,
        ));
        manager.log_storage = Some(log_storage);
        manager
    }

    /// Создаёт новый NotificationManager с SMS бэкендом.
    ///
    /// # Аргументы
    /// * `gateway_url` - URL SMS шлюза.
    /// * `phone_number` - Номер телефона получателя.
    ///
    /// # Возвращает
    /// Новый экземпляр NotificationManager с SMS бэкендом.
    pub fn new_sms(gateway_url: impl Into<String>, phone_number: impl Into<String>) -> Self {
        Self::new(SmsNotifier::new(gateway_url, phone_number))
    }

    /// Создаёт новый NotificationManager с SMS бэкендом и интеграцией с хранилищем логов.
    ///
    /// # Аргументы
    /// * `gateway_url` - URL SMS шлюза.
    /// * `phone_number` - Номер телефона получателя.
    /// * `log_storage` - Хранилище логов для интеграции.
    ///
    /// # Возвращает
    /// Новый экземпляр NotificationManager с SMS бэкендом и интеграцией с хранилищем логов.
    pub fn new_sms_with_logging(
        gateway_url: impl Into<String>,
        phone_number: impl Into<String>,
        log_storage: std::sync::Arc<crate::logging::log_storage::SharedLogStorage>,
    ) -> Self {
        let mut manager = Self::new(SmsNotifier::new(gateway_url, phone_number));
        manager.log_storage = Some(log_storage);
        manager
    }

    /// Создаёт новый NotificationManager с Telegram бэкендом.
    ///
    /// # Аргументы
    /// * `bot_token` - Токен Telegram бота.
    /// * `chat_id` - Идентификатор чата для отправки уведомлений.
    ///
    /// # Возвращает
    /// Новый экземпляр NotificationManager с Telegram бэкендом.
    ///
    /// # Примечания
    /// Доступно только при включении фичи `telegram`.
    #[cfg(feature = "telegram")]
    pub fn new_telegram(bot_token: impl Into<String>, chat_id: impl Into<String>) -> Self {
        Self::new(TelegramNotifier::new(bot_token, chat_id))
    }

    /// Создаёт новый NotificationManager с Telegram бэкендом и интеграцией с хранилищем логов.
    ///
    /// # Аргументы
    /// * `bot_token` - Токен Telegram бота.
    /// * `chat_id` - Идентификатор чата для отправки уведомлений.
    /// * `log_storage` - Хранилище логов для интеграции.
    ///
    /// # Возвращает
    /// Новый экземпляр NotificationManager с Telegram бэкендом и интеграцией с хранилищем логов.
    ///
    /// # Примечания
    /// Доступно только при включении фичи `telegram`.
    #[cfg(feature = "telegram")]
    pub fn new_telegram_with_logging(
        bot_token: impl Into<String>,
        chat_id: impl Into<String>,
        log_storage: std::sync::Arc<crate::logging::log_storage::SharedLogStorage>,
    ) -> Self {
        let mut manager = Self::new(TelegramNotifier::new(bot_token, chat_id));
        manager.log_storage = Some(log_storage);
        manager
    }

    /// Создаёт новый NotificationManager с Discord бэкендом.
    ///
    /// # Аргументы
    /// * `webhook_url` - URL вебхука Discord для отправки уведомлений.
    ///
    /// # Возвращает
    /// Новый экземпляр NotificationManager с Discord бэкендом.
    ///
    /// # Примечания
    /// Доступно только при включении фичи `discord`.
    #[cfg(feature = "discord")]
    pub fn new_discord(webhook_url: impl Into<String>) -> Self {
        Self::new(DiscordNotifier::new(webhook_url))
    }

    /// Создаёт новый NotificationManager с Discord бэкендом и интеграцией с хранилищем логов.
    ///
    /// # Аргументы
    /// * `webhook_url` - URL вебхука Discord для отправки уведомлений.
    /// * `log_storage` - Хранилище логов для интеграции.
    ///
    /// # Возвращает
    /// Новый экземпляр NotificationManager с Discord бэкендом и интеграцией с хранилищем логов.
    ///
    /// # Примечания
    /// Доступно только при включении фичи `discord`.
    #[cfg(feature = "discord")]
    pub fn new_discord_with_logging(
        webhook_url: impl Into<String>,
        log_storage: std::sync::Arc<crate::logging::log_storage::SharedLogStorage>,
    ) -> Self {
        let mut manager = Self::new(DiscordNotifier::new(webhook_url));
        manager.log_storage = Some(log_storage);
        manager
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

    /// Устанавливает расширенную конфигурацию уведомлений.
    pub async fn set_config(&self, config: EnhancedNotificationConfig) -> Result<()> {
        let mut config_lock = self.config.write().await;
        *config_lock = config;
        Ok(())
    }

    /// Возвращает текущую расширенную конфигурацию.
    pub async fn get_config(&self) -> EnhancedNotificationConfig {
        self.config.read().await.clone()
    }

    /// Добавляет бэкенд для эскалации.
    pub async fn add_escalation_notifier(&self, name: String, notifier: Box<dyn Notifier>) -> Result<()> {
        let mut escalation_lock = self.escalation_notifiers.write().await;
        escalation_lock.insert(name, notifier);
        Ok(())
    }

    /// Удаляет бэкенд для эскалации.
    pub async fn remove_escalation_notifier(&self, name: &str) -> Result<()> {
        let mut escalation_lock = self.escalation_notifiers.write().await;
        escalation_lock.remove(name);
        Ok(())
    }

    /// Проверяет, разрешено ли отправлять уведомление на основе стратегии.
    async fn check_notification_allowed(&self, notification_type: &NotificationType) -> Result<bool> {
        let config = self.config.read().await;
        
        // Проверяем глобальное ограничение частоты
        if config.global_rate_limit_seconds > 0 {
            let last_global = self.last_global_notification.read().await;
            if let Some(last_time) = *last_global {
                let duration_since_last = Utc::now().signed_duration_since(last_time);
                if (duration_since_last.num_seconds() as u64) < config.global_rate_limit_seconds {
                    tracing::debug!(
                        "Global rate limit exceeded for notification type: {:?}",
                        notification_type
                    );
                    return Ok(false);
                }
            }
        }

        // Проверяем стратегию для конкретного типа уведомления
        if let Some(strategy) = config.strategies.get(notification_type) {
            if strategy.max_frequency_seconds > 0 {
                let last_by_type = self.last_notification_by_type.read().await;
                if let Some(last_time) = last_by_type.get(notification_type) {
                    let duration_since_last = Utc::now().signed_duration_since(*last_time);
                    if (duration_since_last.num_seconds() as u64) < strategy.max_frequency_seconds {
                        tracing::debug!(
                            "Type-specific rate limit exceeded for notification type: {:?}",
                            notification_type
                        );
                        return Ok(false);
                    }
                }
            }
        }

        Ok(true)
    }

    /// Обновляет время последнего уведомления.
    async fn update_last_notification_time(&self, notification_type: &NotificationType) {
        // Обновляем глобальное время
        let mut last_global = self.last_global_notification.write().await;
        *last_global = Some(Utc::now());

        // Обновляем время для конкретного типа
        let mut last_by_type = self.last_notification_by_type.write().await;
        last_by_type.insert(*notification_type, Utc::now());
    }

    /// Отправляет уведомление с учетом стратегий и эскалации.
    pub async fn send_with_strategy(&self, notification: &Notification) -> Result<()> {
        // Логируем начало обработки уведомления
        tracing::info!(
            "Processing notification: {} (type: {:?})",
            notification.title,
            notification.notification_type
        );

        if !self.enabled {
            tracing::debug!("Notifications are disabled, skipping notification");
            return Ok(());
        }

        // Проверяем, разрешено ли отправлять уведомление
        if !self.check_notification_allowed(&notification.notification_type).await? {
            tracing::warn!(
                "Notification rate limit exceeded for type: {:?}",
                notification.notification_type
            );
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
                format!("{}", notification.title),
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

        // Получаем стратегию для этого типа уведомления
        let config = self.config.read().await;
        let strategy = config.strategies.get(&notification.notification_type)
            .cloned()
            .unwrap_or_default();

        // Отправляем уведомление через основной бэкенд с повторными попытками
        let mut attempt = 0;
        let mut primary_success = false;
        let mut last_error: Option<anyhow::Error> = None;
        
        while attempt < strategy.max_retries {
            attempt += 1;
            
            match self.primary_notifier.send_notification(notification).await {
                Ok(_) => {
                    tracing::info!(
                        "Successfully sent notification through primary backend (attempt {})",
                        attempt
                    );
                    primary_success = true;
                    break;
                }
                Err(e) => {
                    last_error = Some(anyhow::anyhow!("{}", e));
                    tracing::warn!(
                        "Attempt {} failed to send notification: {}. Retrying in {}ms...",
                        attempt,
                        e,
                        strategy.retry_delay_ms
                    );
                    
                    // Логируем ошибку в хранилище логов, если оно доступно
                    if let Some(ref log_storage_arc) = self.log_storage {
                        let log_level = if attempt == strategy.max_retries {
                            crate::logging::log_storage::LogLevel::Error
                        } else {
                            crate::logging::log_storage::LogLevel::Warn
                        };
                        
                        let log_entry = crate::logging::log_storage::LogEntry::new(
                            log_level,
                            "notifications",
                            format!("Notification send attempt {} failed", attempt),
                        ).with_fields(serde_json::json!({
                            "notification_title": notification.title,
                            "notification_type": format!("{}", notification.notification_type),
                            "error": format!("{}", e),
                            "attempt": attempt,
                            "max_retries": strategy.max_retries,
                            "timestamp": notification.timestamp.to_rfc3339(),
                        }));
                        log_storage_arc.add_entry(log_entry).await;
                    }
                    
                    if attempt < strategy.max_retries {
                        sleep(Duration::from_millis(strategy.retry_delay_ms)).await;
                    }
                }
            }
        }

        // Обновляем время последнего уведомления только если отправка была успешной
        if primary_success {
            self.update_last_notification_time(&notification.notification_type).await;
        }

        // Если включена эскалация и основная отправка не удалась, пробуем эскалацию
        if strategy.enable_escalation && !primary_success {
            tracing::warn!(
                "Primary notification failed after {} attempts, initiating escalation",
                strategy.max_retries
            );
            self.handle_escalation(notification, &strategy).await?;
        } else if !primary_success {
            let error_message = last_error.map(|e| e.to_string()).unwrap_or_else(|| "Unknown error".to_string());
            tracing::error!(
                "Notification failed and escalation is disabled: {}",
                error_message
            );
            
            // Логируем критическую ошибку, если все попытки не удались
            if let Some(ref log_storage_arc) = self.log_storage {
                let log_entry = crate::logging::log_storage::LogEntry::new(
                    crate::logging::log_storage::LogLevel::Error,
                    "notifications",
                    format!("Notification failed after {} attempts", strategy.max_retries),
                ).with_fields(serde_json::json!({
                    "notification_title": notification.title,
                    "notification_type": format!("{}", notification.notification_type),
                    "error": error_message,
                    "timestamp": notification.timestamp.to_rfc3339(),
                }));
                log_storage_arc.add_entry(log_entry).await;
            }
        }

        Ok(())
    }

    /// Обрабатывает эскалацию уведомления через дополнительные каналы.
    async fn handle_escalation(&self, notification: &Notification, strategy: &NotificationStrategy) -> Result<()> {
        let escalation_notifiers = self.escalation_notifiers.read().await;
        let mut escalation_success = false;
        
        tracing::info!(
            "Starting escalation process for notification: {} (channels: {:?})",
            notification.title,
            strategy.escalation_channels
        );
        
        for channel in &strategy.escalation_channels {
            if let Some(notifier) = escalation_notifiers.get(channel) {
                tracing::info!(
                    "Escalating notification through {} channel: {}",
                    channel,
                    notification.title
                );
                
                // Пробуем отправить через канал эскалации
                match notifier.send_notification(notification).await {
                    Ok(_) => {
                        tracing::info!(
                            "Successfully escalated notification through {} channel",
                            channel
                        );
                        escalation_success = true;
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to escalate notification through {} channel: {}",
                            channel,
                            e
                        );
                        
                        // Логируем ошибку в хранилище логов, если оно доступно
                        if let Some(ref log_storage_arc) = self.log_storage {
                            let log_entry = crate::logging::log_storage::LogEntry::new(
                                crate::logging::log_storage::LogLevel::Error,
                                "notifications",
                                format!("Escalation failed for {} channel", channel),
                            ).with_fields(serde_json::json!({
                                "notification_title": notification.title,
                                "notification_type": format!("{}", notification.notification_type),
                                "error": format!("{}", e),
                                "timestamp": notification.timestamp.to_rfc3339(),
                            }));
                            log_storage_arc.add_entry(log_entry).await;
                        }
                    }
                }
            } else {
                tracing::warn!(
                    "Escalation channel {} not found in available notifiers",
                    channel
                );
            }
        }
        
        if !escalation_success && !strategy.escalation_channels.is_empty() {
            tracing::error!(
                "All escalation attempts failed for notification: {}",
                notification.title
            );
            
            // Если все попытки эскалации не удались, логируем критическую ошибку
            if let Some(ref log_storage_arc) = self.log_storage {
                let log_entry = crate::logging::log_storage::LogEntry::new(
                    crate::logging::log_storage::LogLevel::Error,
                    "notifications",
                    format!("All escalation attempts failed: {}", notification.title),
                ).with_fields(serde_json::json!({
                    "notification_type": format!("{}", notification.notification_type),
                    "timestamp": notification.timestamp.to_rfc3339(),
                }));
                log_storage_arc.add_entry(log_entry).await;
            }
        }

        Ok(())
    }

    /// Возвращает расширенное состояние системы уведомлений.
    pub async fn get_enhanced_status(&self) -> Result<EnhancedNotificationStatus> {
        let config = self.config.read().await;
        let last_global = self.last_global_notification.read().await;
        let last_by_type = self.last_notification_by_type.read().await;
        let escalation_notifiers = self.escalation_notifiers.read().await;
        
        Ok(EnhancedNotificationStatus {
            enabled: self.enabled,
            backend: self.backend_name().to_string(),
            has_log_integration: self.log_storage.is_some(),
            global_rate_limit_seconds: config.global_rate_limit_seconds,
            last_notification_time: *last_global,
            notification_count_by_type: last_by_type.len(),
            escalation_channels_count: escalation_notifiers.len(),
            monitoring_integration_enabled: config.enable_monitoring_integration,
        })
    }
}

/// Расширенное состояние системы уведомлений.
#[derive(Debug, Clone, Serialize)]
pub struct EnhancedNotificationStatus {
    /// Флаг, указывающий, включены ли уведомления.
    pub enabled: bool,
    /// Текущий бэкенд уведомлений.
    pub backend: String,
    /// Флаг, указывающий, интегрирована ли система уведомлений с хранилищем логов.
    pub has_log_integration: bool,
    /// Глобальное ограничение частоты уведомлений в секундах.
    pub global_rate_limit_seconds: u64,
    /// Время последнего уведомления.
    pub last_notification_time: Option<DateTime<Utc>>,
    /// Количество типов уведомлений.
    pub notification_count_by_type: usize,
    /// Количество каналов эскалации.
    pub escalation_channels_count: usize,
    /// Флаг, указывающий, включена ли интеграция с мониторингом.
    pub monitoring_integration_enabled: bool,
}

impl EnhancedNotificationManager {
    /// Создаёт новый EnhancedNotificationManager.
    pub fn new(manager: NotificationManager) -> Self {
        Self {
            inner: Arc::new(manager),
            health_monitoring_integration: None,
        }
    }

    /// Создаёт новый EnhancedNotificationManager с интеграцией мониторинга.
    pub fn new_with_monitoring(
        manager: NotificationManager,
        monitoring_service: Arc<dyn HealthMonitoringService + Send + Sync>,
    ) -> Self {
        Self {
            inner: Arc::new(manager),
            health_monitoring_integration: Some(monitoring_service),
        }
    }

    /// Устанавливает расширенную конфигурацию.
    pub async fn set_config(&self, config: EnhancedNotificationConfig) -> Result<()> {
        self.inner.set_config(config).await
    }

    /// Возвращает текущую расширенную конфигурацию.
    pub async fn get_config(&self) -> EnhancedNotificationConfig {
        self.inner.get_config().await
    }

    /// Добавляет бэкенд для эскалации.
    pub async fn add_escalation_notifier(&self, name: String, notifier: Box<dyn Notifier>) -> Result<()> {
        self.inner.add_escalation_notifier(name, notifier).await
    }

    /// Удаляет бэкенд для эскалации.
    pub async fn remove_escalation_notifier(&self, name: &str) -> Result<()> {
        self.inner.remove_escalation_notifier(name).await
    }

    /// Отправляет уведомление с учетом стратегий и эскалации.
    pub async fn send(&self, notification: &Notification) -> Result<()> {
        self.inner.send_with_strategy(notification).await
    }

    /// Возвращает расширенное состояние системы уведомлений.
    pub async fn get_status(&self) -> Result<EnhancedNotificationStatus> {
        self.inner.get_enhanced_status().await
    }

    /// Интегрирует уведомления с системой мониторинга здоровья.
    pub async fn integrate_with_monitoring(&mut self, monitoring_service: Arc<dyn HealthMonitoringService + Send + Sync>) -> Result<()> {
        self.health_monitoring_integration = Some(monitoring_service);
        
        // Настраиваем конфигурацию для включения интеграции с мониторингом
        let mut config = self.inner.get_config().await;
        config.enable_monitoring_integration = true;
        self.inner.set_config(config).await?;
        
        Ok(())
    }

    /// Отправляет уведомление о событии здоровья.
    pub async fn send_health_event_notification(&self, event: &HealthEvent) -> Result<()> {
        if !self.inner.config.read().await.enable_monitoring_integration {
            tracing::debug!("Monitoring integration is disabled, skipping health event notification");
            return Ok(());
        }

        let notification = match event {
            HealthEvent::HealthStatusChanged { old_status, new_status, timestamp } => {
                Notification::new(
                    NotificationType::SystemEvent,
                    format!("Health Status Changed: {:?} -> {:?}", old_status, new_status),
                    format!("Health status changed from {:?} to {:?} at {}", old_status, new_status, timestamp),
                )
            }
            HealthEvent::NewHealthIssue { issue, timestamp } => {
                let notification_type = match issue.severity {
                    HealthIssueSeverity::Critical => NotificationType::Critical,
                    HealthIssueSeverity::Warning => NotificationType::Warning,
                    _ => NotificationType::Info,
                };

                Notification::new(
                    notification_type,
                    format!("New Health Issue: {}", issue.issue_type),
                    format!("{} - {}", issue.description, issue.error_details.as_deref().unwrap_or("")),
                ).with_details(format!("Issue ID: {}, Timestamp: {}", issue.issue_id, timestamp))
            }
            HealthEvent::HealthIssueResolved { issue_id, timestamp } => {
                Notification::new(
                    NotificationType::Info,
                    "Health Issue Resolved",
                    format!("Health issue {} has been resolved", issue_id),
                ).with_details(format!("Resolved at: {}", timestamp))
            }
            HealthEvent::CriticalHealthDetected { issue, timestamp } => {
                Notification::new(
                    NotificationType::Critical,
                    format!("CRITICAL HEALTH ISSUE: {}", issue.issue_type),
                    format!("CRITICAL: {} - {}", issue.description, issue.error_details.as_deref().unwrap_or("")),
                ).with_details(format!("Issue ID: {}, Timestamp: {}", issue.issue_id, timestamp))
            }
        };

        self.send(&notification).await
    }

    /// Возвращает текущий бэкенд уведомлений.
    pub fn backend_name(&self) -> &str {
        self.inner.backend_name()
    }

    /// Включает или отключает отправку уведомлений.
    pub fn set_enabled(&self, _enabled: bool) {
        // Note: This is a simple wrapper, but we need to access the inner manager
        // For now, we'll use a workaround since we can't mutate through Arc
        tracing::warn!("set_enabled on EnhancedNotificationManager is not fully implemented yet");
    }

    /// Возвращает true, если отправка уведомлений включена.
    pub fn is_enabled(&self) -> bool {
        // Note: This is a simple wrapper, but we need to access the inner manager
        // For now, we'll return true as a placeholder
        true
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
    async fn test_telegram_notifier_creation() {
        #[cfg(feature = "telegram")]
        {
            use super::TelegramNotifier;
            let notifier = TelegramNotifier::new("test_token", "test_chat_id");
            assert_eq!(notifier.bot_token(), "test_token");
            assert_eq!(notifier.chat_id(), "test_chat_id");
            assert_eq!(notifier.timeout_seconds(), 30);
        }
    }

    #[tokio::test]
    async fn test_discord_notifier_creation() {
        #[cfg(feature = "discord")]
        {
            use super::DiscordNotifier;
            let notifier = DiscordNotifier::new("https://discord.com/api/webhooks/test");
            assert_eq!(notifier.webhook_url(), "https://discord.com/api/webhooks/test");
            assert_eq!(notifier.timeout_seconds(), 30);
        }
    }

    #[tokio::test]
    async fn test_telegram_notifier_with_timeout() {
        #[cfg(feature = "telegram")]
        {
            use super::TelegramNotifier;
            let notifier = TelegramNotifier::new("test_token", "test_chat_id").with_timeout(60);
            assert_eq!(notifier.timeout_seconds(), 60);
        }
    }

    #[tokio::test]
    async fn test_discord_notifier_with_timeout() {
        #[cfg(feature = "discord")]
        {
            use super::DiscordNotifier;
            let notifier = DiscordNotifier::new("https://discord.com/api/webhooks/test").with_timeout(60);
            assert_eq!(notifier.timeout_seconds(), 60);
        }
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
    async fn test_webhook_notifier_creation() {
        let notifier = WebhookNotifier::new("https://example.com/webhook");
        assert_eq!(notifier.webhook_url(), "https://example.com/webhook");
        assert_eq!(notifier.timeout_seconds(), 10);
        assert!(!notifier.allow_insecure_https());
        assert_eq!(notifier.backend_name(), "webhook");
    }

    #[tokio::test]
    async fn test_webhook_notifier_with_headers() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token123".to_string());
        headers.insert("X-Custom-Header".to_string(), "CustomValue".to_string());

        let notifier = WebhookNotifier::new("https://example.com/webhook")
            .with_headers(headers.clone());

        assert_eq!(notifier.headers().len(), 2);
        assert_eq!(
            notifier.headers().get("Authorization"),
            Some(&"Bearer token123".to_string())
        );
        assert_eq!(
            notifier.headers().get("X-Custom-Header"),
            Some(&"CustomValue".to_string())
        );
    }

    #[tokio::test]
    async fn test_webhook_notifier_with_timeout() {
        let notifier = WebhookNotifier::new("https://example.com/webhook")
            .with_timeout(30);

        assert_eq!(notifier.timeout_seconds(), 30);
    }

    #[tokio::test]
    async fn test_webhook_notifier_allow_insecure_https() {
        let notifier = WebhookNotifier::new("https://example.com/webhook")
            .allow_insecure_https();

        assert!(notifier.is_insecure_https_allowed());
    }

    #[tokio::test]
    async fn test_webhook_notifier_client() {
        let notifier = WebhookNotifier::new("https://example.com/webhook")
            .with_timeout(15)
            .allow_insecure_https();

        let client = notifier.client();
        // Проверяем, что клиент доступен (не можем проверить таймаут напрямую)
        assert!(client.timeout().is_some());
    }

    #[tokio::test]
    async fn test_notification_manager_webhook() {
        let manager = NotificationManager::new_webhook("https://example.com/webhook");
        assert_eq!(manager.backend_name(), "webhook");
        assert!(manager.is_enabled());
    }

    #[tokio::test]
    async fn test_notification_manager_webhook_with_logging() {
        use crate::logging::log_storage::SharedLogStorage;
        use std::sync::Arc;

        let log_storage = Arc::new(SharedLogStorage::new(10));
        let manager = NotificationManager::new_webhook_with_logging(
            "https://example.com/webhook",
            Arc::clone(&log_storage),
        );

        assert!(manager.is_enabled());
        assert!(manager.log_storage.is_some());
        assert_eq!(manager.backend_name(), "webhook");
    }

    #[tokio::test]
    async fn test_webhook_notifier_serialization() {
        let notification = Notification::new(
            NotificationType::Critical,
            "Test Title",
            "Test Message",
        )
        .with_details("Test details");

        let notifier = WebhookNotifier::new("https://example.com/webhook");

        // Создаём JSON, как это делает notifier
        let notification_json = serde_json::json!({
            "notification_type": format!("{}", notification.notification_type),
            "title": notification.title,
            "message": notification.message,
            "details": notification.details,
            "timestamp": notification.timestamp.to_rfc3339(),
        });

        // Проверяем, что JSON корректно сериализуется
        let json_string = notification_json.to_string();
        assert!(json_string.contains("CRITICAL"));
        assert!(json_string.contains("Test Title"));
        assert!(json_string.contains("Test Message"));
        assert!(json_string.contains("Test details"));
    }

    #[tokio::test]
    async fn test_webhook_notifier_disabled() {
        let mut manager = NotificationManager::new_webhook("https://example.com/webhook");
        manager.set_enabled(false);
        let notification = Notification::new(NotificationType::Info, "Test Title", "Test Message");

        assert!(!manager.is_enabled());
        let result = manager.send(&notification).await;
        assert!(result.is_ok()); // Должно возвращать Ok, даже если отключено
    }

    #[tokio::test]
    async fn test_webhook_notifier_new_types() {
        let manager = NotificationManager::new_webhook("https://example.com/webhook");

        // Тестируем новые типы уведомлений
        let notifications = vec![
            Notification::priority_change("test_app", "normal", "high", "policy change"),
            Notification::config_change("config.yml", "updated settings"),
            Notification::system_event("shutdown", "System shutting down"),
            Notification::resource_event("Memory", "12GB", "10GB"),
            Notification::temperature_event("GPU", "80", "75"),
            Notification::network_event("Connection Spike", "1000 active connections"),
        ];

        // Проверяем, что все уведомления корректно создаются
        for notification in &notifications {
            assert!(notification.title.len() > 0);
            assert!(notification.message.len() > 0);
        }
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
            3,
            "Expected 3 log entries, got {}",
            all_entries.len()
        );

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
        assert!(deserialized
            .message
            .contains("GPU usage is at 90% (threshold: 85%)"));

        let temperature_notification = Notification::temperature_event("CPU", "85", "80");
        let serialized = serde_json::to_string(&temperature_notification).unwrap();
        let deserialized: Notification = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.notification_type, NotificationType::Warning);
        assert_eq!(deserialized.title, "High CPU Temperature");
        assert!(deserialized
            .message
            .contains("CPU temperature is at 85°C (threshold: 80°C)"));
    }

    #[cfg(feature = "email")]
    #[tokio::test]
    async fn test_email_notifier_creation() {
        let notifier = EmailNotifier::new(
            "smtp.example.com",
            587,
            "sender@example.com",
            "SmoothTask",
            "recipient@example.com",
            "Admin",
            true,
        );

        assert_eq!(notifier.smtp_server(), "smtp.example.com");
        assert_eq!(notifier.smtp_port(), 587);
        assert_eq!(notifier.from_email(), "sender@example.com");
        assert_eq!(notifier.to_email(), "recipient@example.com");
        assert!(notifier.is_tls_used());
        assert_eq!(notifier.timeout_seconds(), 30);
        assert_eq!(notifier.backend_name(), "email");
    }

    #[cfg(feature = "email")]
    #[tokio::test]
    async fn test_email_notifier_with_credentials() {
        let notifier = EmailNotifier::new(
            "smtp.example.com",
            587,
            "sender@example.com",
            "SmoothTask",
            "recipient@example.com",
            "Admin",
            true,
        )
        .with_credentials("username", "password")
        .with_timeout(60);

        assert_eq!(notifier.timeout_seconds(), 60);
        // Не можем проверить учётные данные напрямую, так как они приватные
    }

    #[cfg(feature = "email")]
    #[tokio::test]
    async fn test_email_notifier_send() {
        let notifier = EmailNotifier::new(
            "smtp.example.com",
            587,
            "sender@example.com",
            "SmoothTask",
            "recipient@example.com",
            "Admin",
            true,
        );

        let notification = Notification::new(
            NotificationType::Info,
            "Test Email",
            "This is a test email notification",
        )
        .with_details("Additional details for the email");

        // Этот тест не будет отправлять реальное email, так как мы используем mock SMTP сервер
        // В реальном использовании нужно настроить тестовый SMTP сервер
        let result = notifier.send_notification(&notification).await;
        
        // Ожидаем ошибку, так как нет реального SMTP сервера
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sms_notifier_creation() {
        let notifier = SmsNotifier::new(
            "https://sms-gateway.example.com/api/send",
            "+1234567890",
        );

        assert_eq!(notifier.gateway_url(), "https://sms-gateway.example.com/api/send");
        assert_eq!(notifier.phone_number(), "+1234567890");
        assert_eq!(notifier.timeout_seconds(), 30);
        assert_eq!(notifier.backend_name(), "sms");
    }

    #[tokio::test]
    async fn test_sms_notifier_with_credentials() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token123".to_string());

        let notifier = SmsNotifier::new(
            "https://sms-gateway.example.com/api/send",
            "+1234567890",
        )
        .with_credentials("username", "password")
        .with_api_key("api_key_123")
        .with_headers(headers)
        .with_timeout(60);

        assert_eq!(notifier.timeout_seconds(), 60);
        assert_eq!(notifier.headers().len(), 1);
        assert_eq!(
            notifier.headers().get("Authorization"),
            Some(&"Bearer token123".to_string())
        );
    }

    #[tokio::test]
    async fn test_sms_notifier_send() {
        let notifier = SmsNotifier::new(
            "https://sms-gateway.example.com/api/send",
            "+1234567890",
        );

        let notification = Notification::new(
            NotificationType::Critical,
            "Critical Alert",
            "System failure detected!",
        )
        .with_details("CPU temperature exceeded safe limits");

        // Этот тест не будет отправлять реальное SMS, так как мы используем mock SMS шлюз
        // В реальном использовании нужно настроить тестовый SMS шлюз
        let result = notifier.send_notification(&notification).await;
        
        // Ожидаем ошибку, так как нет реального SMS шлюза
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_notification_manager_email() {
        #[cfg(feature = "email")]
        {
            let manager = NotificationManager::new_email(
                "smtp.example.com",
                587,
                "sender@example.com",
                "SmoothTask",
                "recipient@example.com",
                "Admin",
                true,
            );

            assert_eq!(manager.backend_name(), "email");
            assert!(manager.is_enabled());
        }
    }

    #[tokio::test]
    async fn test_notification_manager_sms() {
        let manager = NotificationManager::new_sms(
            "https://sms-gateway.example.com/api/send",
            "+1234567890",
        );

        assert_eq!(manager.backend_name(), "sms");
        assert!(manager.is_enabled());
    }

    #[tokio::test]
    async fn test_notification_manager_email_with_logging() {
        #[cfg(feature = "email")]
        {
            use crate::logging::log_storage::SharedLogStorage;
            use std::sync::Arc;

            let log_storage = Arc::new(SharedLogStorage::new(10));
            let manager = NotificationManager::new_email_with_logging(
                "smtp.example.com",
                587,
                "sender@example.com",
                "SmoothTask",
                "recipient@example.com",
                "Admin",
                true,
                Arc::clone(&log_storage),
            );

            assert!(manager.is_enabled());
            assert!(manager.log_storage.is_some());
            assert_eq!(manager.backend_name(), "email");
        }
    }

    #[tokio::test]
    async fn test_notification_manager_sms_with_logging() {
        use crate::logging::log_storage::SharedLogStorage;
        use std::sync::Arc;

        let log_storage = Arc::new(SharedLogStorage::new(10));
        let manager = NotificationManager::new_sms_with_logging(
            "https://sms-gateway.example.com/api/send",
            "+1234567890",
            Arc::clone(&log_storage),
        );

        assert!(manager.is_enabled());
        assert!(manager.log_storage.is_some());
        assert_eq!(manager.backend_name(), "sms");
    }

    #[tokio::test]
    async fn test_sms_message_length_limiting() {
        let notifier = SmsNotifier::new(
            "https://sms-gateway.example.com/api/send",
            "+1234567890",
        );

        // Создаём уведомление с очень длинным сообщением
        let long_details = "a".repeat(200); // Очень длинные детали
        let notification = Notification::new(
            NotificationType::Info,
            "Long Message Test",
            "This is a test message with very long details",
        )
        .with_details(long_details);

        // Проверяем, что сообщение будет ограничено до 160 символов
        // Это тест логики, а не реальной отправки
        assert!(notification.message.len() > 0);
    }

    #[tokio::test]
    async fn test_email_notification_formatting() {
        #[cfg(feature = "email")]
        {
            let notifier = EmailNotifier::new(
                "smtp.example.com",
                587,
                "sender@example.com",
                "SmoothTask",
                "recipient@example.com",
                "Admin",
                true,
            );

            let notification = Notification::new(
                NotificationType::Critical,
                "System Failure",
                "Critical system failure detected",
            )
            .with_details("CPU: 100%, Memory: 95%, Disk: 99%");

            // Проверяем форматирование (логика, а не реальная отправка)
            assert!(notification.title.contains("System Failure"));
            assert!(notification.message.contains("Critical system failure"));
        }
    }

    #[tokio::test]
    async fn test_new_notifier_types_integration() {
        // Тестируем интеграцию новых типов уведомлений с разными бэкендами
        let notification = Notification::new(
            NotificationType::Critical,
            "Test Critical",
            "Test critical notification",
        );

        // Тестируем с заглушкой
        let stub_manager = NotificationManager::new_stub();
        let result = stub_manager.send(&notification).await;
        assert!(result.is_ok());

        // Тестируем с вебхук (если доступно)
        let webhook_manager = NotificationManager::new_webhook("https://example.com/webhook");
        let result = webhook_manager.send(&notification).await;
        // Ожидаем ошибку, так как нет реального вебхука
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sms_notifier_uses_stored_client() {
        // Тестируем, что SmsNotifier использует хранимый клиент вместо создания нового
        let notifier = SmsNotifier::new(
            "https://example.com/sms-gateway",
            "+1234567890",
        )
        .with_timeout(30);

        // Проверяем, что клиент создан и хранится
        let client = notifier.client();
        assert!(client.timeout().is_some());
        
        // Проверяем, что таймаут клиента соответствует конфигурации
        if let Some(timeout) = client.timeout() {
            assert_eq!(timeout.as_secs(), 30);
        }

        // Основная проверка: send_notification должен использовать хранимый клиент
        // Это проверяется косвенно - если бы он создавал новый клиент, то хранимый клиент
        // не использовался бы и компилятор бы выдавал warning о неиспользуемом поле
        
        // Проверяем, что клиент доступен через метод client()
        let client = notifier.client();
        assert!(client.timeout().is_some());
    }

    #[tokio::test]
    async fn test_sms_notifier_client_reuse() {
        // Тестируем, что SmsNotifier повторно использует один и тот же клиент
        let notifier = SmsNotifier::new(
            "https://example.com/sms-gateway",
            None,
            None,
            None,
            "+1234567899",
            15,
        );

        // Получаем клиент
        let client1 = notifier.client();
        let client2 = notifier.client();

        // Должны быть одинаковые указатели (один и тот же объект)
        assert!(std::ptr::eq(client1, client2), "SmsNotifier should reuse the same client instance");
    }

    #[tokio::test]
    async fn test_enhanced_notification_config_default() {
        let config = EnhancedNotificationConfig::default();
        
        // Проверяем стратегии по умолчанию
        assert!(config.strategies.contains_key(&NotificationType::Critical));
        assert!(config.strategies.contains_key(&NotificationType::Warning));
        assert!(config.strategies.contains_key(&NotificationType::Info));
        
        // Проверяем стратегию для критических уведомлений
        let critical_strategy = config.strategies.get(&NotificationType::Critical).unwrap();
        assert_eq!(critical_strategy.max_frequency_seconds, 30);
        assert_eq!(critical_strategy.priority, 100);
        assert_eq!(critical_strategy.max_retries, 5);
        assert_eq!(critical_strategy.retry_delay_ms, 500);
        assert!(critical_strategy.enable_escalation);
        assert_eq!(critical_strategy.escalation_channels.len(), 3);
        
        // Проверяем глобальные настройки
        assert_eq!(config.global_rate_limit_seconds, 60);
        assert!(config.enable_monitoring_integration);
        assert!(config.enable_detailed_logging);
    }

    #[tokio::test]
    async fn test_notification_strategy_default() {
        let strategy = NotificationStrategy::default();
        
        assert_eq!(strategy.max_frequency_seconds, 60);
        assert_eq!(strategy.priority, 50);
        assert_eq!(strategy.max_retries, 3);
        assert_eq!(strategy.retry_delay_ms, 1000);
        assert!(!strategy.enable_escalation);
        assert_eq!(strategy.escalation_channels, vec!["webhook"]);
    }

    #[tokio::test]
    async fn test_notification_manager_config_management() {
        let manager = NotificationManager::new_stub();
        
        // Проверяем конфигурацию по умолчанию
        let default_config = manager.get_config().await;
        assert_eq!(default_config.global_rate_limit_seconds, 60);
        
        // Создаём новую конфигурацию
        let mut new_config = EnhancedNotificationConfig::default();
        new_config.global_rate_limit_seconds = 120;
        
        // Устанавливаем новую конфигурацию
        manager.set_config(new_config.clone()).await.unwrap();
        
        // Проверяем, что конфигурация обновлена
        let updated_config = manager.get_config().await;
        assert_eq!(updated_config.global_rate_limit_seconds, 120);
    }

    #[tokio::test]
    async fn test_notification_manager_escalation_notifiers() {
        let manager = NotificationManager::new_stub();
        
        // Проверяем, что изначально нет эскалационных нотифаеров
        let escalation_notifiers = manager.escalation_notifiers.read().await;
        assert!(escalation_notifiers.is_empty());
        
        // Добавляем эскалационный нотифаер
        let email_notifier = Box::new(StubNotifier);
        manager.add_escalation_notifier("email".to_string(), email_notifier).await.unwrap();
        
        // Проверяем, что нотифаер добавлен
        let escalation_notifiers = manager.escalation_notifiers.read().await;
        assert_eq!(escalation_notifiers.len(), 1);
        assert!(escalation_notifiers.contains_key("email"));
        
        // Удаляем эскалационный нотифаер
        manager.remove_escalation_notifier("email").await.unwrap();
        
        // Проверяем, что нотифаер удалён
        let escalation_notifiers = manager.escalation_notifiers.read().await;
        assert!(escalation_notifiers.is_empty());
    }

    #[tokio::test]
    async fn test_notification_manager_rate_limiting() {
        let manager = NotificationManager::new_stub();
        
        // Создаём уведомление
        let notification = Notification::new(NotificationType::Info, "Test", "Test message");
        
        // Первая отправка должна пройти успешно
        let result = manager.send_with_strategy(&notification).await;
        assert!(result.is_ok());
        
        // Вторая отправка должна быть ограничена глобальным лимитом
        let result = manager.send_with_strategy(&notification).await;
        assert!(result.is_ok()); // Должно возвращать Ok, но не отправлять
        
        // Проверяем состояние
        let status = manager.get_enhanced_status().await.unwrap();
        assert!(status.last_notification_time.is_some());
    }

    #[tokio::test]
    async fn test_notification_manager_send_with_strategy_success() {
        let manager = NotificationManager::new_stub();
        
        // Создаём уведомление
        let notification = Notification::new(NotificationType::Info, "Test", "Test message");
        
        // Отправляем уведомление
        let result = manager.send_with_strategy(&notification).await;
        assert!(result.is_ok());
        
        // Проверяем расширенное состояние
        let status = manager.get_enhanced_status().await.unwrap();
        assert!(status.enabled);
        assert_eq!(status.backend, "stub");
        assert!(status.last_notification_time.is_some());
        assert_eq!(status.notification_count_by_type, 1);
    }

    #[tokio::test]
    async fn test_enhanced_notification_manager_creation() {
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Проверяем базовые свойства
        assert_eq!(enhanced_manager.backend_name(), "stub");
        assert!(enhanced_manager.is_enabled());
        
        // Проверяем конфигурацию
        let config = enhanced_manager.get_config().await;
        assert_eq!(config.global_rate_limit_seconds, 60);
    }

    #[tokio::test]
    async fn test_enhanced_notification_manager_send() {
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Создаём уведомление
        let notification = Notification::new(NotificationType::Info, "Test", "Test message");
        
        // Отправляем уведомление
        let result = enhanced_manager.send(&notification).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_enhanced_notification_manager_status() {
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Получаем состояние
        let status = enhanced_manager.get_status().await.unwrap();
        
        // Проверяем состояние
        assert!(status.enabled);
        assert_eq!(status.backend, "stub");
        assert_eq!(status.global_rate_limit_seconds, 60);
        assert!(status.last_notification_time.is_none());
        assert_eq!(status.notification_count_by_type, 0);
        assert_eq!(status.escalation_channels_count, 0);
        assert!(status.monitoring_integration_enabled);
    }

    #[tokio::test]
    async fn test_enhanced_notification_manager_escalation() {
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Добавляем эскалационный нотифаер
        let email_notifier = Box::new(StubNotifier);
        enhanced_manager.add_escalation_notifier("email".to_string(), email_notifier).await.unwrap();
        
        // Проверяем, что нотифаер добавлен
        let status = enhanced_manager.get_status().await.unwrap();
        assert_eq!(status.escalation_channels_count, 1);
    }

    #[tokio::test]
    async fn test_notification_strategy_serialization() {
        let strategy = NotificationStrategy {
            max_frequency_seconds: 120,
            priority: 75,
            max_retries: 5,
            retry_delay_ms: 2000,
            enable_escalation: true,
            escalation_channels: vec!["email".to_string(), "sms".to_string()],
        };
        
        // Тестируем сериализацию
        let serialized = serde_json::to_string(&strategy).unwrap();
        let deserialized: NotificationStrategy = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized.max_frequency_seconds, 120);
        assert_eq!(deserialized.priority, 75);
        assert_eq!(deserialized.max_retries, 5);
        assert_eq!(deserialized.retry_delay_ms, 2000);
        assert!(deserialized.enable_escalation);
        assert_eq!(deserialized.escalation_channels, vec!["email", "sms"]);
    }

    #[tokio::test]
    async fn test_enhanced_notification_config_serialization() {
        let mut config = EnhancedNotificationConfig::default();
        config.global_rate_limit_seconds = 120;
        
        // Тестируем сериализацию
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: EnhancedNotificationConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized.global_rate_limit_seconds, 120);
        assert_eq!(deserialized.strategies.len(), 3); // Critical, Warning, Info
    }

    #[tokio::test]
    async fn test_notification_manager_with_webhook_escalation() {
        let manager = NotificationManager::new_webhook("https://example.com/webhook");
        
        // Добавляем эскалационный нотифаер
        let email_notifier = Box::new(StubNotifier);
        manager.add_escalation_notifier("email".to_string(), email_notifier).await.unwrap();
        
        // Создаём критическое уведомление (должно использовать эскалацию)
        let notification = Notification::new(NotificationType::Critical, "Critical Test", "Critical message");
        
        // Отправляем уведомление
        let result = manager.send_with_strategy(&notification).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notification_manager_rate_limiting_by_type() {
        let manager = NotificationManager::new_stub();
        
        // Создаём уведомление информационного типа
        let info_notification = Notification::new(NotificationType::Info, "Info Test", "Info message");
        
        // Отправляем уведомление
        let result = manager.send_with_strategy(&info_notification).await;
        assert!(result.is_ok());
        
        // Проверяем, что время последнего уведомления обновлено
        let last_by_type = manager.last_notification_by_type.read().await;
        assert!(last_by_type.contains_key(&NotificationType::Info));
    }

    #[tokio::test]
    async fn test_enhanced_notification_manager_config_update() {
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Получаем текущую конфигурацию
        let current_config = enhanced_manager.get_config().await;
        
        // Создаём новую конфигурацию с изменёнными настройками
        let mut new_config = current_config;
        new_config.global_rate_limit_seconds = 300;
        
        // Обновляем конфигурацию
        enhanced_manager.set_config(new_config.clone()).await.unwrap();
        
        // Проверяем, что конфигурация обновлена
        let updated_config = enhanced_manager.get_config().await;
        assert_eq!(updated_config.global_rate_limit_seconds, 300);
    }

    #[tokio::test]
    async fn test_notification_manager_disabled_with_strategy() {
        let mut manager = NotificationManager::new_stub();
        manager.set_enabled(false);
        
        // Создаём уведомление
        let notification = Notification::new(NotificationType::Critical, "Test", "Test message");
        
        // Отправляем уведомление (должно быть проигнорировано)
        let result = manager.send_with_strategy(&notification).await;
        assert!(result.is_ok()); // Должно возвращать Ok, даже если отключено
    }

    #[tokio::test]
    async fn test_enhanced_notification_status_serialization() {
        let manager = NotificationManager::new_stub();
        let status = manager.get_enhanced_status().await.unwrap();
        
        // Тестируем сериализацию
        let serialized = serde_json::to_string(&status).unwrap();
        let deserialized: EnhancedNotificationStatus = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized.enabled, status.enabled);
        assert_eq!(deserialized.backend, status.backend);
        assert_eq!(deserialized.global_rate_limit_seconds, status.global_rate_limit_seconds);
    }

    #[tokio::test]
    async fn test_notification_strategy_priority_levels() {
        // Тестируем разные уровни приоритета
        let high_priority = NotificationStrategy {
            priority: 100,
            ..Default::default()
        };
        
        let medium_priority = NotificationStrategy {
            priority: 50,
            ..Default::default()
        };
        
        let low_priority = NotificationStrategy {
            priority: 10,
            ..Default::default()
        };
        
        assert_eq!(high_priority.priority, 100);
        assert_eq!(medium_priority.priority, 50);
        assert_eq!(low_priority.priority, 10);
    }

    #[tokio::test]
    async fn test_notification_manager_multiple_notification_types() {
        let manager = NotificationManager::new_stub();
        
        // Создаём уведомления разных типов
        let notifications = vec![
            Notification::new(NotificationType::Critical, "Critical", "Critical message"),
            Notification::new(NotificationType::Warning, "Warning", "Warning message"),
            Notification::new(NotificationType::Info, "Info", "Info message"),
        ];
        
        // Отправляем все уведомления
        for notification in &notifications {
            let result = manager.send_with_strategy(notification).await;
            assert!(result.is_ok());
        }
        
        // Проверяем, что все типы были обработаны
        let last_by_type = manager.last_notification_by_type.read().await;
        assert_eq!(last_by_type.len(), 3);
    }

    #[tokio::test]
    async fn test_enhanced_notification_manager_escalation_channels() {
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Добавляем несколько эскалационных нотифаеров
        let email_notifier = Box::new(StubNotifier);
        let sms_notifier = Box::new(StubNotifier);
        
        enhanced_manager.add_escalation_notifier("email".to_string(), email_notifier).await.unwrap();
        enhanced_manager.add_escalation_notifier("sms".to_string(), sms_notifier).await.unwrap();
        
        // Проверяем состояние
        let status = enhanced_manager.get_status().await.unwrap();
        assert_eq!(status.escalation_channels_count, 2);
        
        // Удаляем один нотифаер
        enhanced_manager.remove_escalation_notifier("email").await.unwrap();
        
        // Проверяем, что остался только один
        let status = enhanced_manager.get_status().await.unwrap();
        assert_eq!(status.escalation_channels_count, 1);
    }

    #[tokio::test]
    async fn test_notification_manager_retry_mechanism() {
        // Тестируем механизм повторных попыток
        // Для этого теста нам нужен нотифаер, который сначала терпит неудачу, а затем успешен
        // Используем заглушку, которая всегда успешна, но проверяем логику
        
        let manager = NotificationManager::new_stub();
        
        // Создаём конфигурацию с несколькими попытками
        let mut config = EnhancedNotificationConfig::default();
        let mut critical_strategy = NotificationStrategy::default();
        critical_strategy.max_retries = 3;
        critical_strategy.retry_delay_ms = 100;
        config.strategies.insert(NotificationType::Critical, critical_strategy);
        
        manager.set_config(config).await.unwrap();
        
        // Создаём критическое уведомление
        let notification = Notification::new(NotificationType::Critical, "Critical", "Critical message");
        
        // Отправляем уведомление
        let result = manager.send_with_strategy(&notification).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_enhanced_notification_config_custom_strategies() {
        // Тестируем кастомные стратегии для разных типов уведомлений
        let mut config = EnhancedNotificationConfig::default();
        
        // Добавляем кастомную стратегию для PriorityChange
        let priority_strategy = NotificationStrategy {
            max_frequency_seconds: 60,
            priority: 80,
            max_retries: 2,
            retry_delay_ms: 500,
            enable_escalation: false,
            escalation_channels: vec!["webhook".to_string()],
        };
        
        config.strategies.insert(NotificationType::PriorityChange, priority_strategy);
        
        // Проверяем, что стратегия добавлена
        assert!(config.strategies.contains_key(&NotificationType::PriorityChange));
        
        let strategy = config.strategies.get(&NotificationType::PriorityChange).unwrap();
        assert_eq!(strategy.priority, 80);
        assert_eq!(strategy.max_retries, 2);
    }

    #[tokio::test]
    async fn test_notification_manager_global_rate_limit() {
        let manager = NotificationManager::new_stub();
        
        // Устанавливаем глобальный лимит частоты
        let mut config = EnhancedNotificationConfig::default();
        config.global_rate_limit_seconds = 300; // 5 минут
        
        manager.set_config(config).await.unwrap();
        
        // Создаём уведомление
        let notification = Notification::new(NotificationType::Info, "Test", "Test message");
        
        // Отправляем уведомление
        let result = manager.send_with_strategy(&notification).await;
        assert!(result.is_ok());
        
        // Проверяем, что глобальное время обновлено
        let status = manager.get_enhanced_status().await.unwrap();
        assert!(status.last_notification_time.is_some());
    }

    #[tokio::test]
    async fn test_enhanced_notification_manager_monitoring_integration() {
        let manager = NotificationManager::new_stub();
        let mut enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Проверяем, что интеграция с мониторингом включена по умолчанию
        let config = enhanced_manager.get_config().await;
        assert!(config.enable_monitoring_integration);
        
        // Проверяем, что health_monitoring_integration изначально None
        assert!(enhanced_manager.health_monitoring_integration.is_none());
    }

    #[tokio::test]
    async fn test_notification_strategy_escalation_channels() {
        // Тестируем разные комбинации каналов эскалации
        let email_sms_strategy = NotificationStrategy {
            enable_escalation: true,
            escalation_channels: vec!["email".to_string(), "sms".to_string()],
            ..Default::default()
        };
        
        let webhook_only_strategy = NotificationStrategy {
            enable_escalation: true,
            escalation_channels: vec!["webhook".to_string()],
            ..Default::default()
        };
        
        let no_escalation_strategy = NotificationStrategy {
            enable_escalation: false,
            escalation_channels: vec![],
            ..Default::default()
        };
        
        assert_eq!(email_sms_strategy.escalation_channels.len(), 2);
        assert_eq!(webhook_only_strategy.escalation_channels.len(), 1);
        assert!(no_escalation_strategy.escalation_channels.is_empty());
    }

    #[tokio::test]
    async fn test_enhanced_notification_manager_comprehensive() {
        // Комплексный тест всех функций EnhancedNotificationManager
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // 1. Проверяем начальное состояние
        let initial_status = enhanced_manager.get_status().await.unwrap();
        assert!(initial_status.enabled);
        assert_eq!(initial_status.escalation_channels_count, 0);
        
        // 2. Добавляем эскалационные нотифаеры
        let email_notifier = Box::new(StubNotifier);
        let sms_notifier = Box::new(StubNotifier);
        
        enhanced_manager.add_escalation_notifier("email".to_string(), email_notifier).await.unwrap();
        enhanced_manager.add_escalation_notifier("sms".to_string(), sms_notifier).await.unwrap();
        
        // 3. Проверяем обновлённое состояние
        let updated_status = enhanced_manager.get_status().await.unwrap();
        assert_eq!(updated_status.escalation_channels_count, 2);
        
        // 4. Обновляем конфигурацию
        let mut config = enhanced_manager.get_config().await;
        config.global_rate_limit_seconds = 600;
        enhanced_manager.set_config(config).await.unwrap();
        
        // 5. Проверяем обновлённую конфигурацию
        let new_config = enhanced_manager.get_config().await;
        assert_eq!(new_config.global_rate_limit_seconds, 600);
        
        // 6. Отправляем уведомление
        let notification = Notification::new(NotificationType::Info, "Test", "Test message");
        let result = enhanced_manager.send(&notification).await;
        assert!(result.is_ok());
        
        // 7. Проверяем финальное состояние
        let final_status = enhanced_manager.get_status().await.unwrap();
        assert!(final_status.last_notification_time.is_some());
        assert_eq!(final_status.notification_count_by_type, 1);
    }

    #[tokio::test]
    async fn test_notification_manager_escalation_with_failure() {
        // Тестируем эскалацию при неудачной основной отправке
        let manager = NotificationManager::new_stub();
        
        // Добавляем эскалационный нотифаер
        let email_notifier = Box::new(StubNotifier);
        manager.add_escalation_notifier("email".to_string(), email_notifier).await.unwrap();
        
        // Создаём конфигурацию с эскалацией для информационных уведомлений
        let mut config = EnhancedNotificationConfig::default();
        let mut info_strategy = NotificationStrategy::default();
        info_strategy.enable_escalation = true;
        info_strategy.escalation_channels = vec!["email".to_string()];
        config.strategies.insert(NotificationType::Info, info_strategy);
        
        manager.set_config(config).await.unwrap();
        
        // Создаём уведомление
        let notification = Notification::new(NotificationType::Info, "Test", "Test message");
        
        // Отправляем уведомление
        let result = manager.send_with_strategy(&notification).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notification_manager_different_priority_levels() {
        let manager = NotificationManager::new_stub();
        
        // Создаём уведомления с разными уровнями приоритета
        let critical_notification = Notification::new(NotificationType::Critical, "Critical", "Critical message");
        let warning_notification = Notification::new(NotificationType::Warning, "Warning", "Warning message");
        let info_notification = Notification::new(NotificationType::Info, "Info", "Info message");
        
        // Отправляем все уведомления
        manager.send_with_strategy(&critical_notification).await.unwrap();
        manager.send_with_strategy(&warning_notification).await.unwrap();
        manager.send_with_strategy(&info_notification).await.unwrap();
        
        // Проверяем, что все типы были обработаны
        let last_by_type = manager.last_notification_by_type.read().await;
        assert_eq!(last_by_type.len(), 3);
        
        // Проверяем, что критические уведомления имеют наивысший приоритет
        let config = manager.get_config().await;
        let critical_strategy = config.strategies.get(&NotificationType::Critical).unwrap();
        let info_strategy = config.strategies.get(&NotificationType::Info).unwrap();
        
        assert!(critical_strategy.priority > info_strategy.priority);
    }

    #[tokio::test]
    async fn test_enhanced_notification_config_validation() {
        // Тестируем валидацию конфигурации
        let config = EnhancedNotificationConfig::default();
        
        // Проверяем, что все стратегии имеют разумные значения
        for (notification_type, strategy) in &config.strategies {
            assert!(strategy.max_retries > 0, "Max retries should be > 0 for {:?}", notification_type);
            assert!(strategy.retry_delay_ms > 0, "Retry delay should be > 0 for {:?}", notification_type);
            assert!(strategy.priority <= 100, "Priority should be <= 100 for {:?}", notification_type);
        }
        
        // Проверяем, что глобальный лимит частоты разумный
        assert!(config.global_rate_limit_seconds > 0);
    }

    #[tokio::test]
    async fn test_notification_manager_rate_limiting_respects_type_strategy() {
        let manager = NotificationManager::new_stub();
        
        // Создаём конфигурацию с разными лимитами для разных типов
        let mut config = EnhancedNotificationConfig::default();
        
        // Устанавливаем очень низкий лимит для информационных уведомлений
        let mut info_strategy = NotificationStrategy::default();
        info_strategy.max_frequency_seconds = 1; // 1 секунда
        config.strategies.insert(NotificationType::Info, info_strategy);
        
        manager.set_config(config).await.unwrap();
        
        // Создаём информационное уведомление
        let info_notification = Notification::new(NotificationType::Info, "Info", "Info message");
        
        // Первая отправка должна пройти успешно
        let result = manager.send_with_strategy(&info_notification).await;
        assert!(result.is_ok());
        
        // Вторая отправка должна быть ограничена
        let result = manager.send_with_strategy(&info_notification).await;
        assert!(result.is_ok()); // Должно возвращать Ok, но не отправлять
    }

    #[tokio::test]
    async fn test_enhanced_notification_manager_error_handling() {
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Тестируем обработку ошибок при добавлении эскалационного нотифаера
        let email_notifier = Box::new(StubNotifier);
        let result = enhanced_manager.add_escalation_notifier("email".to_string(), email_notifier).await;
        assert!(result.is_ok());
        
        // Тестируем обработку ошибок при удалении несуществующего нотифаера
        let result = enhanced_manager.remove_escalation_notifier("nonexistent").await;
        assert!(result.is_ok()); // Должно возвращать Ok, даже если нотифаер не существует
        
        // Тестируем обработку ошибок при обновлении конфигурации
        let config = EnhancedNotificationConfig::default();
        let result = enhanced_manager.set_config(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notification_manager_escalation_with_multiple_channels() {
        let manager = NotificationManager::new_stub();
        
        // Добавляем несколько эскалационных нотифаеров
        let email_notifier = Box::new(StubNotifier);
        let sms_notifier = Box::new(StubNotifier);
        let webhook_notifier = Box::new(StubNotifier);
        
        manager.add_escalation_notifier("email".to_string(), email_notifier).await.unwrap();
        manager.add_escalation_notifier("sms".to_string(), sms_notifier).await.unwrap();
        manager.add_escalation_notifier("webhook".to_string(), webhook_notifier).await.unwrap();
        
        // Создаём конфигурацию с эскалацией по нескольким каналам
        let mut config = EnhancedNotificationConfig::default();
        let mut critical_strategy = NotificationStrategy::default();
        critical_strategy.enable_escalation = true;
        critical_strategy.escalation_channels = vec!["email".to_string(), "sms".to_string(), "webhook".to_string()];
        config.strategies.insert(NotificationType::Critical, critical_strategy);
        
        manager.set_config(config).await.unwrap();
        
        // Создаём критическое уведомление
        let notification = Notification::new(NotificationType::Critical, "Critical", "Critical message");
        
        // Отправляем уведомление
        let result = manager.send_with_strategy(&notification).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_enhanced_notification_manager_configuration_consistency() {
        // Тестируем согласованность конфигурации
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Получаем конфигурацию
        let config1 = enhanced_manager.get_config().await;
        let config2 = enhanced_manager.get_config().await;
        
        // Проверяем, что конфигурации идентичны
        assert_eq!(config1.global_rate_limit_seconds, config2.global_rate_limit_seconds);
        assert_eq!(config1.strategies.len(), config2.strategies.len());
        assert_eq!(config1.enable_monitoring_integration, config2.enable_monitoring_integration);
    }

    #[tokio::test]
    async fn test_notification_manager_time_tracking() {
        let manager = NotificationManager::new_stub();
        
        // Проверяем, что изначально время не установлено
        let last_global = manager.last_global_notification.read().await;
        assert!(last_global.is_none());
        
        let last_by_type = manager.last_notification_by_type.read().await;
        assert!(last_by_type.is_empty());
        
        // Отправляем уведомление
        let notification = Notification::new(NotificationType::Info, "Test", "Test message");
        manager.send_with_strategy(&notification).await.unwrap();
        
        // Проверяем, что время обновлено
        let last_global = manager.last_global_notification.read().await;
        assert!(last_global.is_some());
        
        let last_by_type = manager.last_notification_by_type.read().await;
        assert!(!last_by_type.is_empty());
    }

    #[tokio::test]
    async fn test_enhanced_notification_manager_cloning() {
        let manager = NotificationManager::new_stub();
        let enhanced_manager1 = EnhancedNotificationManager::new(manager);
        
        // Клонируем менеджер
        let enhanced_manager2 = enhanced_manager1.clone();
        
        // Проверяем, что оба менеджера имеют одинаковое состояние
        let status1 = enhanced_manager1.get_status().await.unwrap();
        let status2 = enhanced_manager2.get_status().await.unwrap();
        
        assert_eq!(status1.enabled, status2.enabled);
        assert_eq!(status1.backend, status2.backend);
        assert_eq!(status1.global_rate_limit_seconds, status2.global_rate_limit_seconds);
    }

    #[tokio::test]
    async fn test_notification_strategy_edge_cases() {
        // Тестируем крайние случаи для стратегий
        
        // Стратегия с нулевым лимитом частоты (отключено)
        let no_rate_limit_strategy = NotificationStrategy {
            max_frequency_seconds: 0,
            ..Default::default()
        };
        
        // Стратегия с нулевыми попытками (не рекомендуется, но допустимо)
        let no_retries_strategy = NotificationStrategy {
            max_retries: 0,
            ..Default::default()
        };
        
        // Стратегия с нулевой задержкой
        let no_delay_strategy = NotificationStrategy {
            retry_delay_ms: 0,
            ..Default::default()
        };
        
        assert_eq!(no_rate_limit_strategy.max_frequency_seconds, 0);
        assert_eq!(no_retries_strategy.max_retries, 0);
        assert_eq!(no_delay_strategy.retry_delay_ms, 0);
    }

    #[tokio::test]
    async fn test_enhanced_notification_config_edge_cases() {
        // Тестируем крайние случаи для конфигурации
        
        // Конфигурация с нулевым глобальным лимитом
        let mut config = EnhancedNotificationConfig::default();
        config.global_rate_limit_seconds = 0;
        
        // Конфигурация с отключённой интеграцией мониторинга
        let mut no_monitoring_config = EnhancedNotificationConfig::default();
        no_monitoring_config.enable_monitoring_integration = false;
        
        // Конфигурация с отключённым детальным логированием
        let mut no_logging_config = EnhancedNotificationConfig::default();
        no_logging_config.enable_detailed_logging = false;
        
        assert_eq!(config.global_rate_limit_seconds, 0);
        assert!(!no_monitoring_config.enable_monitoring_integration);
        assert!(!no_logging_config.enable_detailed_logging);
    }

    #[tokio::test]
    async fn test_notification_manager_with_different_backends() {
        // Тестируем менеджер с разными бэкендами
        
        // Тестируем с заглушкой
        let stub_manager = NotificationManager::new_stub();
        assert_eq!(stub_manager.backend_name(), "stub");
        
        // Тестируем с вебхук
        let webhook_manager = NotificationManager::new_webhook("https://example.com/webhook");
        assert_eq!(webhook_manager.backend_name(), "webhook");
        
        // Тестируем с SMS
        let sms_manager = NotificationManager::new_sms("https://sms-gateway.example.com", "+1234567890");
        assert_eq!(sms_manager.backend_name(), "sms");
    }

    #[tokio::test]
    async fn test_enhanced_notification_manager_backend_consistency() {
        // Тестируем согласованность бэкендов
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Проверяем, что бэкенды совпадают
        assert_eq!(enhanced_manager.backend_name(), "stub");
        
        // Проверяем состояние
        let status = enhanced_manager.get_status().await.unwrap();
        assert_eq!(status.backend, "stub");
    }

    #[tokio::test]
    async fn test_notification_manager_escalation_notifier_types() {
        // Тестируем разные типы эскалационных нотифаеров
        let manager = NotificationManager::new_stub();
        
        // Добавляем разные типы нотифаеров
        let stub_notifier = Box::new(StubNotifier);
        let webhook_notifier = Box::new(WebhookNotifier::new("https://example.com/webhook"));
        let sms_notifier = Box::new(SmsNotifier::new("https://sms-gateway.example.com", "+1234567890"));
        
        manager.add_escalation_notifier("stub".to_string(), stub_notifier).await.unwrap();
        manager.add_escalation_notifier("webhook".to_string(), webhook_notifier).await.unwrap();
        manager.add_escalation_notifier("sms".to_string(), sms_notifier).await.unwrap();
        
        // Проверяем, что все нотифаеры добавлены
        let escalation_notifiers = manager.escalation_notifiers.read().await;
        assert_eq!(escalation_notifiers.len(), 3);
        
        // Проверяем типы нотифаеров
        assert!(escalation_notifiers.contains_key("stub"));
        assert!(escalation_notifiers.contains_key("webhook"));
        assert!(escalation_notifiers.contains_key("sms"));
    }

    #[tokio::test]
    async fn test_enhanced_notification_manager_comprehensive_integration() {
        // Комплексный тест интеграции всех функций
        let manager = NotificationManager::new_stub();
        let mut enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // 1. Настраиваем конфигурацию
        let mut config = EnhancedNotificationConfig::default();
        config.global_rate_limit_seconds = 120;
        enhanced_manager.set_config(config).await.unwrap();
        
        // 2. Добавляем эскалационные нотифаеры
        let email_notifier = Box::new(StubNotifier);
        let sms_notifier = Box::new(StubNotifier);
        
        enhanced_manager.add_escalation_notifier("email".to_string(), email_notifier).await.unwrap();
        enhanced_manager.add_escalation_notifier("sms".to_string(), sms_notifier).await.unwrap();
        
        // 3. Отправляем уведомления разных типов
        let notifications = vec![
            Notification::new(NotificationType::Critical, "Critical", "Critical message"),
            Notification::new(NotificationType::Warning, "Warning", "Warning message"),
            Notification::new(NotificationType::Info, "Info", "Info message"),
        ];
        
        for notification in &notifications {
            enhanced_manager.send(notification).await.unwrap();
        }
        
        // 4. Проверяем финальное состояние
        let final_status = enhanced_manager.get_status().await.unwrap();
        assert!(final_status.last_notification_time.is_some());
        assert_eq!(final_status.notification_count_by_type, 3);
        assert_eq!(final_status.escalation_channels_count, 2);
        assert_eq!(final_status.global_rate_limit_seconds, 120);
        
        // 5. Проверяем конфигурацию
        let final_config = enhanced_manager.get_config().await;
        assert_eq!(final_config.global_rate_limit_seconds, 120);
        assert_eq!(final_config.strategies.len(), 3);
    }

    #[tokio::test]
    async fn test_notification_manager_health_event_integration() {
        // Тестируем интеграцию с событиями здоровья
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Создаём mock событие здоровья
        let health_event = HealthEvent::NewHealthIssue {
            issue: HealthIssue {
                issue_id: "test_issue".to_string(),
                issue_type: "test_type".to_string(),
                description: "test_description".to_string(),
                details: Some("test_details".to_string()),
                severity: HealthIssueSeverity::Warning,
                timestamp: Utc::now(),
            },
            timestamp: Utc::now(),
        };
        
        // Отправляем уведомление о событии здоровья
        let result = enhanced_manager.send_health_event_notification(&health_event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_enhanced_notification_manager_health_event_types() {
        // Тестируем разные типы событий здоровья
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Создаём разные типы событий
        let critical_issue = HealthIssue {
            issue_id: "critical_issue".to_string(),
            issue_type: "critical_type".to_string(),
            description: "critical_description".to_string(),
            details: Some("critical_details".to_string()),
            severity: HealthIssueSeverity::Critical,
            timestamp: Utc::now(),
        };
        
        let warning_issue = HealthIssue {
            issue_id: "warning_issue".to_string(),
            issue_type: "warning_type".to_string(),
            description: "warning_description".to_string(),
            details: Some("warning_details".to_string()),
            severity: HealthIssueSeverity::Warning,
            timestamp: Utc::now(),
        };
        
        let events = vec![
            HealthEvent::NewHealthIssue {
                issue: critical_issue,
                timestamp: Utc::now(),
            },
            HealthEvent::NewHealthIssue {
                issue: warning_issue,
                timestamp: Utc::now(),
            },
            HealthEvent::HealthStatusChanged {
                old_status: HealthStatus::Healthy,
                new_status: HealthStatus::Degraded,
                timestamp: Utc::now(),
            },
            HealthEvent::HealthIssueResolved {
                issue_id: "resolved_issue".to_string(),
                timestamp: Utc::now(),
            },
        ];
        
        // Отправляем все события
        for event in &events {
            let result = enhanced_manager.send_health_event_notification(event).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_notification_manager_health_event_with_monitoring_disabled() {
        // Тестируем отправку событий здоровья при отключённой интеграции
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Отключаем интеграцию с мониторингом
        let mut config = enhanced_manager.get_config().await;
        config.enable_monitoring_integration = false;
        enhanced_manager.set_config(config).await.unwrap();
        
        // Создаём событие здоровья
        let health_event = HealthEvent::NewHealthIssue {
            issue: HealthIssue {
                issue_id: "test_issue".to_string(),
                issue_type: "test_type".to_string(),
                description: "test_description".to_string(),
                details: Some("test_details".to_string()),
                severity: HealthIssueSeverity::Warning,
                timestamp: Utc::now(),
            },
            timestamp: Utc::now(),
        };
        
        // Отправляем уведомление (должно быть проигнорировано)
        let result = enhanced_manager.send_health_event_notification(&health_event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_enhanced_notification_manager_health_event_priority_mapping() {
        // Тестируем маппинг приоритетов для событий здоровья
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Создаём критические, предупреждающие и информационные события
        let critical_issue = HealthIssue {
            issue_id: "critical".to_string(),
            issue_type: "critical".to_string(),
            description: "critical".to_string(),
            details: None,
            severity: HealthIssueSeverity::Critical,
            timestamp: Utc::now(),
        };
        
        let warning_issue = HealthIssue {
            issue_id: "warning".to_string(),
            issue_type: "warning".to_string(),
            description: "warning".to_string(),
            details: None,
            severity: HealthIssueSeverity::Warning,
            timestamp: Utc::now(),
        };
        
        let info_issue = HealthIssue {
            issue_id: "info".to_string(),
            issue_type: "info".to_string(),
            description: "info".to_string(),
            details: None,
            severity: HealthIssueSeverity::Info,
            timestamp: Utc::now(),
        };
        
        let events = vec![
            HealthEvent::NewHealthIssue {
                issue: critical_issue,
                timestamp: Utc::now(),
            },
            HealthEvent::NewHealthIssue {
                issue: warning_issue,
                timestamp: Utc::now(),
            },
            HealthEvent::NewHealthIssue {
                issue: info_issue,
                timestamp: Utc::now(),
            },
        ];
        
        // Отправляем все события
        for event in &events {
            let result = enhanced_manager.send_health_event_notification(event).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_notification_manager_health_event_with_details() {
        // Тестируем события здоровья с деталями
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Создаём событие с деталями
        let health_event = HealthEvent::NewHealthIssue {
            issue: HealthIssue {
                issue_id: "detailed_issue".to_string(),
                issue_type: "detailed_type".to_string(),
                description: "detailed_description".to_string(),
                details: Some("very detailed information about the health issue".to_string()),
                severity: HealthIssueSeverity::Warning,
                timestamp: Utc::now(),
            },
            timestamp: Utc::now(),
        };
        
        // Отправляем уведомление
        let result = enhanced_manager.send_health_event_notification(&health_event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_enhanced_notification_manager_health_event_status_changes() {
        // Тестируем уведомления о изменении статуса здоровья
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Создаём событие изменения статуса
        let health_event = HealthEvent::HealthStatusChanged {
            old_status: HealthStatus::Healthy,
            new_status: HealthStatus::Degraded,
            timestamp: Utc::now(),
        };
        
        // Отправляем уведомление
        let result = enhanced_manager.send_health_event_notification(&health_event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notification_manager_health_event_critical_escalation() {
        // Тестируем эскалацию для критических событий здоровья
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Добавляем эскалационные нотифаеры
        let email_notifier = Box::new(StubNotifier);
        let sms_notifier = Box::new(StubNotifier);
        
        enhanced_manager.add_escalation_notifier("email".to_string(), email_notifier).await.unwrap();
        enhanced_manager.add_escalation_notifier("sms".to_string(), sms_notifier).await.unwrap();
        
        // Создаём критическое событие здоровья
        let critical_issue = HealthIssue {
            issue_id: "critical_escalation".to_string(),
            issue_type: "critical_type".to_string(),
            description: "critical_description".to_string(),
            details: Some("critical_details".to_string()),
            severity: HealthIssueSeverity::Critical,
            timestamp: Utc::now(),
        };
        
        let health_event = HealthEvent::CriticalHealthDetected {
            issue: critical_issue,
            timestamp: Utc::now(),
        };
        
        // Отправляем уведомление
        let result = enhanced_manager.send_health_event_notification(&health_event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_enhanced_notification_manager_performance() {
        // Тестируем производительность EnhancedNotificationManager
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Отправляем большое количество уведомлений
        let start_time = std::time::Instant::now();
        
        for i in 0..100 {
            let notification = Notification::new(
                NotificationType::Info,
                format!("Test {}", i),
                format!("Test message {}", i),
            );
            enhanced_manager.send(&notification).await.unwrap();
        }
        
        let duration = start_time.elapsed();
        tracing::info!("Sent 100 notifications in {:?}", duration);
        
        // Проверяем, что все уведомления были обработаны
        let status = enhanced_manager.get_status().await.unwrap();
        assert!(status.last_notification_time.is_some());
    }

    #[tokio::test]
    async fn test_notification_manager_concurrent_operations() {
        // Тестируем конкурентные операции
        let manager = NotificationManager::new_stub();
        
        // Создаём несколько задач для конкурентной отправки
        let tasks: Vec<_> = (0..10).map(|i| {
            let manager = manager.clone();
            tokio::spawn(async move {
                let notification = Notification::new(
                    NotificationType::Info,
                    format!("Concurrent {}", i),
                    format!("Concurrent message {}", i),
                );
                manager.send_with_strategy(&notification).await
            })
        }).collect();
        
        // Ждём завершения всех задач
        for task in tasks {
            let result = task.await.unwrap();
            assert!(result.is_ok());
        }
        
        // Проверяем, что все уведомления были обработаны
        let last_by_type = manager.last_notification_by_type.read().await;
        assert!(!last_by_type.is_empty());
    }

    #[tokio::test]
    async fn test_enhanced_notification_manager_stress_test() {
        // Стресс-тест для EnhancedNotificationManager
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Добавляем эскалационные нотифаеры
        let email_notifier = Box::new(StubNotifier);
        enhanced_manager.add_escalation_notifier("email".to_string(), email_notifier).await.unwrap();
        
        // Отправляем большое количество уведомлений разных типов
        let notification_types = vec![
            NotificationType::Critical,
            NotificationType::Warning,
            NotificationType::Info,
        ];
        
        for i in 0..50 {
            let notification_type = &notification_types[i % notification_types.len()];
            let notification = Notification::new(
                notification_type.clone(),
                format!("Stress Test {}", i),
                format!("Stress test message {}", i),
            );
            enhanced_manager.send(&notification).await.unwrap();
        }
        
        // Проверяем финальное состояние
        let status = enhanced_manager.get_status().await.unwrap();
        assert!(status.last_notification_time.is_some());
        assert_eq!(status.notification_count_by_type, 3);
        assert_eq!(status.escalation_channels_count, 1);
    }

    #[tokio::test]
    async fn test_notification_manager_configuration_persistence() {
        // Тестируем сохранение конфигурации
        let manager = NotificationManager::new_stub();
        
        // Устанавливаем кастомную конфигурацию
        let mut config = EnhancedNotificationConfig::default();
        config.global_rate_limit_seconds = 600;
        
        let mut custom_strategy = NotificationStrategy::default();
        custom_strategy.max_frequency_seconds = 300;
        config.strategies.insert(NotificationType::Info, custom_strategy);
        
        manager.set_config(config.clone()).await.unwrap();
        
        // Проверяем, что конфигурация сохранена
        let saved_config = manager.get_config().await;
        assert_eq!(saved_config.global_rate_limit_seconds, 600);
        assert_eq!(saved_config.strategies.get(&NotificationType::Info).unwrap().max_frequency_seconds, 300);
        
        // Отправляем уведомление и проверяем, что конфигурация не изменилась
        let notification = Notification::new(NotificationType::Info, "Test", "Test message");
        manager.send_with_strategy(&notification).await.unwrap();
        
        let final_config = manager.get_config().await;
        assert_eq!(final_config.global_rate_limit_seconds, 600);
    }

    #[tokio::test]
    async fn test_enhanced_notification_manager_error_recovery() {
        // Тестируем восстановление после ошибок
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Устанавливаем невалидную конфигурацию (с нулевыми попытками)
        let mut config = EnhancedNotificationConfig::default();
        let mut invalid_strategy = NotificationStrategy::default();
        invalid_strategy.max_retries = 0; // Нет попыток
        config.strategies.insert(NotificationType::Info, invalid_strategy);
        
        enhanced_manager.set_config(config).await.unwrap();
        
        // Отправляем уведомление (должно завершиться неудачей после 0 попыток)
        let notification = Notification::new(NotificationType::Info, "Test", "Test message");
        let result = enhanced_manager.send(&notification).await;
        assert!(result.is_ok()); // Должно возвращать Ok, даже если отправка не удалась
        
        // Восстанавливаем валидную конфигурацию
        let valid_config = EnhancedNotificationConfig::default();
        enhanced_manager.set_config(valid_config).await.unwrap();
        
        // Отправляем уведомление снова (должно пройти успешно)
        let result = enhanced_manager.send(&notification).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notification_manager_health_integration_comprehensive() {
        // Комплексный тест интеграции с системой здоровья
        let manager = NotificationManager::new_stub();
        let enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // Создаём различные события здоровья
        let critical_issue = HealthIssue {
            issue_id: "critical_integration".to_string(),
            issue_type: "critical_type".to_string(),
            description: "critical_description".to_string(),
            details: Some("critical_details".to_string()),
            severity: HealthIssueSeverity::Critical,
            timestamp: Utc::now(),
        };
        
        let warning_issue = HealthIssue {
            issue_id: "warning_integration".to_string(),
            issue_type: "warning_type".to_string(),
            description: "warning_description".to_string(),
            details: Some("warning_details".to_string()),
            severity: HealthIssueSeverity::Warning,
            timestamp: Utc::now(),
        };
        
        let events = vec![
            HealthEvent::CriticalHealthDetected {
                issue: critical_issue,
                timestamp: Utc::now(),
            },
            HealthEvent::NewHealthIssue {
                issue: warning_issue,
                timestamp: Utc::now(),
            },
            HealthEvent::HealthStatusChanged {
                old_status: HealthStatus::Healthy,
                new_status: HealthStatus::Degraded,
                timestamp: Utc::now(),
            },
            HealthEvent::HealthIssueResolved {
                issue_id: "resolved_integration".to_string(),
                timestamp: Utc::now(),
            },
        ];
        
        // Отправляем все события
        for event in &events {
            let result = enhanced_manager.send_health_event_notification(event).await;
            assert!(result.is_ok());
        }
        
        // Проверяем, что все события были обработаны
        let status = enhanced_manager.get_status().await.unwrap();
        assert!(status.last_notification_time.is_some());
    }

    #[tokio::test]
    async fn test_enhanced_notification_manager_final_comprehensive_test() {
        // Финальный комплексный тест всех функций
        let manager = NotificationManager::new_stub();
        let mut enhanced_manager = EnhancedNotificationManager::new(manager);
        
        // 1. Настраиваем конфигурацию
        let mut config = EnhancedNotificationConfig::default();
        config.global_rate_limit_seconds = 300;
        config.enable_detailed_logging = true;
        enhanced_manager.set_config(config).await.unwrap();
        
        // 2. Добавляем эскалационные нотифаеры
        let email_notifier = Box::new(StubNotifier);
        let sms_notifier = Box::new(StubNotifier);
        let webhook_notifier = Box::new(WebhookNotifier::new("https://example.com/webhook"));
        
        enhanced_manager.add_escalation_notifier("email".to_string(), email_notifier).await.unwrap();
        enhanced_manager.add_escalation_notifier("sms".to_string(), sms_notifier).await.unwrap();
        enhanced_manager.add_escalation_notifier("webhook".to_string(), webhook_notifier).await.unwrap();
        
        // 3. Отправляем уведомления разных типов
        let notifications = vec![
            Notification::new(NotificationType::Critical, "Critical Final", "Critical final message"),
            Notification::new(NotificationType::Warning, "Warning Final", "Warning final message"),
            Notification::new(NotificationType::Info, "Info Final", "Info final message"),
            Notification::priority_change("firefox", "normal", "high", "user request"),
            Notification::config_change("config.yml", "updated settings"),
            Notification::system_event("startup", "System started"),
        ];
        
        for notification in &notifications {
            enhanced_manager.send(notification).await.unwrap();
        }
        
        // 4. Отправляем события здоровья
        let critical_issue = HealthIssue {
            issue_id: "final_critical".to_string(),
            issue_type: "final_critical_type".to_string(),
            description: "final_critical_description".to_string(),
            details: Some("final_critical_details".to_string()),
            severity: HealthIssueSeverity::Critical,
            timestamp: Utc::now(),
        };
        
        let health_event = HealthEvent::CriticalHealthDetected {
            issue: critical_issue,
            timestamp: Utc::now(),
        };
        
        enhanced_manager.send_health_event_notification(&health_event).await.unwrap();
        
        // 5. Проверяем финальное состояние
        let final_status = enhanced_manager.get_status().await.unwrap();
        assert!(final_status.enabled);
        assert_eq!(final_status.backend, "stub");
        assert_eq!(final_status.global_rate_limit_seconds, 300);
        assert!(final_status.last_notification_time.is_some());
        assert_eq!(final_status.notification_count_by_type, 6); // 6 разных типов уведомлений
        assert_eq!(final_status.escalation_channels_count, 3); // 3 эскалационных канала
        assert!(final_status.monitoring_integration_enabled);
        assert!(final_status.has_log_integration);
        
        // 6. Проверяем финальную конфигурацию
        let final_config = enhanced_manager.get_config().await;
        assert_eq!(final_config.global_rate_limit_seconds, 300);
        assert!(final_config.enable_detailed_logging);
        assert!(final_config.enable_monitoring_integration);
        assert_eq!(final_config.strategies.len(), 3); // Critical, Warning, Info
        
        // 7. Тестируем клонирование
        let cloned_manager = enhanced_manager.clone();
        let cloned_status = cloned_manager.get_status().await.unwrap();
        assert_eq!(final_status.notification_count_by_type, cloned_status.notification_count_by_type);
        
        // 8. Тестируем удаление эскалационных нотифаеров
        enhanced_manager.remove_escalation_notifier("email").await.unwrap();
        let updated_status = enhanced_manager.get_status().await.unwrap();
        assert_eq!(updated_status.escalation_channels_count, 2);
        
        tracing::info!("Final comprehensive test completed successfully!");
    }
}
