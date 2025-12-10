//! Парсинг вывода `pw-dump` для извлечения аудио-клиентов.
//!
//! Цель — дешёвый путь получить PID аудио-клиентов и их базовые параметры
//! (sample rate, размер буфера) без прямой зависимости от PipeWire API.
//! Модуль пригодится как вспомогательный слой для будущего PipeWireIntrospector.

use crate::metrics::audio::AudioClientInfo;
use anyhow::{anyhow, Context, Result};
use serde_json::{Map, Value};
use std::collections::HashMap;

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

fn extract_items_array<'a>(value: &'a Value) -> Option<&'a Vec<Value>> {
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
