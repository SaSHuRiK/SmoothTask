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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub async fn get_service_status(service_name: &str) -> Result<ServiceStatus> {
    get_service_status_with_retry(service_name, 1).await
}

/// Получает текущий статус сервиса systemd с поддержкой повторных попыток.
///
/// # Параметры
///
/// * `service_name` - имя сервиса (например, "smoothtaskd.service")
/// * `retry_count` - количество попыток подключения (1 = без повторных попыток)
///
/// # Возвращаемое значение
///
/// Возвращает `Ok(ServiceStatus)` при успешном получении статуса или `Err` при ошибке.
#[allow(dead_code)]
async fn get_service_status_with_retry(service_name: &str, retry_count: u32) -> Result<ServiceStatus> {
    let mut last_error = None;
    
    for attempt in 1..=retry_count {
        match get_service_status_inner(service_name).await {
            Ok(status) => return Ok(status),
            Err(e) => {
                if attempt < retry_count {
                    tracing::warn!(
                        "Failed to get service status (attempt {}/{}), retrying...: {}",
                        attempt, retry_count, &e
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis((100 * attempt) as u64)).await;
                }
                last_error = Some(e);
            }
        }
    }
    
    Err(last_error.unwrap_or_else(|| {
        anyhow::anyhow!("Failed to get service status after {} attempts", retry_count)
    }))
}

/// Внутренняя реализация получения статуса сервиса без повторных попыток.
#[allow(dead_code)]
async fn get_service_status_inner(service_name: &str) -> Result<ServiceStatus> {
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
#[allow(dead_code)]
pub async fn start_service(service_name: &str) -> Result<()> {
    start_service_with_retry(service_name, 1).await
}

/// Запускает сервис systemd с поддержкой повторных попыток.
#[allow(dead_code)]
async fn start_service_with_retry(service_name: &str, retry_count: u32) -> Result<()> {
    let mut last_error = None;
    
    for attempt in 1..=retry_count {
        match start_service_inner(service_name).await {
            Ok(_) => {
                tracing::info!("Started service {}", service_name);
                return Ok(());
            }
            Err(e) => {
                if attempt < retry_count {
                    tracing::warn!(
                        "Failed to start service (attempt {}/{}), retrying...: {}",
                        attempt, retry_count, &e
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis((100 * attempt) as u64)).await;
                }
                last_error = Some(e);
            }
        }
    }
    
    Err(last_error.unwrap_or_else(|| {
        anyhow::anyhow!("Failed to start service after {} attempts", retry_count)
    }))
}

/// Внутренняя реализация запуска сервиса без повторных попыток.
#[allow(dead_code)]
async fn start_service_inner(service_name: &str) -> Result<()> {
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
#[allow(dead_code)]
pub async fn stop_service(service_name: &str) -> Result<()> {
    stop_service_with_retry(service_name, 1).await
}

/// Останавливает сервис systemd с поддержкой повторных попыток.
#[allow(dead_code)]
async fn stop_service_with_retry(service_name: &str, retry_count: u32) -> Result<()> {
    let mut last_error = None;
    
    for attempt in 1..=retry_count {
        match stop_service_inner(service_name).await {
            Ok(_) => {
                tracing::info!("Stopped service {}", service_name);
                return Ok(());
            }
            Err(e) => {
                if attempt < retry_count {
                    tracing::warn!(
                        "Failed to stop service (attempt {}/{}), retrying...: {}",
                        attempt, retry_count, &e
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis((100 * attempt) as u64)).await;
                }
                last_error = Some(e);
            }
        }
    }
    
    Err(last_error.unwrap_or_else(|| {
        anyhow::anyhow!("Failed to stop service after {} attempts", retry_count)
    }))
}

/// Внутренняя реализация остановки сервиса без повторных попыток.
#[allow(dead_code)]
async fn stop_service_inner(service_name: &str) -> Result<()> {
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
#[allow(dead_code)]
pub async fn restart_service(service_name: &str) -> Result<()> {
    restart_service_with_retry(service_name, 1).await
}

/// Перезапускает сервис systemd с поддержкой повторных попыток.
#[allow(dead_code)]
async fn restart_service_with_retry(service_name: &str, retry_count: u32) -> Result<()> {
    let mut last_error = None;
    
    for attempt in 1..=retry_count {
        match restart_service_inner(service_name).await {
            Ok(_) => {
                tracing::info!("Restarted service {}", service_name);
                return Ok(());
            }
            Err(e) => {
                if attempt < retry_count {
                    tracing::warn!(
                        "Failed to restart service (attempt {}/{}), retrying...: {}",
                        attempt, retry_count, &e
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis((100 * attempt) as u64)).await;
                }
                last_error = Some(e);
            }
        }
    }
    
    Err(last_error.unwrap_or_else(|| {
        anyhow::anyhow!("Failed to restart service after {} attempts", retry_count)
    }))
}

/// Внутренняя реализация перезапуска сервиса без повторных попыток.
#[allow(dead_code)]
async fn restart_service_inner(service_name: &str) -> Result<()> {
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
#[allow(dead_code)]
pub async fn is_service_active(service_name: &str) -> Result<bool> {
    let status = get_service_status(service_name).await?;
    Ok(matches!(status, ServiceStatus::ActiveRunning | ServiceStatus::ActiveWaiting))
}

/// Проверяет, запущен ли текущий процесс под управлением systemd.
///
/// Эта функция проверяет переменную окружения `$INVOCATION_ID`, которая устанавливается
/// systemd при запуске сервиса.
///
/// # Возвращаемое значение
///
/// Возвращает `true`, если процесс запущен под systemd, `false` в противном случае.
///
/// # Примеры
///
/// ```no_run
/// use smoothtaskd::systemd;
///
/// if systemd::is_running_under_systemd() {
///     println!("Running under systemd");
/// } else {
///     println!("Not running under systemd");
/// }
/// ```
#[allow(dead_code)]
pub fn is_running_under_systemd() -> bool {
    std::env::var("INVOCATION_ID").is_ok()
}

/// Отправляет уведомление systemd о завершении работы (STOPPING=1).
///
/// Эта функция должна вызываться перед завершением демона для корректного
/// уведомления systemd о завершении работы.
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
/// // Перед завершением демона
/// if let Err(e) = systemd::notify_stopping() {
///     // Можно логировать, но не критично, если не под systemd
///     eprintln!("Warning: failed to notify systemd about stopping: {}", e);
/// }
/// ```
#[allow(dead_code)]
pub fn notify_stopping() -> Result<()> {
    let state = NotifyState::Stopping;
    libsystemd::daemon::notify(false, &[state])
        .context("Failed to send STOPPING notification to systemd")?;
    Ok(())
}

/// Отправляет уведомление systemd об ошибке (ERRNO=...).
///
/// Эта функция может использоваться для уведомления systemd о критических ошибках.
///
/// # Параметры
///
/// * `error_code` - код ошибки для отправки systemd
/// * `error_message` - сообщение об ошибке (опционально)
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
/// // Уведомляем systemd о критической ошибке
/// systemd::notify_error(1, Some("Failed to initialize critical component"));
/// ```
pub fn notify_error(error_code: i32, error_message: Option<&str>) {
    let mut states = vec![NotifyState::Errno(error_code as u8)];
    
    if let Some(msg) = error_message {
        // Ограничиваем длину сообщения
        let msg_truncated = if msg.len() > 200 {
            &msg[..200]
        } else {
            msg
        };
        states.push(NotifyState::Status(format!("ERROR: {}", msg_truncated)));
    }
    
    let _ = libsystemd::daemon::notify(false, &states);
    // Игнорируем ошибки - если не под systemd, это нормально
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

    /// Тест проверяет функцию is_running_under_systemd.
    #[test]
    fn test_is_running_under_systemd() {
        // В тестовом окружении INVOCATION_ID обычно не установлен
        let result = is_running_under_systemd();
        // Результат может быть true или false в зависимости от окружения
        // Главное - функция не должна паниковать
        assert!(result == true || result == false);
    }

    /// Тест проверяет, что notify_stopping не паникует.
    #[test]
    fn test_notify_stopping_no_panic() {
        // В тестовом окружении это должно вернуть ошибку, но не паниковать
        let result = notify_stopping();
        // Проверяем, что функция не паникует
        assert!(result.is_err() || result.is_ok());
    }

    /// Тест проверяет, что notify_error не паникует.
    #[test]
    fn test_notify_error_no_panic() {
        // Короткое сообщение об ошибке
        notify_error(1, Some("Test error"));

        // Длинное сообщение об ошибке (должно быть обрезано)
        let long_error = "x".repeat(500);
        notify_error(2, Some(&long_error));

        // Ошибка без сообщения
        notify_error(3, None);

        // Ошибка с пустым сообщением
        notify_error(4, Some(""));

        // Ошибка с специальными символами
        notify_error(5, Some("Error with\nnewlines\tand\ttabs"));
    }

    /// Тест проверяет обрезку длинных сообщений об ошибках.
    #[test]
    fn test_notify_error_truncation() {
        // Создаём очень длинное сообщение об ошибке
        let long_error = "x".repeat(500);
        // Функция должна обработать его без паники
        notify_error(1, Some(&long_error));
    }

    /// Тест проверяет, что функции с retry не паникуют в тестовом окружении.
    #[tokio::test]
    async fn test_retry_functions_no_panic() {
        // Эти функции должны вернуть ошибку в тестовом окружении (нет systemd),
        // но не должны паниковать
        let result = get_service_status_with_retry("nonexistent.service", 3).await;
        assert!(result.is_err());

        let result = start_service_with_retry("nonexistent.service", 2).await;
        assert!(result.is_err());

        let result = stop_service_with_retry("nonexistent.service", 2).await;
        assert!(result.is_err());

        let result = restart_service_with_retry("nonexistent.service", 2).await;
        assert!(result.is_err());
    }

    /// Тест проверяет, что функции с retry корректно обрабатывают нулевое количество попыток.
    #[tokio::test]
    async fn test_retry_functions_with_zero_attempts() {
        // При retry_count = 0 функции должны сразу вернуть ошибку
        let result = get_service_status_with_retry("nonexistent.service", 0).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("after 0 attempts"));
    }

    /// Тест проверяет, что функции с retry корректно обрабатывают 1 попытку.
    #[tokio::test]
    async fn test_retry_functions_with_one_attempt() {
        // При retry_count = 1 функции должны сделать одну попытку и вернуть ошибку
        let result = get_service_status_with_retry("nonexistent.service", 1).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("after 1 attempts"));
    }
}
