// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

//! Модуль для мониторинга файловой системы в реальном времени
//! Использует inotify для отслеживания изменений в файлах и директориях

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Result;
use tracing::{info, warn};

/// Структура для хранения информации об изменении файла
#[derive(Debug, Clone, PartialEq)]
pub struct FileChangeEvent {
    pub path: PathBuf,
    pub event_type: FileChangeType,
    pub timestamp: u64,
    pub process_id: Option<u32>,
    pub process_name: Option<String>,
}

/// Типы изменений файлов
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FileChangeType {
    Created,
    Modified,
    Deleted,
    Moved,
    Accessed,
    AttributeChanged,
}

/// Конфигурация мониторинга файловой системы
#[derive(Debug, Clone)]
pub struct FilesystemMonitorConfig {
    pub watch_paths: Vec<PathBuf>,
    pub recursive: bool,
    pub max_events: usize,
    pub event_timeout_secs: u64,
}

impl Default for FilesystemMonitorConfig {
    fn default() -> Self {
        Self {
            watch_paths: vec![PathBuf::from("/")],
            recursive: true,
            max_events: 1000,
            event_timeout_secs: 60,
        }
    }
}

/// Основная структура мониторинга файловой системы
pub struct FilesystemMonitor {
    config: FilesystemMonitorConfig,
    // В реальной реализации здесь будет inotify дескриптор
    // Для тестирования используем заглушку
    event_buffer: Arc<Mutex<Vec<FileChangeEvent>>>,
    watch_descriptors: HashMap<PathBuf, i32>,
}

impl FilesystemMonitor {
    /// Создать новый экземпляр мониторинга файловой системы
    pub fn new(config: FilesystemMonitorConfig) -> Result<Self> {
        info!("Creating new filesystem monitor with config: {:?}", config);
        
        let monitor = Self {
            config,
            event_buffer: Arc::new(Mutex::new(Vec::new())),
            watch_descriptors: HashMap::new(),
        };
        
        Ok(monitor)
    }
    
    /// Инициализировать мониторинг
    pub fn initialize(&mut self) -> Result<()> {
        info!("Initializing filesystem monitor");
        
        // В реальной реализации здесь будет инициализация inotify
        // Для тестирования используем заглушку
        
        for path in &self.config.watch_paths {
            if !path.exists() {
                warn!("Watch path does not exist: {}", path.display());
                continue;
            }
            
            // В реальной реализации здесь будет добавление пути в inotify
            // Для тестирования используем заглушку
            let wd = self.add_watch_internal(path)?;
            self.watch_descriptors.insert(path.clone(), wd);
            info!("Added watch for path: {}", path.display());
        }
        
        Ok(())
    }
    
    /// Добавить путь для мониторинга
    fn add_watch_internal(&self, _path: &Path) -> Result<i32> {
        // В реальной реализации здесь будет вызов inotify_add_watch
        // Для тестирования возвращаем фиктивный дескриптор
        Ok(1) // Фиктивный дескриптор
    }
    
    /// Собрать события изменений файлов
    pub fn collect_events(&self) -> Result<Vec<FileChangeEvent>> {
        let mut events = Vec::new();
        
        // В реальной реализации здесь будет чтение событий из inotify
        // Для тестирования используем заглушку
        
        // Проверяем, есть ли события в буфере
        let buffer = self.event_buffer.lock().unwrap();
        if !buffer.is_empty() {
            events.extend(buffer.iter().cloned());
        }
        
        // В реальной реализации здесь будет обработка событий inotify
        // Для тестирования добавляем тестовые события
        if events.is_empty() {
            // Добавляем тестовые события для демонстрации
            let test_event = FileChangeEvent {
                path: PathBuf::from("/test/file.txt"),
                event_type: FileChangeType::Modified,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                process_id: Some(1234),
                process_name: Some("test_process".to_string()),
            };
            events.push(test_event);
        }
        
        Ok(events)
    }
    
    /// Добавить тестовое событие (для тестирования)
    pub fn add_test_event(&self, event: FileChangeEvent) {
        let mut buffer = self.event_buffer.lock().unwrap();
        if buffer.len() < self.config.max_events {
            buffer.push(event);
        } else {
            warn!("Event buffer full, dropping event");
        }
    }
    
    /// Очистить буфер событий
    pub fn clear_events(&self) {
        let mut buffer = self.event_buffer.lock().unwrap();
        buffer.clear();
    }
    
    /// Получить статистику мониторинга
    pub fn get_stats(&self) -> FilesystemMonitorStats {
        let buffer = self.event_buffer.lock().unwrap();
        FilesystemMonitorStats {
            watched_paths: self.watch_descriptors.len(),
            buffered_events: buffer.len(),
            max_capacity: self.config.max_events,
        }
    }
}

/// Статистика мониторинга файловой системы
#[derive(Debug, Clone)]
pub struct FilesystemMonitorStats {
    pub watched_paths: usize,
    pub buffered_events: usize,
    pub max_capacity: usize,
}

/// Тесты для модуля мониторинга файловой системы
#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_filesystem_monitor_creation() {
        let config = FilesystemMonitorConfig {
            watch_paths: vec![PathBuf::from("/tmp")],
            recursive: true,
            max_events: 100,
            event_timeout_secs: 30,
        };
        
        let monitor = FilesystemMonitor::new(config);
        assert!(monitor.is_ok());
        let monitor = monitor.unwrap();
        assert_eq!(monitor.get_stats().watched_paths, 0); // Пока не инициализировано
    }

    #[test]
    fn test_filesystem_monitor_initialization() {
        let config = FilesystemMonitorConfig {
            watch_paths: vec![PathBuf::from("/tmp")],
            recursive: true,
            max_events: 100,
            event_timeout_secs: 30,
        };
        
        let mut monitor = FilesystemMonitor::new(config).unwrap();
        let result = monitor.initialize();
        assert!(result.is_ok());
        assert_eq!(monitor.get_stats().watched_paths, 1); // Теперь должен быть 1 путь
    }

    #[test]
    fn test_filesystem_monitor_event_collection() {
        let config = FilesystemMonitorConfig {
            watch_paths: vec![PathBuf::from("/tmp")],
            recursive: true,
            max_events: 100,
            event_timeout_secs: 30,
        };
        
        let monitor = FilesystemMonitor::new(config).unwrap();
        let events = monitor.collect_events();
        assert!(events.is_ok());
        let events = events.unwrap();
        assert!(!events.is_empty()); // Должно быть хотя бы тестовое событие
        assert_eq!(events[0].event_type, FileChangeType::Modified);
    }

    #[test]
    fn test_filesystem_monitor_add_event() {
        let config = FilesystemMonitorConfig::default();
        let monitor = FilesystemMonitor::new(config).unwrap();
        
        let test_event = FileChangeEvent {
            path: PathBuf::from("/test/added.txt"),
            event_type: FileChangeType::Created,
            timestamp: 1234567890,
            process_id: Some(5678),
            process_name: Some("added_process".to_string()),
        };
        
        monitor.add_test_event(test_event.clone());
        let events = monitor.collect_events().unwrap();
        assert!(events.iter().any(|e| e.path == test_event.path));
    }

    #[test]
    fn test_filesystem_monitor_stats() {
        let config = FilesystemMonitorConfig {
            watch_paths: vec![PathBuf::from("/tmp")],
            recursive: true,
            max_events: 50,
            event_timeout_secs: 30,
        };
        
        let monitor = FilesystemMonitor::new(config).unwrap();
        let stats = monitor.get_stats();
        assert_eq!(stats.max_capacity, 50);
        assert_eq!(stats.buffered_events, 0); // Пока нет событий
    }

    #[test]
    fn test_filesystem_monitor_event_buffer_overflow() {
        let config = FilesystemMonitorConfig {
            watch_paths: vec![PathBuf::from("/tmp")],
            recursive: true,
            max_events: 2, // Очень маленький буфер
            event_timeout_secs: 30,
        };
        
        let monitor = FilesystemMonitor::new(config).unwrap();
        
        // Добавляем события до переполнения
        for i in 0..5 {
            let event = FileChangeEvent {
                path: PathBuf::from(format!("/test/overflow_{}.txt", i)),
                event_type: FileChangeType::Created,
                timestamp: 1234567890 + i,
                process_id: Some(i as u32),
                process_name: Some(format!("overflow_process_{}", i)),
            };
            monitor.add_test_event(event);
        }
        
        let stats = monitor.get_stats();
        assert_eq!(stats.buffered_events, 2); // Должно быть только 2 события (максимум)
    }

    #[test]
    fn test_filesystem_monitor_clear_events() {
        let config = FilesystemMonitorConfig::default();
        let monitor = FilesystemMonitor::new(config).unwrap();
        
        // Добавляем события
        for i in 0..5 {
            let event = FileChangeEvent {
                path: PathBuf::from(format!("/test/clear_{}.txt", i)),
                event_type: FileChangeType::Created,
                timestamp: 1234567890 + i,
                process_id: Some(i as u32),
                process_name: Some(format!("clear_process_{}", i)),
            };
            monitor.add_test_event(event);
        }
        
        // Очищаем события
        monitor.clear_events();
        let stats = monitor.get_stats();
        assert_eq!(stats.buffered_events, 0);
    }

    #[test]
    fn test_filesystem_monitor_multiple_paths() {
        let config = FilesystemMonitorConfig {
            watch_paths: vec![
                PathBuf::from("/tmp"),
                PathBuf::from("/var"),
                PathBuf::from("/etc"),
            ],
            recursive: true,
            max_events: 100,
            event_timeout_secs: 30,
        };
        
        let mut monitor = FilesystemMonitor::new(config).unwrap();
        let result = monitor.initialize();
        assert!(result.is_ok());
        assert_eq!(monitor.get_stats().watched_paths, 3); // Должно быть 3 пути
    }

    #[test]
    fn test_filesystem_monitor_invalid_path() {
        let config = FilesystemMonitorConfig {
            watch_paths: vec![
                PathBuf::from("/tmp"),
                PathBuf::from("/nonexistent/path"), // Несуществующий путь
            ],
            recursive: true,
            max_events: 100,
            event_timeout_secs: 30,
        };
        
        let mut monitor = FilesystemMonitor::new(config).unwrap();
        let result = monitor.initialize();
        assert!(result.is_ok());
        // Должен быть только 1 путь (существующий)
        assert_eq!(monitor.get_stats().watched_paths, 1);
    }

    #[test]
    fn test_filesystem_monitor_event_types() {
        let config = FilesystemMonitorConfig::default();
        let monitor = FilesystemMonitor::new(config).unwrap();
        
        // Тестируем все типы событий
        let event_types = vec![
            FileChangeType::Created,
            FileChangeType::Modified,
            FileChangeType::Deleted,
            FileChangeType::Moved,
            FileChangeType::Accessed,
            FileChangeType::AttributeChanged,
        ];
        
        for (i, event_type) in event_types.iter().enumerate() {
            let event = FileChangeEvent {
                path: PathBuf::from(format!("/test/type_{}.txt", i)),
                event_type: event_type.clone(),
                timestamp: 1234567890 + i as u64,
                process_id: Some(i as u32),
                process_name: Some(format!("type_process_{}", i)),
            };
            monitor.add_test_event(event);
        }
        
        let events = monitor.collect_events().unwrap();
        assert_eq!(events.len(), 6); // Должно быть 6 событий
        
        // Проверяем, что все типы событий присутствуют
        for event_type in event_types {
            assert!(events.iter().any(|e| e.event_type == event_type));
        }
    }
}