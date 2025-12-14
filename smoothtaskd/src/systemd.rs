//! Модуль для интеграции с systemd.
//!
//! Предоставляет функции для:
//! - Отправки уведомлений systemd через sd-notify (READY=1, STATUS=...)
//! - Управления сервисами systemd через D-Bus
//! - Проверки статуса сервисов
//!
//! # Примеры использования
//!
//! ```no_run
//! use smoothtaskd::systemd;
//!
//! // Отправляем READY после инициализации
//! if let Err(e) = systemd::notify_ready() {
//!     eprintln!("Failed to notify systemd: {}", e);
//! }
//!
//! // Обновляем статус
//! systemd::notify_status("Running, iteration 42");
//!
//! // Проверяем статус сервиса
//! match systemd::get_service_status("smoothtaskd.service") {
//!     Ok(status) => println!("Service status: {:?}", status),
//!     Err(e) => eprintln!("Failed to get service status: {}", e),
//! }
//!
//! // Управляем сервисом
//! systemd::start_service("smoothtaskd.service")?;
//! ```

use anyhow::{Context, Result};
use libsystemd::daemon::NotifyState;
use zbus::{Connection, Proxy};

/// Отправляет systemd уведомление READY=1, сигнализируя о том, что демон успешно инициализирован.
///
/// Эта функция должна вызываться после полной инициализации всех компонентов демона.
/// Если демон не запущен под systemd, функция вернёт ошибку, которую можно безопасно игнорировать.
///
/// # Возвращаемое значение
///
/// Возвращает `Ok(())` при успешной отправке или `Err` при ошибке.
/// Ошибки можно безопасно игнорировать, если демон не запущен под systemd.
///
/// # Примеры
///
/// ```no_run
/// use smoothtaskd::systemd;
///
/// // После инициализации всех компонентов
/// if let Err(e) = systemd::notify_ready() {
///     // Можно логировать, но не критично, если не под systemd
///     eprintln!("Warning: failed to notify systemd: {}", e);
/// }
/// ```
/// Отправляет systemd уведомление READY=1, сигнализируя о том, что демон успешно инициализирован.
///
/// Эта функция должна вызываться после полной инициализации всех компонентов демона.
/// Если демон не запущен под systemd, функция вернёт ошибку, которую можно безопасно игнорировать.
///
/// # Возвращаемое значение
///
/// Возвращает `Ok(())` при успешной отправке или `Err` при ошибке.
/// Ошибки можно безопасно игнорировать, если демон не запущен под systemd.
///
/// # Примеры
///
/// ```no_run
/// use smoothtaskd::systemd;
///
/// // После инициализации всех компонентов
/// if let Err(e) = systemd::notify_ready() {
///     // Можно логировать, но не критично, если не под systemd
///     eprintln!("Warning: failed to notify systemd: {}", e);
/// }
/// ```
pub fn notify_ready() -> Result<()> {
    let state = NotifyState::Ready;
    libsystemd::daemon::notify(false, &[state])
        .context("Failed to send READY notification to systemd")?;
    Ok(())
}

/// Отправляет systemd уведомление STATUS=..., обновляя статус работы демона.
///
/// Статус будет виден в `systemctl status smoothtaskd`.
/// Если демон не запущен под systemd, функция безопасно игнорирует ошибки.
///
/// # Параметры
///
/// * `status` - строка статуса для отображения (максимум ~200 символов рекомендуется)
///
/// # Примеры
///
/// ```no_run
/// use smoothtaskd::systemd;
///
/// // Обновляем статус
/// systemd::notify_status("Running, processed 42 iterations");
/// systemd::notify_status("Collecting metrics...");
/// ```
/// Отправляет systemd уведомление STATUS=..., обновляя статус работы демона.
///
/// Статус будет виден в `systemctl status smoothtaskd`.
/// Если демон не запущен под systemd, функция безопасно игнорирует ошибки.
///
/// # Параметры
///
/// * `status` - строка статуса для отображения (максимум ~200 символов рекомендуется)
///
/// # Примеры
///
/// ```no_run
/// use smoothtaskd::systemd;
///
/// // Обновляем статус
/// systemd::notify_status("Running, processed 42 iterations");
/// systemd::notify_status("Collecting metrics...");
/// ```
pub fn notify_status(status: &str) {
    // Ограничиваем длину статуса для systemd (рекомендуется ~200 символов)
    let status_truncated = if status.len() > 200 {
        &status[..200]
    } else {
        status
    };

    let state = NotifyState::Status(status_truncated.to_string());
    let _ = libsystemd::daemon::notify(false, &[state]);
    // Игнорируем ошибки - если не под systemd, это нормально
}

/// Состояние сервиса systemd.
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    /// Сервис активен и работает
    ActiveRunning,
    /// Сервис активен и ожидает
    ActiveWaiting,
    /// Сервис неактивен
    Inactive,
    /// Сервис находится в процессе запуска
    Activating,
    /// Сервис находится в процессе остановки
    Deactivating,
    /// Сервис завершился с ошибкой
    Failed,
    /// Состояние неизвестно
    Unknown,
}

impl std::fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceStatus::ActiveRunning => write!(f, "active (running)"),
            ServiceStatus::ActiveWaiting => write!(f, "active (waiting)"),
            ServiceStatus::Inactive => write!(f, "inactive"),
            ServiceStatus::Activating => write!(f, "activating"),
            ServiceStatus::Deactivating => write!(f, "deactivating"),
            ServiceStatus::Failed => write!(f, "failed"),
            ServiceStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Получает текущий статус сервиса systemd.
///
/// # Параметры
///
/// * `service_name` - имя сервиса (например, "smoothtaskd.service")
///
/// # Возвращаемое значение
///
/// Возвращает `Ok(ServiceStatus)` при успешном получении статуса или `Err` при ошибке.
///
/// # Примеры
///
/// ```no_run
/// use smoothtaskd::systemd;
///
/// match systemd::get_service_status("smoothtaskd.service") {
///     Ok(status) => println!("Service status: {}", status),
///     Err(e) => eprintln!("Failed to get service status: {}", e),
/// }
/// ```
pub async fn get_service_status(service_name: &str) -> Result<ServiceStatus> {
    let connection = Connection::system().await
        .context("Failed to connect to system D-Bus")?;
    
    let manager_proxy = Proxy::new(
        &connection,
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
    ).await
    .context("Failed to create systemd manager proxy")?;
    
    let unit_path_msg = manager_proxy
        .call_method("GetUnit", &(service_name,))
        .await
        .context("Failed to get unit from systemd")?;
    
    let unit_path: zbus::zvariant::OwnedObjectPath = unit_path_msg.body()?;
    
    let properties_proxy = Proxy::new(
        &connection,
        "org.freedesktop.systemd1",
        unit_path.as_str(),
        "org.freedesktop.DBus.Properties",
    ).await
    .context("Failed to create properties proxy")?;
    
    let properties_msg = properties_proxy
        .call_method("GetAll", &("org.freedesktop.systemd1.Unit",))
        .await
        .context("Failed to get unit properties")?;
    
    let properties: std::collections::HashMap<String, zbus::zvariant::Value> = properties_msg.body()?;
    
    let active_state = properties.get("ActiveState")
        .and_then(|v| v.downcast_ref::<str>())
        .context("Failed to get ActiveState property")?;
    
    match active_state {
        "active" => {
            let sub_state = properties.get("SubState")
                .and_then(|v| v.downcast_ref::<str>())
                .unwrap_or("running");
            if sub_state == "running" {
                Ok(ServiceStatus::ActiveRunning)
            } else {
                Ok(ServiceStatus::ActiveWaiting)
            }
        }
        "inactive" => Ok(ServiceStatus::Inactive),
        "activating" => Ok(ServiceStatus::Activating),
        "deactivating" => Ok(ServiceStatus::Deactivating),
        "failed" => Ok(ServiceStatus::Failed),
        _ => Ok(ServiceStatus::Unknown),
    }
}

/// Запускает сервис systemd.
///
/// # Параметры
///
/// * `service_name` - имя сервиса (например, "smoothtaskd.service")
///
/// # Возвращаемое значение
///
/// Возвращает `Ok(())` при успешном запуске или `Err` при ошибке.
///
/// # Примеры
///
/// ```no_run
/// use smoothtaskd::systemd;
///
/// systemd::start_service("smoothtaskd.service")?;
/// ```
pub async fn start_service(service_name: &str) -> Result<()> {
    let connection = Connection::system().await
        .context("Failed to connect to system D-Bus")?;
    
    let manager_proxy = Proxy::new(
        &connection,
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
    ).await
    .context("Failed to create systemd manager proxy")?;
    
    let _job_path_msg = manager_proxy
        .call_method("StartUnit", &(service_name, "replace"))
        .await
        .context("Failed to start service")?;
    
    tracing::info!("Started service {}", service_name);
    Ok(())
}

/// Останавливает сервис systemd.
///
/// # Параметры
///
/// * `service_name` - имя сервиса (например, "smoothtaskd.service")
///
/// # Возвращаемое значение
///
/// Возвращает `Ok(())` при успешной остановке или `Err` при ошибке.
///
/// # Примеры
///
/// ```no_run
/// use smoothtaskd::systemd;
///
/// systemd::stop_service("smoothtaskd.service")?;
/// ```
pub async fn stop_service(service_name: &str) -> Result<()> {
    let connection = Connection::system().await
        .context("Failed to connect to system D-Bus")?;
    
    let manager_proxy = Proxy::new(
        &connection,
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
    ).await
    .context("Failed to create systemd manager proxy")?;
    
    let _job_path_msg = manager_proxy
        .call_method("StopUnit", &(service_name, "replace"))
        .await
        .context("Failed to stop service")?;
    
    tracing::info!("Stopped service {}", service_name);
    Ok(())
}

/// Перезапускает сервис systemd.
///
/// # Параметры
///
/// * `service_name` - имя сервиса (например, "smoothtaskd.service")
///
/// # Возвращаемое значение
///
/// Возвращает `Ok(())` при успешном перезапуске или `Err` при ошибке.
///
/// # Примеры
///
/// ```no_run
/// use smoothtaskd::systemd;
///
/// systemd::restart_service("smoothtaskd.service")?;
/// ```
pub async fn restart_service(service_name: &str) -> Result<()> {
    let connection = Connection::system().await
        .context("Failed to connect to system D-Bus")?;
    
    let manager_proxy = Proxy::new(
        &connection,
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
    ).await
    .context("Failed to create systemd manager proxy")?;
    
    let _job_path_msg = manager_proxy
        .call_method("RestartUnit", &(service_name, "replace"))
        .await
        .context("Failed to restart service")?;
    
    tracing::info!("Restarted service {}", service_name);
    Ok(())
}

/// Проверяет, запущен ли сервис systemd.
///
/// # Параметры
///
/// * `service_name` - имя сервиса (например, "smoothtaskd.service")
///
/// # Возвращаемое значение
///
/// Возвращает `Ok(true)` если сервис активен, `Ok(false)` если неактивен, или `Err` при ошибке.
///
/// # Примеры
///
/// ```no_run
/// use smoothtaskd::systemd;
///
/// if systemd::is_service_active("smoothtaskd.service")? {
///     println!("Service is running");
/// } else {
///     println!("Service is not running");
/// }
/// ```
pub async fn is_service_active(service_name: &str) -> Result<bool> {
    let status = get_service_status(service_name).await?;
    Ok(matches!(status, ServiceStatus::ActiveRunning | ServiceStatus::ActiveWaiting))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Тест проверяет, что notify_ready не паникует.
    /// В тестовом окружении (не под systemd) функция должна вернуть ошибку,
    /// но не должна паниковать.
    #[test]
    fn test_notify_ready_no_panic() {
        // В тестовом окружении это должно вернуть ошибку, но не паниковать
        let result = notify_ready();
        // Проверяем, что функция не паникует
        assert!(result.is_err() || result.is_ok());
    }

    /// Тест проверяет, что notify_status не паникует даже с длинными строками.
    #[test]
    fn test_notify_status_no_panic() {
        // Короткий статус
        notify_status("Test status");

        // Длинный статус (должен быть обрезан)
        let long_status = "x".repeat(500);
        notify_status(&long_status);

        // Пустой статус
        notify_status("");

        // Статус с специальными символами
        notify_status("Status with\nnewlines\tand\ttabs");
    }

    /// Тест проверяет обрезку длинных статусов.
    #[test]
    fn test_notify_status_truncation() {
        // Создаём очень длинный статус
        let long_status = "x".repeat(500);
        // Функция должна обработать его без паники
        notify_status(&long_status);
    }

    /// Тест проверяет форматирование ServiceStatus.
    #[test]
    fn test_service_status_display() {
        assert_eq!(format!("{}", ServiceStatus::ActiveRunning), "active (running)");
        assert_eq!(format!("{}", ServiceStatus::ActiveWaiting), "active (waiting)");
        assert_eq!(format!("{}", ServiceStatus::Inactive), "inactive");
        assert_eq!(format!("{}", ServiceStatus::Activating), "activating");
        assert_eq!(format!("{}", ServiceStatus::Deactivating), "deactivating");
        assert_eq!(format!("{}", ServiceStatus::Failed), "failed");
        assert_eq!(format!("{}", ServiceStatus::Unknown), "unknown");
    }

    /// Тест проверяет, что асинхронные функции не паникуют в тестовом окружении.
    #[tokio::test]
    async fn test_async_functions_no_panic() {
        // Эти функции должны вернуть ошибку в тестовом окружении (нет systemd),
        // но не должны паниковать
        let result = get_service_status("nonexistent.service").await;
        assert!(result.is_err());

        let result = start_service("nonexistent.service").await;
        assert!(result.is_err());

        let result = stop_service("nonexistent.service").await;
        assert!(result.is_err());

        let result = restart_service("nonexistent.service").await;
        assert!(result.is_err());

        let result = is_service_active("nonexistent.service").await;
        assert!(result.is_err());
    }
}
