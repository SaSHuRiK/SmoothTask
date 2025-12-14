//! Dynamic Priority Scaling — автоматическое масштабирование приоритетов на основе нагрузки системы.
//!
//! Этот модуль предоставляет функциональность для динамического масштабирования приоритетов
//! процессов на основе текущей нагрузки системы. Когда система перегружена, приоритеты
//! могут быть временно понижены для менее критичных процессов, чтобы освободить ресурсы
//! для более важных задач.

use crate::config::config_struct::Config;
use crate::logging::snapshots::{GlobalMetrics, Snapshot};
use crate::policy::classes::PriorityClass;
use std::collections::HashMap;

/// Информация о текущей нагрузке системы для динамического масштабирования.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SystemLoadInfo {
    /// Текущая нагрузка CPU (1-минутное среднее).
    pub cpu_load_1min: f64,
    
    /// Текущая нагрузка CPU (5-минутное среднее).
    pub cpu_load_5min: f64,
    
    /// Текущая нагрузка CPU (15-минутное среднее).
    pub cpu_load_15min: f64,
    
    /// Давление CPU (PSI some avg10).
    pub psi_cpu_some_avg10: Option<f64>,
    
    /// Давление I/O (PSI some avg10).
    pub psi_io_some_avg10: Option<f64>,
    
    /// Давление памяти (PSI some avg10).
    pub psi_mem_some_avg10: Option<f64>,
    
    /// Общее использование памяти (доля от общего объема).
    pub memory_usage_ratio: f64,
    
    /// Уровень нагрузки системы (0.0 - 1.0, где 1.0 - максимальная нагрузка).
    pub load_level: f64,
}

impl SystemLoadInfo {
    /// Создать информацию о нагрузке из глобальных метрик.
    pub fn from_global_metrics(global: &GlobalMetrics) -> Self {
        let cpu_load_1min = global.load_avg_one;
        let cpu_load_5min = global.load_avg_five;
        let cpu_load_15min = global.load_avg_fifteen;
        
        let psi_cpu_some_avg10 = global.psi_cpu_some_avg10;
        let psi_io_some_avg10 = global.psi_io_some_avg10;
        let psi_mem_some_avg10 = global.psi_mem_some_avg10;
        
        // Рассчитываем использование памяти
        let memory_usage_ratio = if global.mem_total_kb > 0 {
            let used_memory = global.mem_total_kb - global.mem_available_kb;
            used_memory as f64 / global.mem_total_kb as f64
        } else {
            0.0
        };
        
        // Рассчитываем общий уровень нагрузки (0.0 - 1.0)
        let load_level = Self::calculate_load_level(
            cpu_load_1min,
            psi_cpu_some_avg10,
            psi_io_some_avg10,
            psi_mem_some_avg10,
            memory_usage_ratio,
        );
        
        Self {
            cpu_load_1min,
            cpu_load_5min,
            cpu_load_15min,
            psi_cpu_some_avg10,
            psi_io_some_avg10,
            psi_mem_some_avg10,
            memory_usage_ratio,
            load_level,
        }
    }
    
    /// Рассчитать общий уровень нагрузки системы (0.0 - 1.0).
    fn calculate_load_level(
        cpu_load_1min: f64,
        psi_cpu_some_avg10: Option<f64>,
        psi_io_some_avg10: Option<f64>,
        psi_mem_some_avg10: Option<f64>,
        memory_usage_ratio: f64,
    ) -> f64 {
        // Нормализуем CPU нагрузку (1.0 = 100% загрузка на 1 ядро, 2.0 = 100% на 2 ядра и т.д.)
        let normalized_cpu_load = cpu_load_1min / num_cpus::get() as f64;
        
        // Нормализуем PSI значения (0.0 - 1.0)
        let psi_cpu = psi_cpu_some_avg10.unwrap_or(0.0).min(1.0);
        let psi_io = psi_io_some_avg10.unwrap_or(0.0).min(1.0);
        let psi_mem = psi_mem_some_avg10.unwrap_or(0.0).min(1.0);
        
        // Нормализуем использование памяти (0.0 - 1.0)
        let normalized_memory = memory_usage_ratio.min(1.0);
        
        // Рассчитываем общий уровень нагрузки как взвешенную сумму
        // CPU нагрузка имеет наибольший вес (40%)
        // PSI показатели имеют средний вес (20% каждый)
        // Память имеет меньший вес (20%)
        let load_level = (
            normalized_cpu_load * 0.4 +
            psi_cpu * 0.2 +
            psi_io * 0.2 +
            psi_mem * 0.2 +
            normalized_memory * 0.2
        ).min(1.0).max(0.0);
        
        load_level
    }
    
    /// Определить уровень нагрузки системы.
    pub fn get_load_category(&self) -> SystemLoadCategory {
        if self.load_level >= 0.8 {
            SystemLoadCategory::High
        } else if self.load_level >= 0.6 {
            SystemLoadCategory::Medium
        } else if self.load_level >= 0.4 {
            SystemLoadCategory::Normal
        } else {
            SystemLoadCategory::Low
        }
    }
}

/// Категория нагрузки системы.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemLoadCategory {
    /// Низкая нагрузка - система практически не загружена.
    Low,
    /// Нормальная нагрузка - система работает в штатном режиме.
    Normal,
    /// Средняя нагрузка - система начинает испытывать нагрузку.
    Medium,
    /// Высокая нагрузка - система перегружена, требуется масштабирование.
    High,
}

/// Динамический масштабировщик приоритетов.
#[derive(Debug, Clone)]
pub struct DynamicPriorityScaler {
    config: Config,
}

impl DynamicPriorityScaler {
    /// Создать новый динамический масштабировщик.
    pub fn new(config: Config) -> Self {
        Self { config }
    }
    
    /// Применить динамическое масштабирование к приоритетам AppGroup.
    ///
    /// # Аргументы
    ///
    /// * `snapshot` - текущий снапшот системы
    /// * `base_priorities` - базовые приоритеты, определенные политикой
    ///
    /// # Возвращает
    ///
    /// Маппинг `app_group_id -> PriorityClass` с динамически скорректированными приоритетами.
    pub fn scale_priorities(
        &self,
        snapshot: &Snapshot,
        base_priorities: &HashMap<String, PriorityClass>,
    ) -> HashMap<String, PriorityClass> {
        let load_info = SystemLoadInfo::from_global_metrics(&snapshot.global);
        let load_category = load_info.get_load_category();
        
        // Если нагрузка низкая или нормальная, возвращаем базовые приоритеты без изменений
        if load_category == SystemLoadCategory::Low || load_category == SystemLoadCategory::Normal {
            return base_priorities.clone();
        }
        
        // Для средней и высокой нагрузки применяем масштабирование
        let mut scaled_priorities = HashMap::new();
        
        for (app_group_id, base_priority) in base_priorities {
            let scaled_priority = self.scale_priority(*base_priority, load_category);
            scaled_priorities.insert(app_group_id.clone(), scaled_priority);
        }
        
        scaled_priorities
    }
    
    /// Масштабировать приоритет на основе уровня нагрузки.
    ///
    /// # Аргументы
    ///
    /// * `base_priority` - базовый приоритет
    /// * `load_category` - текущая категория нагрузки
    ///
    /// # Возвращает
    ///
    /// Скорректированный приоритет.
    fn scale_priority(&self, base_priority: PriorityClass, load_category: SystemLoadCategory) -> PriorityClass {
        match load_category {
            SystemLoadCategory::Low | SystemLoadCategory::Normal => base_priority,
            SystemLoadCategory::Medium => {
                // При средней нагрузке понижаем приоритеты фоновых процессов
                match base_priority {
                    PriorityClass::CritInteractive => PriorityClass::CritInteractive, // Не понижаем
                    PriorityClass::Interactive => PriorityClass::Interactive, // Не понижаем
                    PriorityClass::Normal => PriorityClass::Background, // Понижаем нормальные до фоновых
                    PriorityClass::Background => PriorityClass::Idle, // Понижаем фоновые до idle
                    PriorityClass::Idle => PriorityClass::Idle, // Не понижаем
                }
            }
            SystemLoadCategory::High => {
                // При высокой нагрузке более агрессивно понижаем приоритеты
                match base_priority {
                    PriorityClass::CritInteractive => PriorityClass::CritInteractive, // Не понижаем
                    PriorityClass::Interactive => PriorityClass::Normal, // Понижаем интерактивные до нормальных
                    PriorityClass::Normal => PriorityClass::Background, // Понижаем нормальные до фоновых
                    PriorityClass::Background => PriorityClass::Idle, // Понижаем фоновые до idle
                    PriorityClass::Idle => PriorityClass::Idle, // Не понижаем
                }
            }
        }
    }
    
    /// Проверить, требуется ли динамическое масштабирование.
    ///
    /// # Аргументы
    ///
    /// * `load_info` - информация о текущей нагрузке
    ///
    /// # Возвращает
    ///
    /// `true`, если требуется масштабирование, `false` в противном случае.
    pub fn should_scale(&self, load_info: &SystemLoadInfo) -> bool {
        let load_category = load_info.get_load_category();
        load_category == SystemLoadCategory::Medium || load_category == SystemLoadCategory::High
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::snapshots::{GlobalMetrics, ResponsivenessMetrics};
    use std::collections::HashMap;

    fn create_test_global_metrics() -> GlobalMetrics {
        GlobalMetrics {
            cpu_user: 0.25,
            cpu_system: 0.15,
            cpu_idle: 0.55,
            cpu_iowait: 0.05,
            mem_total_kb: 16_384_256,
            mem_used_kb: 8_000_000,
            mem_available_kb: 8_384_256,
            swap_total_kb: 8_192_000,
            swap_used_kb: 1_000_000,
            load_avg_one: 1.5,
            load_avg_five: 1.2,
            load_avg_fifteen: 1.0,
            psi_cpu_some_avg10: Some(0.1),
            psi_cpu_some_avg60: Some(0.15),
            psi_io_some_avg10: Some(0.2),
            psi_mem_some_avg10: Some(0.05),
            psi_mem_full_avg10: None,
            user_active: true,
            time_since_last_input_ms: Some(5000),
        }
    }

    #[test]
    fn test_system_load_info_creation() {
        let global = create_test_global_metrics();
        let load_info = SystemLoadInfo::from_global_metrics(&global);
        
        assert_eq!(load_info.cpu_load_1min, 1.5);
        assert_eq!(load_info.cpu_load_5min, 1.2);
        assert_eq!(load_info.cpu_load_15min, 1.0);
        assert_eq!(load_info.psi_cpu_some_avg10, Some(0.1));
        assert_eq!(load_info.psi_io_some_avg10, Some(0.2));
        assert_eq!(load_info.psi_mem_some_avg10, Some(0.05));
        
        // Проверяем, что уровень нагрузки рассчитан
        assert!(load_info.load_level >= 0.0);
        assert!(load_info.load_level <= 1.0);
    }

    #[test]
    fn test_load_level_calculation() {
        // Тест с низкой нагрузкой
        let mut global = create_test_global_metrics();
        global.load_avg_one = 0.5; // Низкая CPU нагрузка
        global.psi_cpu_some_avg10 = Some(0.05);
        global.psi_io_some_avg10 = Some(0.05);
        global.psi_mem_some_avg10 = Some(0.05);
        
        let load_info = SystemLoadInfo::from_global_metrics(&global);
        assert!(load_info.load_level < 0.4);
        assert_eq!(load_info.get_load_category(), SystemLoadCategory::Low);
        
        // Тест с высокой нагрузкой
        let mut global = create_test_global_metrics();
        global.load_avg_one = 4.0; // Высокая CPU нагрузка (на 4-ядерной системе)
        global.psi_cpu_some_avg10 = Some(0.8);
        global.psi_io_some_avg10 = Some(0.7);
        global.psi_mem_some_avg10 = Some(0.6);
        
        let load_info = SystemLoadInfo::from_global_metrics(&global);
        assert!(load_info.load_level >= 0.8);
        assert_eq!(load_info.get_load_category(), SystemLoadCategory::High);
    }

    #[test]
    fn test_priority_scaling_medium_load() {
        let config = Config::default();
        let scaler = DynamicPriorityScaler::new(config);
        
        // Создаем базовые приоритеты
        let mut base_priorities = HashMap::new();
        base_priorities.insert("app1".to_string(), PriorityClass::CritInteractive);
        base_priorities.insert("app2".to_string(), PriorityClass::Interactive);
        base_priorities.insert("app3".to_string(), PriorityClass::Normal);
        base_priorities.insert("app4".to_string(), PriorityClass::Background);
        base_priorities.insert("app5".to_string(), PriorityClass::Idle);
        
        // Создаем снапшот с средней нагрузкой
        let mut global = create_test_global_metrics();
        global.load_avg_one = 3.0; // Средняя нагрузка на 4-ядерной системе
        
        let snapshot = Snapshot {
            snapshot_id: 1,
            timestamp: chrono::Utc::now(),
            global,
            processes: vec![],
            app_groups: vec![],
            responsiveness: ResponsivenessMetrics::default(),
        };
        
        let scaled_priorities = scaler.scale_priorities(&snapshot, &base_priorities);
        
        // Проверяем масштабирование при средней нагрузке
        assert_eq!(scaled_priorities.get("app1").unwrap(), &PriorityClass::CritInteractive); // Не изменено
        assert_eq!(scaled_priorities.get("app2").unwrap(), &PriorityClass::Interactive); // Не изменено
        assert_eq!(scaled_priorities.get("app3").unwrap(), &PriorityClass::Background); // Понижено
        assert_eq!(scaled_priorities.get("app4").unwrap(), &PriorityClass::Idle); // Понижено
        assert_eq!(scaled_priorities.get("app5").unwrap(), &PriorityClass::Idle); // Не изменено
    }

    #[test]
    fn test_priority_scaling_high_load() {
        let config = Config::default();
        let scaler = DynamicPriorityScaler::new(config);
        
        // Создаем базовые приоритеты
        let mut base_priorities = HashMap::new();
        base_priorities.insert("app1".to_string(), PriorityClass::CritInteractive);
        base_priorities.insert("app2".to_string(), PriorityClass::Interactive);
        base_priorities.insert("app3".to_string(), PriorityClass::Normal);
        base_priorities.insert("app4".to_string(), PriorityClass::Background);
        base_priorities.insert("app5".to_string(), PriorityClass::Idle);
        
        // Создаем снапшот с высокой нагрузкой
        let mut global = create_test_global_metrics();
        global.load_avg_one = 5.0; // Высокая нагрузка на 4-ядерной системе
        global.psi_cpu_some_avg10 = Some(0.9);
        
        let snapshot = Snapshot {
            snapshot_id: 1,
            timestamp: chrono::Utc::now(),
            global,
            processes: vec![],
            app_groups: vec![],
            responsiveness: ResponsivenessMetrics::default(),
        };
        
        let scaled_priorities = scaler.scale_priorities(&snapshot, &base_priorities);
        
        // Проверяем масштабирование при высокой нагрузке
        assert_eq!(scaled_priorities.get("app1").unwrap(), &PriorityClass::CritInteractive); // Не изменено
        assert_eq!(scaled_priorities.get("app2").unwrap(), &PriorityClass::Normal); // Понижено
        assert_eq!(scaled_priorities.get("app3").unwrap(), &PriorityClass::Background); // Понижено
        assert_eq!(scaled_priorities.get("app4").unwrap(), &PriorityClass::Idle); // Понижено
        assert_eq!(scaled_priorities.get("app5").unwrap(), &PriorityClass::Idle); // Не изменено
    }

    #[test]
    fn test_no_scaling_low_load() {
        let config = Config::default();
        let scaler = DynamicPriorityScaler::new(config);
        
        // Создаем базовые приоритеты
        let mut base_priorities = HashMap::new();
        base_priorities.insert("app1".to_string(), PriorityClass::CritInteractive);
        base_priorities.insert("app2".to_string(), PriorityClass::Normal);
        base_priorities.insert("app3".to_string(), PriorityClass::Background);
        
        // Создаем снапшот с низкой нагрузкой
        let mut global = create_test_global_metrics();
        global.load_avg_one = 0.5; // Низкая нагрузка
        
        let snapshot = Snapshot {
            snapshot_id: 1,
            timestamp: chrono::Utc::now(),
            global,
            processes: vec![],
            app_groups: vec![],
            responsiveness: ResponsivenessMetrics::default(),
        };
        
        let scaled_priorities = scaler.scale_priorities(&snapshot, &base_priorities);
        
        // Проверяем, что приоритеты не изменились
        assert_eq!(scaled_priorities.get("app1").unwrap(), &PriorityClass::CritInteractive);
        assert_eq!(scaled_priorities.get("app2").unwrap(), &PriorityClass::Normal);
        assert_eq!(scaled_priorities.get("app3").unwrap(), &PriorityClass::Background);
    }

    #[test]
    fn test_should_scale() {
        let config = Config::default();
        let scaler = DynamicPriorityScaler::new(config);
        
        // Тест с низкой нагрузкой - не требуется масштабирование
        let mut global = create_test_global_metrics();
        global.load_avg_one = 0.5;
        let load_info = SystemLoadInfo::from_global_metrics(&global);
        assert!(!scaler.should_scale(&load_info));
        
        // Тест со средней нагрузкой - требуется масштабирование
        let mut global = create_test_global_metrics();
        global.load_avg_one = 3.0;
        let load_info = SystemLoadInfo::from_global_metrics(&global);
        assert!(scaler.should_scale(&load_info));
        
        // Тест с высокой нагрузкой - требуется масштабирование
        let mut global = create_test_global_metrics();
        global.load_avg_one = 5.0;
        let load_info = SystemLoadInfo::from_global_metrics(&global);
        assert!(scaler.should_scale(&load_info));
    }
}
