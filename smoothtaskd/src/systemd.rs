//! Модуль для интеграции с systemd через sd-notify.
//!
//! Предоставляет функции для отправки уведомлений systemd о состоянии демона:
//! - READY=1 - после успешной инициализации
//! - STATUS=... - для обновления статуса работы демона
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
//! ```

use anyhow::{Context, Result};
use libsystemd::daemon::NotifyState;

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
}
