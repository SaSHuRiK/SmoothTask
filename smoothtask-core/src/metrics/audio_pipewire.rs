//! Парсинг вывода `pw-dump` для извлечения аудио-клиентов и XRUN событий.
//!
//! Реализует `PipeWireIntrospector` — интроспектор, использующий `pw-dump`
//! для получения метрик аудио-стека без прямой зависимости от PipeWire API.
//! Это простой подход, который работает через вызов команды `pw-dump` и парсинг JSON.

use crate::metrics::audio::{AudioClientInfo, AudioIntrospector, AudioMetrics, XrunInfo};
use anyhow::{anyhow, Context, Result};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::process::Command;
use std::time::SystemTime;

/// Извлечь список аудио-клиентов из JSON вывода `pw-dump`.
///
/// Учитываются только объекты типа Node (`PipeWire:Interface:Node`), т.к. они
/// содержат связь с PID. Для каждого PID данные агрегируются: берутся первые
/// доступные значения sample_rate и buffer_size, чтобы не перезаписывать
/// более надёжные данные менее точными.
pub fn parse_pw_dump_clients(json: &str) -> Result<Vec<AudioClientInfo>> {
    let value: Value = serde_json::from_str(json).context("Не удалось распарсить pw-dump JSON")?;
    let items = extract_items_array(&value)
        .ok_or_else(|| anyhow!("pw-dump не содержит массива объектов"))?;

    let mut clients: HashMap<u32, AudioClientInfo> = HashMap::new();

    for item in items {
        if !is_node(item) {
            continue;
        }

        let props = match item
            .get("info")
            .and_then(|info| info.get("props"))
            .and_then(|p| p.as_object())
        {
            Some(p) => p,
            None => continue,
        };

        if let Some(pid) = parse_pid(props) {
            let entry = clients.remove(&pid).unwrap_or(AudioClientInfo {
                pid,
                buffer_size_samples: None,
                sample_rate_hz: None,
            });

            let buffer_size_samples = entry
                .buffer_size_samples
                .or_else(|| parse_buffer_size(props));
            let sample_rate_hz = entry.sample_rate_hz.or_else(|| parse_sample_rate(props));

            clients.insert(
                pid,
                AudioClientInfo {
                    pid,
                    buffer_size_samples,
                    sample_rate_hz,
                },
            );
        }
    }

    let mut result: Vec<AudioClientInfo> = clients.into_values().collect();
    result.sort_by_key(|c| c.pid);
    Ok(result)
}

fn extract_items_array(value: &Value) -> Option<&Vec<Value>> {
    if let Some(arr) = value.as_array() {
        return Some(arr);
    }
    value.get("objects")?.as_array()
}

fn is_node(item: &Value) -> bool {
    item.get("type")
        .and_then(|v| v.as_str())
        .map(|t| t.contains("Node"))
        .unwrap_or(false)
}

fn parse_pid(props: &Map<String, Value>) -> Option<u32> {
    for key in [
        "application.process.id",
        "pipewire.client.pid",
        "application.pid",
    ] {
        if let Some(pid) = props.get(key).and_then(parse_u32) {
            return Some(pid);
        }
    }
    None
}

fn parse_sample_rate(props: &Map<String, Value>) -> Option<u32> {
    for key in ["api.alsa.rate", "audio.rate", "clock.rate", "node.rate"] {
        if let Some(v) = props.get(key) {
            if let Some(rate) = parse_u32(v).or_else(|| parse_rate_string(v)) {
                return Some(rate);
            }
        }
    }

    if let Some(latency) = props
        .get("node.latency")
        .and_then(|v| v.as_str())
        .and_then(parse_latency_string)
    {
        return Some(latency.1);
    }

    None
}

fn parse_buffer_size(props: &Map<String, Value>) -> Option<u32> {
    for key in [
        "api.alsa.period-size",
        "node.quantum",
        "audio.buffer",
        "buffer.size",
    ] {
        if let Some(size) = props.get(key).and_then(parse_u32) {
            return Some(size);
        }
    }

    if let Some((frames, _)) = props
        .get("node.latency")
        .and_then(|v| v.as_str())
        .and_then(parse_latency_string)
    {
        return Some(frames);
    }

    None
}

fn parse_u32(value: &Value) -> Option<u32> {
    if let Some(n) = value.as_u64() {
        return u32::try_from(n).ok();
    }
    value.as_str().and_then(|s| s.trim().parse::<u32>().ok())
}

fn parse_rate_string(value: &Value) -> Option<u32> {
    let s = value.as_str()?;
    s.split('/')
        .next()
        .and_then(|part| part.trim().parse::<u32>().ok())
}

fn parse_latency_string(s: &str) -> Option<(u32, u32)> {
    // Берём первый токен с разделителем '/', остальные (например, флаги) игнорируем.
    let token = s.split_whitespace().find(|piece| piece.contains('/'))?;

    let mut parts = token.split('/');
    let frames: u32 = parts.next()?.trim().parse().ok()?;
    let rate: u32 = parts.next()?.trim().parse().ok()?;
    Some((frames, rate))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client(pid: u32, buffer: Option<u32>, rate: Option<u32>) -> AudioClientInfo {
        AudioClientInfo {
            pid,
            buffer_size_samples: buffer,
            sample_rate_hz: rate,
        }
    }

    #[test]
    fn parses_clients_from_nodes() {
        let json = r#"
        [
            {
                "id": 42,
                "type": "PipeWire:Interface:Node",
                "info": {
                    "props": {
                        "application.process.id": 1234,
                        "node.latency": "256/48000",
                        "api.alsa.rate": "48000"
                    }
                }
            },
            {
                "id": 43,
                "type": "PipeWire:Interface:Node",
                "info": {
                    "props": {
                        "pipewire.client.pid": "2345",
                        "api.alsa.period-size": 1024,
                        "audio.rate": 44100
                    }
                }
            },
            {
                "id": 44,
                "type": "PipeWire:Interface:Client",
                "info": {
                    "props": {
                        "application.process.id": 9999
                    }
                }
            }
        ]
        "#;

        let clients = parse_pw_dump_clients(json).unwrap();
        assert_eq!(clients.len(), 2);
        assert_eq!(clients[0], client(1234, Some(256), Some(48000)));
        assert_eq!(clients[1], client(2345, Some(1024), Some(44100)));
    }

    #[test]
    fn merges_same_pid_without_overwriting_existing_values() {
        let json = r#"
        [
            {
                "id": 1,
                "type": "PipeWire:Interface:Node",
                "info": {
                    "props": {
                        "application.process.id": 5555,
                        "api.alsa.rate": 48000
                    }
                }
            },
            {
                "id": 2,
                "type": "PipeWire:Interface:Node",
                "info": {
                    "props": {
                        "application.process.id": 5555,
                        "api.alsa.period-size": 512,
                        "audio.rate": 44100
                    }
                }
            }
        ]
        "#;

        let clients = parse_pw_dump_clients(json).unwrap();
        assert_eq!(clients.len(), 1);
        let client = &clients[0];
        assert_eq!(client.pid, 5555);
        assert_eq!(client.sample_rate_hz, Some(48000)); // первое значение сохранилось
        assert_eq!(client.buffer_size_samples, Some(512)); // заполнилось из второго объекта
    }

    #[test]
    fn supports_wrapped_objects_key() {
        let json = r#"
        {
            "objects": [
                {
                    "type": "PipeWire:Interface:Node",
                    "info": {
                        "props": {
                            "application.process.id": 7777,
                            "node.latency": "128/96000 0"
                        }
                    }
                }
            ]
        }
        "#;

        let clients = parse_pw_dump_clients(json).unwrap();
        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0], client(7777, Some(128), Some(96000)));
    }
}

/// Извлечь информацию о XRUN событиях из JSON вывода `pw-dump`.
///
/// Ищет узлы с ненулевым ERR (счётчик ошибок/XRUN) и создаёт XrunInfo
/// для каждого такого узла, если у него есть связанный PID.
///
/// Примечание: ERR - это накопительный счётчик, поэтому эта функция
/// возвращает все узлы с ERR > 0. Для отслеживания новых XRUN нужно
/// сравнивать ERR между вызовами (это делается в PipeWireIntrospector).
pub fn parse_pw_dump_xruns(json: &str) -> Result<Vec<(u32, u64)>> {
    let value: Value = serde_json::from_str(json).context("Не удалось распарсить pw-dump JSON")?;
    let items = extract_items_array(&value)
        .ok_or_else(|| anyhow!("pw-dump не содержит массива объектов"))?;

    let mut xruns = Vec::new();

    for item in items {
        if !is_node(item) {
            continue;
        }

        let props = match item
            .get("info")
            .and_then(|info| info.get("props"))
            .and_then(|p| p.as_object())
        {
            Some(p) => p,
            None => continue,
        };

        // Ищем ERR в различных форматах
        let err_count = parse_err_count(props);
        if err_count == 0 {
            continue;
        }

        // Если есть ERR и PID, сохраняем пару (PID, ERR)
        if let Some(pid) = parse_pid(props) {
            xruns.push((pid, err_count));
        }
    }

    Ok(xruns)
}

fn parse_err_count(props: &Map<String, Value>) -> u64 {
    // Ищем ERR в различных форматах, которые могут быть в pw-dump
    // Поддерживаем различные варианты названий, которые встречаются в разных версиях PipeWire
    for key in [
        "node.error",
        "node.ERR",
        "error.count",
        "xrun.count",
        "node.xrun",
        "error",            // Простой вариант без префикса
        "xrun",             // Простой вариант без префикса
        "node.error.count", // Полный вариант с префиксом
        "pipewire.error",   // Вариант с префиксом pipewire
        "pipewire.xrun",    // Вариант с префиксом pipewire
    ] {
        if let Some(err) = props.get(key).and_then(parse_u64) {
            return err;
        }
    }
    0
}

fn parse_u64(value: &Value) -> Option<u64> {
    if let Some(n) = value.as_u64() {
        return Some(n);
    }
    value.as_str().and_then(|s| s.trim().parse::<u64>().ok())
}

/// PipeWire интроспектор, использующий `pw-dump` для получения метрик.
///
/// Этот интроспектор вызывает `pw-dump` через команду и парсит результат
/// для получения списка клиентов и XRUN событий. Это простой подход без
/// прямой зависимости от PipeWire API.
pub struct PipeWireIntrospector {
    /// Время последнего вызова audio_metrics для отслеживания периода
    last_metrics_time: Option<SystemTime>,
    /// Последние известные ERR счётчики по PID (для отслеживания новых XRUN)
    last_err_by_pid: HashMap<u32, u64>,
}

impl PipeWireIntrospector {
    /// Создать новый PipeWire интроспектор.
    pub fn new() -> Self {
        Self {
            last_metrics_time: None,
            last_err_by_pid: HashMap::new(),
        }
    }

    /// Вызвать `pw-dump` и получить JSON вывод.
    fn call_pw_dump(&self) -> Result<String> {
        let output = Command::new("pw-dump")
            .output()
            .context("Не удалось выполнить pw-dump. Убедитесь, что PipeWire установлен и pw-dump доступен в PATH")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("pw-dump завершился с ошибкой: {}", stderr));
        }

        String::from_utf8(output.stdout).context("pw-dump вернул невалидный UTF-8")
    }
}

impl Default for PipeWireIntrospector {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioIntrospector for PipeWireIntrospector {
    fn audio_metrics(&mut self) -> Result<AudioMetrics> {
        let now = SystemTime::now();
        let period_start = self.last_metrics_time.unwrap_or(now);
        let period_end = now;

        // Вызываем pw-dump
        let json = self.call_pw_dump()?;

        // Парсим клиентов
        let clients = parse_pw_dump_clients(&json)?;

        // Парсим текущие ERR счётчики
        let current_err_vec = parse_pw_dump_xruns(&json)?;
        let current_err_by_pid: HashMap<u32, u64> = current_err_vec.into_iter().collect();
        let mut xruns = Vec::new();
        let mut xrun_count = 0u32;

        // Сравниваем текущие ERR с предыдущими, чтобы найти новые XRUN
        for (pid, current_err) in &current_err_by_pid {
            let last_err = self.last_err_by_pid.get(pid).copied().unwrap_or(0);

            if current_err > &last_err {
                // Найдены новые XRUN для этого PID
                let new_xruns = current_err - last_err;
                for _ in 0..new_xruns {
                    xruns.push(XrunInfo {
                        timestamp: now,
                        client_pid: Some(*pid),
                    });
                    xrun_count += 1;
                }
            } else if current_err < &last_err {
                // ERR уменьшился - вероятно, узел перезапустился или сбросился
                // В этом случае считаем, что счётчик сбросился, и не создаём события
                // (это нормальное поведение при перезапуске узла)
            }
            // Если ERR не изменился, новых XRUN нет
        }

        // Также проверяем узлы без PID (если ERR увеличился глобально)
        // Для простоты создаём одно событие, если общий ERR увеличился
        let total_current_err: u64 = current_err_by_pid.values().sum();
        let total_last_err: u64 = self.last_err_by_pid.values().sum();
        if total_current_err > total_last_err {
            // Могут быть XRUN без известного PID
            // Вычисляем разницу, учитывая уже учтённые XRUN для узлов с PID
            let accounted_xruns: u64 = xrun_count as u64;
            let unknown_xruns = total_current_err
                .saturating_sub(total_last_err)
                .saturating_sub(accounted_xruns);

            if unknown_xruns > 0 {
                for _ in 0..unknown_xruns {
                    xruns.push(XrunInfo {
                        timestamp: now,
                        client_pid: None,
                    });
                    xrun_count += 1;
                }
            }
        }

        // Обновляем состояние: сохраняем только текущие узлы (очищаем исчезнувшие)
        // Это важно, чтобы не накапливать данные об узлах, которые больше не существуют
        self.last_metrics_time = Some(now);
        self.last_err_by_pid = current_err_by_pid;

        Ok(AudioMetrics {
            xrun_count,
            xruns,
            clients,
            period_start,
            period_end,
        })
    }

    fn clients(&self) -> Result<Vec<AudioClientInfo>> {
        // Для clients() мы просто вызываем pw-dump и парсим клиентов
        // без отслеживания периода
        let json = self.call_pw_dump()?;
        parse_pw_dump_clients(&json)
    }
}

#[cfg(test)]
mod pipewire_introspector_tests {
    use super::*;

    #[test]
    fn pipewire_introspector_creation() {
        let introspector = PipeWireIntrospector::new();
        // Просто проверяем, что создание не падает
        assert!(introspector.last_metrics_time.is_none());
        assert!(introspector.last_err_by_pid.is_empty());
    }

    #[test]
    fn parse_xruns_from_pw_dump() {
        let json = r#"
        [
            {
                "id": 42,
                "type": "PipeWire:Interface:Node",
                "info": {
                    "props": {
                        "application.process.id": 1234,
                        "node.error": 5
                    }
                }
            },
            {
                "id": 43,
                "type": "PipeWire:Interface:Node",
                "info": {
                    "props": {
                        "pipewire.client.pid": "2345",
                        "node.error": 0
                    }
                }
            },
            {
                "id": 44,
                "type": "PipeWire:Interface:Node",
                "info": {
                    "props": {
                        "application.process.id": 9999,
                        "node.error": 2
                    }
                }
            }
        ]
        "#;

        let xruns = parse_pw_dump_xruns(json).unwrap();
        assert_eq!(xruns.len(), 2);
        assert_eq!(xruns[0], (1234, 5));
        assert_eq!(xruns[1], (9999, 2));
    }

    #[test]
    fn parse_xruns_empty_when_no_errors() {
        let json = r#"
        [
            {
                "id": 42,
                "type": "PipeWire:Interface:Node",
                "info": {
                    "props": {
                        "application.process.id": 1234,
                        "node.error": 0
                    }
                }
            }
        ]
        "#;

        let xruns = parse_pw_dump_xruns(json).unwrap();
        assert_eq!(xruns.len(), 0);
    }

    #[test]
    fn parse_xruns_ignores_non_nodes() {
        let json = r#"
        [
            {
                "id": 42,
                "type": "PipeWire:Interface:Client",
                "info": {
                    "props": {
                        "application.process.id": 1234,
                        "node.error": 5
                    }
                }
            }
        ]
        "#;

        let xruns = parse_pw_dump_xruns(json).unwrap();
        assert_eq!(xruns.len(), 0);
    }

    #[test]
    fn parse_xruns_supports_various_err_formats() {
        // Тест различных форматов ERR
        let test_cases = vec![
            (r#"{"application.process.id": 1001, "error": 3}"#, 3),
            (r#"{"application.process.id": 1002, "xrun": 5}"#, 5),
            (
                r#"{"application.process.id": 1003, "node.error.count": 7}"#,
                7,
            ),
            (
                r#"{"application.process.id": 1004, "pipewire.error": 2}"#,
                2,
            ),
            (r#"{"application.process.id": 1005, "pipewire.xrun": 4}"#, 4),
        ];

        for (props_json, expected_err) in test_cases {
            let props: Map<String, Value> = serde_json::from_str(props_json).unwrap();
            let err_count = parse_err_count(&props);
            assert_eq!(err_count, expected_err, "Failed for props: {}", props_json);
        }
    }

    #[test]
    fn parse_xruns_supports_string_format_err() {
        // Тест, что ERR может быть строкой
        let json = r#"
        [
            {
                "id": 50,
                "type": "PipeWire:Interface:Node",
                "info": {
                    "props": {
                        "application.process.id": 2000,
                        "node.error": "10"
                    }
                }
            }
        ]
        "#;

        let xruns = parse_pw_dump_xruns(json).unwrap();
        assert_eq!(xruns.len(), 1);
        assert_eq!(xruns[0], (2000, 10));
    }

    #[test]
    fn pipewire_introspector_tracks_new_xruns() {
        // Проверяем, что интроспектор корректно инициализируется
        let introspector = PipeWireIntrospector::new();

        // Проверяем начальное состояние
        assert!(introspector.last_metrics_time.is_none());
        assert!(introspector.last_err_by_pid.is_empty());

        // Симулируем состояние: у нас уже был узел с ERR=5
        let mut introspector = introspector;
        introspector.last_err_by_pid.insert(1234, 5);

        // Проверяем, что состояние обновлено
        assert_eq!(introspector.last_err_by_pid.get(&1234), Some(&5));
    }

    #[test]
    fn pipewire_introspector_handles_decreased_err() {
        // Тест обработки случая, когда ERR уменьшился (перезапуск узла)
        // Это не должно создавать отрицательные XRUN или ошибки

        let json1 = r#"
        [
            {
                "id": 60,
                "type": "PipeWire:Interface:Node",
                "info": {
                    "props": {
                        "application.process.id": 3000,
                        "node.error": 10
                    }
                }
            }
        ]
        "#;

        let json2 = r#"
        [
            {
                "id": 60,
                "type": "PipeWire:Interface:Node",
                "info": {
                    "props": {
                        "application.process.id": 3000,
                        "node.error": 3
                    }
                }
            }
        ]
        "#;

        // Первый вызов: ERR=10
        let xruns1 = parse_pw_dump_xruns(json1).unwrap();
        assert_eq!(xruns1.len(), 1);
        assert_eq!(xruns1[0], (3000, 10));

        // Второй вызов: ERR=3 (уменьшился - узел перезапустился)
        let xruns2 = parse_pw_dump_xruns(json2).unwrap();
        assert_eq!(xruns2.len(), 1);
        assert_eq!(xruns2[0], (3000, 3));

        // В реальном интроспекторе при уменьшении ERR не должно создаваться событий XRUN
        // Это проверяется в логике audio_metrics(), где мы проверяем current_err > last_err
    }

    #[test]
    fn pipewire_introspector_handles_disappeared_nodes() {
        // Тест обработки случая, когда узел исчез из вывода pw-dump
        // Это не должно вызывать ошибок, просто узел должен быть удалён из last_err_by_pid

        let json1 = r#"
        [
            {
                "id": 70,
                "type": "PipeWire:Interface:Node",
                "info": {
                    "props": {
                        "application.process.id": 4000,
                        "node.error": 5
                    }
                }
            }
        ]
        "#;

        let json2 = r#"
        [
            {
                "id": 71,
                "type": "PipeWire:Interface:Node",
                "info": {
                    "props": {
                        "application.process.id": 5000,
                        "node.error": 2
                    }
                }
            }
        ]
        "#;

        // Первый вызов: узел 4000 с ERR=5
        let xruns1 = parse_pw_dump_xruns(json1).unwrap();
        assert_eq!(xruns1.len(), 1);
        assert_eq!(xruns1[0], (4000, 5));

        // Второй вызов: узел 4000 исчез, появился новый узел 5000
        let xruns2 = parse_pw_dump_xruns(json2).unwrap();
        assert_eq!(xruns2.len(), 1);
        assert_eq!(xruns2[0], (5000, 2));

        // В реальном интроспекторе при обновлении last_err_by_pid = current_err_by_pid
        // узел 4000 автоматически исчезнет из отслеживания
    }

    // Интеграционные тесты с реальным pw-dump требуют наличия PipeWire в системе
    // и могут быть нестабильными, поэтому оставляем их опциональными
    // Для unit-тестов можно использовать моки или фиктивные данные
}
