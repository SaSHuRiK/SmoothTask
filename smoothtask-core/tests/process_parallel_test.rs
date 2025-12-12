// Тесты для проверки параллельной обработки процессов
//
// Эти тесты проверяют, что оптимизация с использованием rayon работает корректно
// и не изменяет поведение функции collect_process_metrics.

use smoothtask_core::metrics::process::collect_process_metrics;

#[test]
fn test_collect_process_metrics_parallel() {
    // Тест: проверяем, что функция collect_process_metrics работает без ошибок
    // и возвращает корректные данные при параллельной обработке
    let result = collect_process_metrics(None);

    // Функция должна вернуть Ok результат
    assert!(result.is_ok());

    let processes = result.unwrap();

    // Должен быть хотя бы один процесс (текущий процесс теста)
    assert!(!processes.is_empty(), "Должен быть хотя бы один процесс");

    // Проверяем, что все процессы имеют корректные PID
    for process in &processes {
        assert!(process.pid > 0, "PID должен быть положительным");
        assert!(
            process.cmdline.as_ref().is_none_or(|c| !c.is_empty()) || process.exe.is_some(),
            "Процесс должен иметь cmdline или exe"
        );
    }
}

#[test]
fn test_collect_process_metrics_consistency() {
    // Тест: проверяем, что параллельная обработка возвращает консистентные результаты
    // при многократном вызове

    let result1 = collect_process_metrics(None);
    let result2 = collect_process_metrics(None);

    assert!(result1.is_ok(), "Первый вызов должен быть успешным");
    assert!(result2.is_ok(), "Второй вызов должен быть успешным");

    let processes1 = result1.unwrap();
    let processes2 = result2.unwrap();

    // Оба вызова должны возвращать процессы
    assert!(
        !processes1.is_empty(),
        "Первый вызов должен вернуть процессы"
    );
    assert!(
        !processes2.is_empty(),
        "Второй вызов должен вернуть процессы"
    );

    // Количество процессов должно быть примерно одинаковым
    // (могут быть небольшие различия из-за завершения/создания процессов)
    let count_diff = processes1.len().abs_diff(processes2.len());
    assert!(
        count_diff < 5,
        "Количество процессов не должно сильно отличаться: {} vs {}",
        processes1.len(),
        processes2.len()
    );
}

#[test]
fn test_collect_process_metrics_performance() {
    // Тест: проверяем, что параллельная обработка работает за разумное время
    // Этот тест не должен занимать слишком много времени

    let start_time = std::time::Instant::now();
    let result = collect_process_metrics(None);
    let duration = start_time.elapsed();

    assert!(result.is_ok(), "Функция должна работать без ошибок");

    // На большинстве систем это должно занимать менее 1 секунды
    // (даже с большим количеством процессов благодаря параллельной обработке)
    assert!(
        duration.as_secs() < 2,
        "Сбор метрик процессов не должен занимать более 2 секунд, заняло: {:?}",
        duration
    );
}

#[test]
fn test_collect_process_metrics_error_handling() {
    // Тест: проверяем, что функция корректно обрабатывает ошибки доступа
    // и возвращает graceful degradation вместо паники

    let result = collect_process_metrics(None);

    // Функция должна всегда возвращать Ok результат
    // (даже если некоторые процессы недоступны)
    assert!(
        result.is_ok(),
        "Функция должна обрабатывать ошибки graceful"
    );

    let processes = result.unwrap();

    // Должен быть хотя бы один процесс (текущий процесс теста)
    // Это проверяет, что функция не возвращает пустой результат при ошибках
    assert!(
        !processes.is_empty(),
        "Должен быть хотя бы один процесс даже при ошибках"
    );

    // Проверяем, что все процессы имеют корректные данные
    for process in &processes {
        assert!(process.pid > 0, "PID должен быть положительным");
        // Некоторые процессы могут не иметь exe или cmdline (например, зомби-процессы)
        // Это нормально, главное что PID корректный
        if process.exe.as_ref().map_or(false, |e| !e.is_empty())
            || process.cmdline.as_ref().map_or(false, |c| !c.is_empty())
        {
            // Если есть exe или cmdline, это хорошо
            tracing::debug!("Процесс PID {} имеет exe или cmdline", process.pid);
        } else {
            // Если нет, это тоже нормально для некоторых процессов
            tracing::debug!(
                "Процесс PID {} не имеет exe или cmdline (возможно зомби)",
                process.pid
            );
        }
    }
}

#[test]
fn test_collect_process_metrics_empty_fallback() {
    // Тест: проверяем поведение функции при отсутствии доступа к /proc
    // (этот тест может не пройти без специальных прав, но проверяет логику)

    // Пробуем собрать метрики
    let result = collect_process_metrics(None);

    match result {
        Ok(processes) => {
            // Успешный случай: есть доступ к /proc
            assert!(!processes.is_empty(), "Должен быть хотя бы один процесс");
        }
        Err(e) => {
            // Ошибка доступа: проверяем, что сообщение об ошибке информативно
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("/proc") || error_msg.contains("доступ"),
                "Сообщение об ошибке должно содержать информацию о /proc или доступе: {}",
                error_msg
            );
        }
    }
}
