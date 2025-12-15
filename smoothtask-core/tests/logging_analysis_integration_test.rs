//! Интеграционные тесты для системы анализа и визуализации логов
//!
//! Эти тесты проверяют:
//! - Полный цикл анализа логов
//! - Расширенные возможности фильтрации и поиска
//! - Визуализацию данных
//! - Интеграцию с системой логирования

use anyhow::Result;
use smoothtask_core::logging::integration::*;
use tempfile::TempDir;
use tokio::runtime::Runtime;

fn create_runtime() -> Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create runtime")
}

#[test]
fn test_complete_log_analysis_cycle() -> Result<()> {
    let runtime = create_runtime();
    let temp_dir = TempDir::new().expect("temp dir");
    let log_dir = temp_dir.path();

    runtime.block_on(async {
        let integration =
            AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
        let metrics_logger = MetricsAsyncLogger::new(integration.clone());

        // Записываем тестовые логи
        metrics_logger
            .log_metrics("ERROR: Critical error in system")
            .await
            .expect("log error");
        metrics_logger
            .log_metrics("WARN: High memory usage detected")
            .await
            .expect("log warning");
        metrics_logger
            .log_metrics("INFO: System started successfully")
            .await
            .expect("log info");
        metrics_logger
            .log_metrics("DEBUG: Processing request")
            .await
            .expect("log debug");
        metrics_logger
            .log_metrics("INFO: Another information message")
            .await
            .expect("log info 2");

        // Выполняем полный цикл анализа
        let analysis = integration
            .enhanced_log_analysis("metrics", None, None, None)
            .await
            .expect("analysis");
        assert!(analysis.total_entries >= 5);
        assert!(analysis.error_count >= 1);
        assert!(analysis.warning_count >= 1);
        assert!(analysis.info_count >= 2);
        assert!(analysis.debug_count >= 1);

        // Визуализируем результаты
        let visualization = integration.log_visualization(&analysis, "detailed");
        assert!(visualization.contains("LOG ANALYSIS REPORT"));
        assert!(visualization.contains("Total Entries:"));

        // Выполняем поиск
        let search_results = integration
            .log_search("metrics", "system", false, 10)
            .await
            .expect("search");
        assert!(search_results.len() >= 2);

        // Выполняем фильтрацию
        let filter_criteria = LogFilterCriteria {
            level: Some("info".to_string()),
            pattern: None,
            case_sensitive: false,
            time_range: None,
        };

        let filtered_results = integration
            .log_filtering("metrics", &filter_criteria, 10)
            .await
            .expect("filter");
        assert!(filtered_results.len() >= 2);

        Ok(())
    })
}

#[test]
fn test_log_analysis_with_filtering() -> Result<()> {
    let runtime = create_runtime();
    let temp_dir = TempDir::new().expect("temp dir");
    let log_dir = temp_dir.path();

    runtime.block_on(async {
        let integration =
            AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
        let metrics_logger = MetricsAsyncLogger::new(integration.clone());

        // Записываем тестовые логи разных уровней
        metrics_logger
            .log_metrics("ERROR: Test error 1")
            .await
            .expect("log error 1");
        metrics_logger
            .log_metrics("ERROR: Test error 2")
            .await
            .expect("log error 2");
        metrics_logger
            .log_metrics("WARN: Test warning")
            .await
            .expect("log warning");
        metrics_logger
            .log_metrics("INFO: Test info")
            .await
            .expect("log info");
        metrics_logger
            .log_metrics("DEBUG: Test debug")
            .await
            .expect("log debug");

        // Тестируем фильтрацию по уровню ERROR
        let error_analysis = integration
            .enhanced_log_analysis("metrics", Some("error"), None, None)
            .await
            .expect("error analysis");
        assert!(error_analysis.filtered_entries >= 2);
        assert!(error_analysis.error_count >= 2);
        assert!(error_analysis.warning_count == 0);

        // Тестируем фильтрацию по уровню WARN
        let warn_analysis = integration
            .enhanced_log_analysis("metrics", Some("warning"), None, None)
            .await
            .expect("warn analysis");
        assert!(warn_analysis.filtered_entries >= 1);
        assert!(warn_analysis.warning_count >= 1);

        // Тестируем фильтрацию по уровню INFO
        let info_analysis = integration
            .enhanced_log_analysis("metrics", Some("info"), None, None)
            .await
            .expect("info analysis");
        assert!(info_analysis.filtered_entries >= 1);
        assert!(info_analysis.info_count >= 1);

        Ok(())
    })
}

#[test]
fn test_log_analysis_with_pattern_search() -> Result<()> {
    let runtime = create_runtime();
    let temp_dir = TempDir::new().expect("temp dir");
    let log_dir = temp_dir.path();

    runtime.block_on(async {
        let integration =
            AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
        let metrics_logger = MetricsAsyncLogger::new(integration.clone());

        // Записываем тестовые логи с разными паттернами
        metrics_logger
            .log_metrics("ERROR: System failure detected")
            .await
            .expect("log 1");
        metrics_logger
            .log_metrics("WARN: System approaching limits")
            .await
            .expect("log 2");
        metrics_logger
            .log_metrics("INFO: System operating normally")
            .await
            .expect("log 3");
        metrics_logger
            .log_metrics("DEBUG: System diagnostics complete")
            .await
            .expect("log 4");

        // Тестируем поиск по паттерну "System"
        let system_analysis = integration
            .enhanced_log_analysis("metrics", None, Some("System"), None)
            .await
            .expect("system analysis");
        assert!(system_analysis.matching_entries >= 4);

        // Тестируем поиск по паттерну "failure"
        let failure_analysis = integration
            .enhanced_log_analysis("metrics", None, Some("failure"), None)
            .await
            .expect("failure analysis");
        assert!(failure_analysis.matching_entries >= 1);

        // Тестируем поиск по паттерну "complete"
        let complete_analysis = integration
            .enhanced_log_analysis("metrics", None, Some("complete"), None)
            .await
            .expect("complete analysis");
        assert!(complete_analysis.matching_entries >= 1);

        Ok(())
    })
}

#[test]
fn test_log_visualization_formats() -> Result<()> {
    let runtime = create_runtime();
    let temp_dir = TempDir::new().expect("temp dir");
    let log_dir = temp_dir.path();

    runtime.block_on(async {
        let integration =
            AsyncLoggingIntegration::new_default(log_dir).expect("create integration");

        // Создаем тестовый результат анализа
        let analysis = LogAnalysisResult {
            log_type: "metrics".to_string(),
            total_entries: 100,
            filtered_entries: 80,
            error_count: 10,
            warning_count: 20,
            info_count: 30,
            debug_count: 20,
            matching_entries: 15,
            analysis_summary: "Test analysis summary".to_string(),
        };

        // Тестируем текстовую визуализацию
        let text_vis = integration.log_visualization(&analysis, "text");
        assert!(text_vis.contains("Log Analysis: metrics"));
        assert!(text_vis.contains("Total Entries: 100"));
        assert!(text_vis.contains("Filtered Entries: 80"));

        // Тестируем простую визуализацию
        let simple_vis = integration.log_visualization(&analysis, "simple");
        assert!(simple_vis.contains("Log Analysis: metrics"));
        assert!(simple_vis.contains("ERROR: 12.5% (10)"));
        assert!(simple_vis.contains("WARN: 25.0% (20)"));

        // Тестируем детальную визуализацию
        let detailed_vis = integration.log_visualization(&analysis, "detailed");
        assert!(detailed_vis.contains("LOG ANALYSIS REPORT: METRICS"));
        assert!(detailed_vis.contains("Total Entries: 100"));
        assert!(detailed_vis.contains("Filtered Entries: 80"));
        assert!(detailed_vis.contains("ERROR: 12.5%"));
        assert!(detailed_vis.contains("WARN: 25.0%"));

        Ok(())
    })
}

#[test]
fn test_log_search_functionality() -> Result<()> {
    let runtime = create_runtime();
    let temp_dir = TempDir::new().expect("temp dir");
    let log_dir = temp_dir.path();

    runtime.block_on(async {
        let integration =
            AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
        let metrics_logger = MetricsAsyncLogger::new(integration.clone());

        // Записываем тестовые логи
        metrics_logger
            .log_metrics("Test message 1: error occurred")
            .await
            .expect("log 1");
        metrics_logger
            .log_metrics("Another test message")
            .await
            .expect("log 2");
        metrics_logger
            .log_metrics("Test message 3: warning")
            .await
            .expect("log 3");
        metrics_logger
            .log_metrics("Non-test message")
            .await
            .expect("log 4");

        // Тестируем поиск без учета регистра
        let results = integration
            .log_search("metrics", "Test", false, 10)
            .await
            .expect("search");
        assert!(results.len() >= 3);

        // Тестируем поиск с учетом регистра
        let case_results = integration
            .log_search("metrics", "test", true, 10)
            .await
            .expect("case search");
        assert!(case_results.len() >= 3);

        // Тестируем поиск с ограничением результатов
        let limited_results = integration
            .log_search("metrics", "Test", false, 2)
            .await
            .expect("limited search");
        assert!(limited_results.len() <= 2);

        // Тестируем поиск по несуществующему паттерну
        let empty_results = integration
            .log_search("metrics", "nonexistent", false, 10)
            .await
            .expect("empty search");
        assert!(empty_results.is_empty());

        Ok(())
    })
}

#[test]
fn test_log_filtering_functionality() -> Result<()> {
    let runtime = create_runtime();
    let temp_dir = TempDir::new().expect("temp dir");
    let log_dir = temp_dir.path();

    runtime.block_on(async {
        let integration =
            AsyncLoggingIntegration::new_default(log_dir).expect("create integration");
        let metrics_logger = MetricsAsyncLogger::new(integration.clone());

        // Записываем тестовые логи
        metrics_logger
            .log_metrics("ERROR: Critical error occurred")
            .await
            .expect("log error");
        metrics_logger
            .log_metrics("WARN: Warning message")
            .await
            .expect("log warning");
        metrics_logger
            .log_metrics("INFO: Information message")
            .await
            .expect("log info");
        metrics_logger
            .log_metrics("DEBUG: Debug message")
            .await
            .expect("log debug");
        metrics_logger
            .log_metrics("ERROR: Another error")
            .await
            .expect("log error 2");

        // Тестируем фильтрацию по уровню ERROR
        let filter_criteria = LogFilterCriteria {
            level: Some("error".to_string()),
            pattern: None,
            case_sensitive: false,
            time_range: None,
        };

        let filtered_results = integration
            .log_filtering("metrics", &filter_criteria, 10)
            .await
            .expect("filter");
        assert!(filtered_results.len() >= 2);
        assert!(filtered_results[0].contains("ERROR"));
        assert!(filtered_results[1].contains("ERROR"));

        // Тестируем фильтрацию по паттерну
        let pattern_criteria = LogFilterCriteria {
            level: None,
            pattern: Some("message".to_string()),
            case_sensitive: false,
            time_range: None,
        };

        let pattern_results = integration
            .log_filtering("metrics", &pattern_criteria, 10)
            .await
            .expect("pattern filter");
        assert!(pattern_results.len() >= 3);

        // Тестируем фильтрацию по уровню и паттерну
        let combined_criteria = LogFilterCriteria {
            level: Some("error".to_string()),
            pattern: Some("Critical".to_string()),
            case_sensitive: false,
            time_range: None,
        };

        let combined_results = integration
            .log_filtering("metrics", &combined_criteria, 10)
            .await
            .expect("combined filter");
        assert!(combined_results.len() >= 1);
        assert!(combined_results[0].contains("ERROR"));
        assert!(combined_results[0].contains("Critical"));

        Ok(())
    })
}

#[test]
fn test_log_analysis_edge_cases() -> Result<()> {
    let runtime = create_runtime();
    let temp_dir = TempDir::new().expect("temp dir");
    let log_dir = temp_dir.path();

    runtime.block_on(async {
        let integration =
            AsyncLoggingIntegration::new_default(log_dir).expect("create integration");

        // Тестируем анализ пустого лога
        let empty_analysis = integration
            .enhanced_log_analysis("metrics", None, None, None)
            .await
            .expect("empty analysis");
        assert_eq!(empty_analysis.total_entries, 0);
        assert_eq!(empty_analysis.filtered_entries, 0);
        assert!(empty_analysis
            .analysis_summary
            .contains("Log file does not exist"));

        // Тестируем визуализацию с нулевыми значениями
        let zero_analysis = LogAnalysisResult {
            log_type: "test".to_string(),
            total_entries: 0,
            filtered_entries: 0,
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            debug_count: 0,
            matching_entries: 0,
            analysis_summary: "Zero analysis".to_string(),
        };

        let zero_vis = integration.log_visualization(&zero_analysis, "detailed");
        assert!(zero_vis.contains("Total Entries: 0"));
        assert!(zero_vis.contains("Filtered Entries: 0"));

        Ok(())
    })
}

#[test]
fn test_log_analysis_multiple_log_types() -> Result<()> {
    let runtime = create_runtime();
    let temp_dir = TempDir::new().expect("temp dir");
    let log_dir = temp_dir.path();

    runtime.block_on(async {
        let integration =
            AsyncLoggingIntegration::new_default(log_dir).expect("create integration");

        // Создаем логгеры для разных типов
        let metrics_logger = MetricsAsyncLogger::new(integration.clone());
        let classify_logger = ClassifyAsyncLogger::new(integration.clone());
        let policy_logger = PolicyAsyncLogger::new(integration);

        // Записываем логи для разных типов
        metrics_logger
            .log_metrics("Metrics log entry")
            .await
            .expect("log metrics");
        classify_logger
            .log_classify("Classify log entry")
            .await
            .expect("log classify");
        policy_logger
            .log_policy("Policy log entry")
            .await
            .expect("log policy");

        // Анализируем разные типы логов
        let metrics_analysis = integration
            .enhanced_log_analysis("metrics", None, None, None)
            .await
            .expect("metrics analysis");
        assert!(metrics_analysis.total_entries >= 1);
        assert_eq!(metrics_analysis.log_type, "metrics");

        let classify_analysis = integration
            .enhanced_log_analysis("classify", None, None, None)
            .await
            .expect("classify analysis");
        assert!(classify_analysis.total_entries >= 1);
        assert_eq!(classify_analysis.log_type, "classify");

        let policy_analysis = integration
            .enhanced_log_analysis("policy", None, None, None)
            .await
            .expect("policy analysis");
        assert!(policy_analysis.total_entries >= 1);
        assert_eq!(policy_analysis.log_type, "policy");

        Ok(())
    })
}
