//! Мониторинг производительности ML-моделей.
//!
//! Этот модуль предоставляет функции для сбора и экспорта метрик
//! производительности ML-моделей в формат Prometheus.

use crate::classify::ml_classifier::MLPerformanceMetrics;
use std::collections::HashMap;

/// Преобразовать метрики производительности ML в формат для Prometheus.
///
/// # Аргументы
///
/// * `metrics` - Метрики производительности ML для преобразования.
/// * `model_name` - Название модели (для лейблов).
///
/// # Возвращаемое значение
///
/// Формат Prometheus для метрик производительности ML.
pub fn ml_metrics_to_prometheus(
    metrics: &MLPerformanceMetrics,
    model_name: &str,
) -> String {
    let mut output = String::new();
    
    // Метрики классификации
    output.push_str(&format!(
        "ml_classifications_total{{model=\"{}\"}} {}\\n",
        model_name, metrics.total_classifications
    ));
    
    output.push_str(&format!(
        "ml_classifications_successful{{model=\"{}\"}} {}\\n",
        model_name, metrics.successful_classifications
    ));
    
    output.push_str(&format!(
        "ml_classifications_errors{{model=\"{}\"}} {}\\n",
        model_name, metrics.classification_errors
    ));
    
    // Метрики времени классификации
    if let Some(avg_time) = metrics.average_classification_time_us() {
        output.push_str(&format!(
            "ml_classification_time_avg_microseconds{{model=\"{}\"}} {}\\n",
            model_name, avg_time
        ));
    }
    
    if let Some(min_time) = metrics.min_classification_time_us {
        output.push_str(&format!(
            "ml_classification_time_min_microseconds{{model=\"{}\"}} {}\\n",
            model_name, min_time
        ));
    }
    
    if let Some(max_time) = metrics.max_classification_time_us {
        output.push_str(&format!(
            "ml_classification_time_max_microseconds{{model=\"{}\"}} {}\\n",
            model_name, max_time
        ));
    }
    
    // Метрики уверенности
    if let Some(avg_confidence) = metrics.average_confidence() {
        output.push_str(&format!(
            "ml_confidence_avg{{model=\"{}\"}} {}\\n",
            model_name, avg_confidence
        ));
    }
    
    output.push_str(&format!(
        "ml_confidence_high{{model=\"{}\"}} {}\\n",
        model_name, metrics.high_confidence_classifications
    ));
    
    output.push_str(&format!(
        "ml_confidence_medium{{model=\"{}\"}} {}\\n",
        model_name, metrics.medium_confidence_classifications
    ));
    
    output.push_str(&format!(
        "ml_confidence_low{{model=\"{}\"}} {}\\n",
        model_name, metrics.low_confidence_classifications
    ));
    
    // Метрики успеха
    if let Some(success_rate) = metrics.success_rate() {
        output.push_str(&format!(
            "ml_success_rate{{model=\"{}\"}} {}\\n",
            model_name, success_rate
        ));
    }
    
    output
}

/// Преобразовать метрики производительности ML для нескольких моделей в формат Prometheus.
///
/// # Аргументы
///
/// * `metrics_map` - Хэш-карта с метриками для нескольких моделей.
///
/// # Возвращаемое значение
///
/// Формат Prometheus для метрик производительности всех ML-моделей.
pub fn ml_metrics_map_to_prometheus(
    metrics_map: &HashMap<String, MLPerformanceMetrics>
) -> String {
    let mut output = String::new();
    
    for (model_name, metrics) in metrics_map {
        output.push_str(&ml_metrics_to_prometheus(metrics, model_name));
    }
    
    output
}

/// Создать сводный отчет по метрикам производительности ML.
///
/// # Аргументы
///
/// * `metrics` - Метрики производительности ML.
/// * `model_name` - Название модели.
///
/// # Возвращаемое значение
///
/// Структурированный отчет с основными метриками.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MLPerformanceReport {
    /// Название модели.
    pub model_name: String,
    
    /// Общее количество классификаций.
    pub total_classifications: u64,
    
    /// Процент успешных классификаций.
    pub success_rate: Option<f64>,
    
    /// Среднее время классификации в микросекундах.
    pub average_classification_time_us: Option<f64>,
    
    /// Средняя уверенность.
    pub average_confidence: Option<f64>,
    
    /// Количество классификаций с высокой уверенностью.
    pub high_confidence_classifications: u64,
    
    /// Количество классификаций со средней уверенностью.
    pub medium_confidence_classifications: u64,
    
    /// Количество классификаций с низкой уверенностью.
    pub low_confidence_classifications: u64,
    
    /// Количество ошибок классификации.
    pub classification_errors: u64,
}

impl MLPerformanceReport {
    /// Создать отчет на основе метрик производительности.
    ///
    /// # Аргументы
    ///
    /// * `metrics` - Метрики производительности ML.
    /// * `model_name` - Название модели.
    ///
    /// # Возвращаемое значение
    ///
    /// Отчет по производительности ML.
    pub fn from_metrics(
        metrics: &MLPerformanceMetrics,
        model_name: impl Into<String>
    ) -> Self {
        Self {
            model_name: model_name.into(),
            total_classifications: metrics.total_classifications,
            success_rate: metrics.success_rate(),
            average_classification_time_us: metrics.average_classification_time_us(),
            average_confidence: metrics.average_confidence(),
            high_confidence_classifications: metrics.high_confidence_classifications,
            medium_confidence_classifications: metrics.medium_confidence_classifications,
            low_confidence_classifications: metrics.low_confidence_classifications,
            classification_errors: metrics.classification_errors,
        }
    }
}

/// Создать отчет по производительности для нескольких моделей.
///
/// # Аргументы
///
/// * `metrics_map` - Хэш-карта с метриками для нескольких моделей.
///
/// # Возвращаемое значение
///
/// Вектор отчетов по производительности для всех моделей.
pub fn create_ml_performance_reports(
    metrics_map: &HashMap<String, MLPerformanceMetrics>
) -> Vec<MLPerformanceReport> {
    metrics_map
        .iter()
        .map(|(model_name, metrics)| 
            MLPerformanceReport::from_metrics(metrics, model_name)
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_metrics() -> MLPerformanceMetrics {
        let mut metrics = MLPerformanceMetrics::new();
        
        // Добавляем успешные классификации
        metrics.record_successful_classification(100, 0.9);  // high confidence
        metrics.record_successful_classification(150, 0.7);  // medium confidence
        metrics.record_successful_classification(200, 0.4);  // low confidence
        
        // Добавляем ошибку
        metrics.record_classification_error();
        
        metrics
    }

    #[test]
    fn test_ml_metrics_to_prometheus() {
        let metrics = create_test_metrics();
        let prometheus_output = ml_metrics_to_prometheus(&metrics, "test_model");
        
        // Проверяем, что вывод содержит ожидаемые метрики
        assert!(prometheus_output.contains("ml_classifications_total{model=\"test_model\"}"));
        assert!(prometheus_output.contains("ml_classifications_successful{model=\"test_model\"}"));
        assert!(prometheus_output.contains("ml_classifications_errors{model=\"test_model\"}"));
        assert!(prometheus_output.contains("ml_confidence_avg{model=\"test_model\"}"));
        assert!(prometheus_output.contains("ml_confidence_high{model=\"test_model\"}"));
        assert!(prometheus_output.contains("ml_confidence_medium{model=\"test_model\"}"));
        assert!(prometheus_output.contains("ml_confidence_low{model=\"test_model\"}"));
        assert!(prometheus_output.contains("ml_success_rate{model=\"test_model\"}"));
    }

    #[test]
    fn test_ml_performance_report() {
        let metrics = create_test_metrics();
        let report = MLPerformanceReport::from_metrics(&metrics, "test_model");
        
        // Проверяем основные поля
        assert_eq!(report.model_name, "test_model");
        assert_eq!(report.total_classifications, 4);
        assert_eq!(report.classification_errors, 1);
        assert_eq!(report.high_confidence_classifications, 1);
        assert_eq!(report.medium_confidence_classifications, 1);
        assert_eq!(report.low_confidence_classifications, 1);
        
        // Проверяем вычисляемые поля
        assert!(report.success_rate.is_some());
        assert!(report.average_classification_time_us.is_some());
        assert!(report.average_confidence.is_some());
        
        if let Some(success_rate) = report.success_rate {
            assert!((success_rate - 0.75).abs() < 0.01);  // 3 из 4 = 75%
        }
        
        if let Some(avg_time) = report.average_classification_time_us {
            assert!((avg_time - 150.0).abs() < 0.01);  // (100 + 150 + 200) / 3 = 150
        }
        
        if let Some(avg_confidence) = report.average_confidence {
            assert!((avg_confidence - 0.6667).abs() < 0.01);  // (0.9 + 0.7 + 0.4) / 3 ≈ 0.6667
        }
    }

    #[test]
    fn test_ml_metrics_map_to_prometheus() {
        let mut metrics_map = HashMap::new();
        
        let metrics1 = create_test_metrics();
        metrics_map.insert("model1".to_string(), metrics1);
        
        let metrics2 = create_test_metrics();
        metrics_map.insert("model2".to_string(), metrics2);
        
        let prometheus_output = ml_metrics_map_to_prometheus(&metrics_map);
        
        // Проверяем, что вывод содержит метрики для обеих моделей
        assert!(prometheus_output.contains("model=\"model1\""));
        assert!(prometheus_output.contains("model=\"model2\""));
    }

    #[test]
    fn test_create_ml_performance_reports() {
        let mut metrics_map = HashMap::new();
        
        let metrics1 = create_test_metrics();
        metrics_map.insert("model1".to_string(), metrics1);
        
        let metrics2 = create_test_metrics();
        metrics_map.insert("model2".to_string(), metrics2);
        
        let reports = create_ml_performance_reports(&metrics_map);
        
        // Проверяем, что созданы отчеты для обеих моделей
        assert_eq!(reports.len(), 2);
        
        // Проверяем, что отчеты содержат правильные названия моделей
        let model_names: Vec<_> = reports.iter().map(|r| r.model_name.as_str()).collect();
        assert!(model_names.contains(&"model1"));
        assert!(model_names.contains(&"model2"));
    }

    #[test]
    fn test_empty_metrics() {
        let metrics = MLPerformanceMetrics::new();
        let prometheus_output = ml_metrics_to_prometheus(&metrics, "empty_model");
        
        // Проверяем, что вывод содержит нулевые метрики
        assert!(prometheus_output.contains("ml_classifications_total{model=\"empty_model\"} 0"));
        assert!(prometheus_output.contains("ml_classifications_successful{model=\"empty_model\"} 0"));
        assert!(prometheus_output.contains("ml_classifications_errors{model=\"empty_model\"} 0"));
    }

    #[test]
    fn test_serialization() {
        let metrics = create_test_metrics();
        let report = MLPerformanceReport::from_metrics(&metrics, "test_model");
        
        // Проверяем, что отчет можно сериализовать в JSON
        let json_result = serde_json::to_string(&report);
        assert!(json_result.is_ok());
        
        let json_string = json_result.unwrap();
        assert!(json_string.contains("test_model"));
        assert!(json_string.contains("total_classifications"));
    }
}
