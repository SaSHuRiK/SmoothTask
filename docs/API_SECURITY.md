# SmoothTask Security API

Этот документ описывает API для работы с системой мониторинга безопасности SmoothTask.

## Обзор

Модуль безопасности SmoothTask предоставляет возможности для:

- Мониторинга подозрительных процессов
- Обнаружения аномальной активности
- Анализа паттернов поведения
- Управления событиями безопасности
- Интеграции с системой уведомлений

## Основные структуры

### SecurityEvent

Представляет событие безопасности в системе.

```rust
pub struct SecurityEvent {
    /// Уникальный идентификатор события
    pub event_id: String,
    /// Время возникновения события
    pub timestamp: DateTime<Utc>,
    /// Тип события
    pub event_type: SecurityEventType,
    /// Серьезность события
    pub severity: SecurityEventSeverity,
    /// Статус события
    pub status: SecurityEventStatus,
    /// Имя процесса (если применимо)
    pub process_name: Option<String>,
    /// Идентификатор процесса (если применимо)
    pub process_id: Option<i32>,
    /// Описание события
    pub description: String,
    /// Детали события
    pub details: Option<String>,
    /// Рекомендации по действиям
    pub recommendations: Option<String>,
    /// Время разрешения события (если решено)
    pub resolved_time: Option<DateTime<Utc>>,
}
```

### SecurityEventType

Типы событий безопасности:

- `SuspiciousProcess` - Подозрительный процесс
- `UnusualProcessActivity` - Необычная активность процесса
- `SuspiciousNetworkConnection` - Подозрительное сетевое соединение
- `AnomalousResourceUsage` - Аномальное использование ресурсов
- `SuspiciousFilesystemActivity` - Подозрительная активность файловой системы
- `PotentialAttack` - Потенциальная атака
- `Unknown` - Неизвестный тип события

### SecurityEventSeverity

Уровни серьезности:

- `Info` - Информационное событие
- `Low` - Низкий уровень угрозы
- `Medium` - Средний уровень угрозы
- `High` - Высокий уровень угрозы
- `Critical` - Критический уровень угрозы

### SecurityEventStatus

Статусы событий:

- `New` - Новое событие
- `Analyzing` - В процессе анализа
- `Analyzed` - Проанализировано
- `FalsePositive` - Ложное срабатывание
- `ConfirmedThreat` - Подтвержденная угроза
- `Ignored` - Игнорируется

## Основные функции

### Мониторинг безопасности

```rust
pub fn check_suspicious_processes(&self) -> Vec<SecurityEvent>
```

Обнаруживает подозрительные процессы в системе.

**Возвращает:** Вектор событий безопасности для подозрительных процессов.

### Анализ аномального использования ресурсов

```rust
pub fn check_anomalous_resource_usage(&self) -> Vec<SecurityEvent>
```

Анализирует использование ресурсов процессами и обнаруживает аномалии.

**Возвращает:** Вектор событий безопасности для аномального использования ресурсов.

### Обнаружение подозрительных паттернов

```rust
pub fn detect_resource_anomaly_patterns(&self) -> Vec<SecurityEvent>
```

Обнаруживает паттерны аномального использования ресурсов.

**Возвращает:** Вектор событий безопасности для обнаруженных паттернов.

### Управление событиями безопасности

```rust
pub fn manage_security_event(&mut self, event: SecurityEvent) -> Result<()>
```

Управляет событием безопасности, обновляя его статус и применяя соответствующие действия.

**Параметры:**
- `event`: Событие безопасности для управления

**Возвращает:** `Result<()>` - результат операции.

### Отправка уведомлений

```rust
pub fn send_security_notification(&self, event: &SecurityEvent) -> Result<()>
```

Отправляет уведомление о событии безопасности.

**Параметры:**
- `event`: Событие безопасности для уведомления

**Возвращает:** `Result<()>` - результат операции.

## Конфигурация

### SecurityMonitorConfig

```rust
pub struct SecurityMonitorConfig {
    /// Включить мониторинг безопасности
    pub enabled: bool,
    /// Интервал проверки безопасности
    pub check_interval: Duration,
    /// Максимальное количество хранимых событий
    pub max_event_history: usize,
    /// Пороги для обнаружения аномалий
    pub anomaly_thresholds: AnomalyThresholds,
    /// Список доверенных процессов
    pub trusted_processes: Vec<String>,
    /// Список подозрительных процессов
    pub suspicious_processes: Vec<String>,
    /// Настройки уведомлений
    pub notification_settings: SecurityNotificationSettings,
}
```

## Примеры использования

### Создание монитора безопасности

```rust
use smoothtask_core::health::security_monitoring::SecurityMonitor;

let config = SecurityMonitorConfig {
    enabled: true,
    check_interval: Duration::from_secs(60),
    max_event_history: 1000,
    anomaly_thresholds: AnomalyThresholds::default(),
    trusted_processes: vec!["smoothtaskd".to_string()],
    suspicious_processes: vec!["malware".to_string()],
    notification_settings: SecurityNotificationSettings::default(),
};

let security_monitor = SecurityMonitor::new(config);
```

### Обнаружение подозрительных процессов

```rust
let suspicious_events = security_monitor.check_suspicious_processes();
for event in suspicious_events {
    println!("Обнаружено подозрительное событие: {:?}", event);
}
```

### Анализ аномального использования ресурсов

```rust
let anomaly_events = security_monitor.check_anomalous_resource_usage();
for event in anomaly_events {
    println!("Обнаружено аномальное использование ресурсов: {:?}", event);
}
```

### Управление событиями безопасности

```rust
let mut security_monitor = SecurityMonitor::new(config);
let event = SecurityEvent {
    event_id: "test-event".to_string(),
    timestamp: Utc::now(),
    event_type: SecurityEventType::SuspiciousProcess,
    severity: SecurityEventSeverity::High,
    status: SecurityEventStatus::New,
    process_name: Some("malware".to_string()),
    process_id: Some(1234),
    description: "Подозрительный процесс обнаружен".to_string(),
    details: None,
    recommendations: None,
    resolved_time: None,
};

security_monitor.manage_security_event(event)?;
```

## Интеграция с системой

Модуль безопасности интегрирован с основной системой мониторинга SmoothTask и может быть использован вместе с другими модулями для комплексного мониторинга и управления системой.

## Тестирование

Для тестирования функциональности безопасности доступны comprehensive тесты в `smoothtask-core/tests/security_monitoring_integration_test.rs`.

## Безопасность

Модуль безопасности разработан с учетом принципов безопасности и предоставляет возможности для обнаружения и предотвращения потенциальных угроз в системе.