//! Бенчмарки для модуля классификации процессов.
//!
//! Эти бенчмарки измеряют производительность:
//! - Сопоставления glob паттернов
//! - Классификации процессов
//! - Кэширования результатов

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use smoothtask_core::classify::rules::PatternDatabase;
use smoothtask_core::logging::snapshots::ProcessRecord;
use std::path::Path;
use tempfile::TempDir;

/// Создает тестовый файл с паттернами
fn create_test_pattern_file(dir: &Path, filename: &str, content: &str) {
    use std::fs;
    use std::io::Write;
    
    let file_path = dir.join(filename);
    let mut file = fs::File::create(file_path).expect("create pattern file");
    file.write_all(content.as_bytes()).expect("write pattern content");
}

/// Бенчмарк для сопоставления glob паттернов
fn glob_pattern_matching_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Glob Pattern Matching");
    
    // Тестовые данные
    let test_cases = vec![
        ("firefox", "firefox"),
        ("firefox-bin", "firefox-*"),
        ("chromium-browser", "*chromium*"),
        ("code-oss", "code-*"),
        ("something-firefox-bin", "*firefox*"),
        ("firefox-esr", "firefox-*-bin"),
        ("firefox-a-bin", "firefox-?-bin"),
        ("long-application-name", "long-*-name"),
    ];
    
    for (text, pattern) in test_cases {
        group.bench_function(format!("match {}/{}", text, pattern), |b| {
            b.iter(|| {
                PatternDatabase::matches_pattern(black_box(text), black_box(pattern))
            })
        });
    }
    
    group.finish();
}

/// Бенчмарк для классификации процессов
fn process_classification_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Process Classification");
    
    // Создаем временную директорию с тестовыми паттернами
    let temp_dir = TempDir::new().expect("temp dir");
    let patterns_dir = temp_dir.path();
    
    // Создаем тестовые паттерны
    create_test_pattern_file(
        patterns_dir,
        "browsers.yml",
        r#"
category: browser
apps:
  - name: firefox
    label: Mozilla Firefox
    exe_patterns: ["firefox", "firefox-*"]
    tags: ["browser", "gui_interactive"]
  - name: chromium
    label: Chromium
    exe_patterns: ["chromium", "chromium-browser", "chrome"]
    tags: ["browser", "gui_interactive"]
  - name: vscode
    label: Visual Studio Code
    exe_patterns: ["code", "code-oss", "vscode"]
    tags: ["ide", "gui_interactive"]
"#,
    );
    
    // Загружаем базу паттернов
    let pattern_db = PatternDatabase::load(patterns_dir).expect("load patterns");
    
    // Тестовые процессы
    let test_processes = vec![
        ProcessRecord {
            pid: 1000,
            ppid: 1,
            uid: 1000,
            gid: 1000,
            exe: Some("firefox".to_string()),
            ..Default::default()
        },
        ProcessRecord {
            pid: 1001,
            ppid: 1,
            uid: 1000,
            gid: 1000,
            exe: Some("firefox-bin".to_string()),
            ..Default::default()
        },
        ProcessRecord {
            pid: 1002,
            ppid: 1,
            uid: 1000,
            gid: 1000,
            exe: Some("chromium-browser".to_string()),
            ..Default::default()
        },
        ProcessRecord {
            pid: 1003,
            ppid: 1,
            uid: 1000,
            gid: 1000,
            exe: Some("code-oss".to_string()),
            ..Default::default()
        },
        ProcessRecord {
            pid: 1004,
            ppid: 1,
            uid: 1000,
            gid: 1000,
            exe: Some("unknown-app".to_string()),
            ..Default::default()
        },
    ];
    
    // Бенчмарк для первого запуска (без кэша)
    group.bench_function("first classification (no cache)", |b| {
        b.iter(|| {
            for process in &test_processes {
                let mut proc = process.clone();
                let db_arc = std::sync::Arc::new(std::sync::Mutex::new(pattern_db.clone()));
                smoothtask_core::classify::rules::classify_process(
                    &mut proc,
                    &db_arc,
                    None,
                    None,
                );
            }
        })
    });
    
    // Бенчмарк для повторного запуска (с кэшем)
    group.bench_function("repeat classification (with cache)", |b| {
        b.iter(|| {
            for process in &test_processes {
                let mut proc = process.clone();
                let db_arc = std::sync::Arc::new(std::sync::Mutex::new(pattern_db.clone()));
                smoothtask_core::classify::rules::classify_process(
                    &mut proc,
                    &db_arc,
                    None,
                    None,
                );
            }
        })
    });
    
    group.finish();
}

/// Бенчмарк для массовой классификации процессов
fn bulk_classification_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Bulk Classification");
    
    // Создаем временную директорию с тестовыми паттернами
    let temp_dir = TempDir::new().expect("temp dir");
    let patterns_dir = temp_dir.path();
    
    // Создаем расширенные тестовые паттерны
    create_test_pattern_file(
        patterns_dir,
        "apps.yml",
        r#"
category: browser
apps:
  - name: firefox
    exe_patterns: ["firefox", "firefox-*"]
    tags: ["browser"]
  - name: chromium
    exe_patterns: ["chromium", "chromium-browser", "chrome"]
    tags: ["browser"]

category: ide
apps:
  - name: vscode
    exe_patterns: ["code", "code-oss", "vscode"]
    tags: ["ide"]
  - name: intellij
    exe_patterns: ["idea", "studio", "intellij"]
    tags: ["ide"]

category: terminal
apps:
  - name: gnome-terminal
    exe_patterns: ["gnome-terminal", "terminal"]
    tags: ["terminal"]
  - name: konsole
    exe_patterns: ["konsole"]
    tags: ["terminal"]
"#,
    );
    
    // Загружаем базу паттернов
    let pattern_db = PatternDatabase::load(patterns_dir).expect("load patterns");
    
    // Создаем большой набор тестовых процессов
    let mut processes = Vec::new();
    for i in 0..100 {
        let exe = match i % 5 {
            0 => "firefox",
            1 => "chromium-browser",
            2 => "code-oss",
            3 => "gnome-terminal",
            4 => "unknown-app",
            _ => "firefox",
        };
        
        processes.push(ProcessRecord {
            pid: 1000 + i,
            ppid: 1,
            uid: 1000,
            gid: 1000,
            exe: Some(exe.to_string()),
            ..Default::default()
        });
    }
    
    // Бенчмарк для массовой классификации
    group.bench_function("bulk classification 100 processes", |b| {
        b.iter(|| {
            for process in &processes {
                let mut proc = process.clone();
                let db_arc = std::sync::Arc::new(std::sync::Mutex::new(pattern_db.clone()));
                smoothtask_core::classify::rules::classify_process(
                    &mut proc,
                    &db_arc,
                    None,
                    None,
                );
            }
        })
    });
    
    group.finish();
}

/// Бенчмарк для кэширования паттернов
fn pattern_caching_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Pattern Caching");
    
    // Создаем временную директорию с тестовыми паттернами
    let temp_dir = TempDir::new().expect("temp dir");
    let patterns_dir = temp_dir.path();
    
    // Создаем тестовые паттерны
    create_test_pattern_file(
        patterns_dir,
        "cache_test.yml",
        r#"
category: test
apps:
  - name: app1
    exe_patterns: ["app1", "app1-*"]
    tags: ["test"]
  - name: app2
    exe_patterns: ["app2", "app2-*"]
    tags: ["test"]
"#,
    );
    
    // Загружаем базу паттернов
    let mut pattern_db = PatternDatabase::load(patterns_dir).expect("load patterns");
    
    // Тестируем кэширование одинаковых запросов
    group.bench_function("cached pattern matching", |b| {
        b.iter(|| {
            // Этот вызов должен использовать кэш после первого выполнения
            let matches = pattern_db.match_process(Some("app1"), None, None);
            black_box(matches);
        })
    });
    
    // Тестируем кэширование разных запросов
    group.bench_function("mixed pattern matching", |b| {
        b.iter(|| {
            let mut db1 = pattern_db.clone();
            let mut db2 = pattern_db.clone();
            let mut db3 = pattern_db.clone();
            let matches1 = db1.match_process(Some("app1"), None, None);
            let matches2 = db2.match_process(Some("app2"), None, None);
            let matches3 = db3.match_process(Some("app1-bin"), None, None);
            black_box((matches1, matches2, matches3));
        })
    });
    
    group.finish();
}

criterion_group!(
    name = classification_benches;
    config = Criterion::default().sample_size(10);
    targets = 
        glob_pattern_matching_benchmark,
        process_classification_benchmark,
        bulk_classification_benchmark,
        pattern_caching_benchmark
);

criterion_main!(classification_benches);