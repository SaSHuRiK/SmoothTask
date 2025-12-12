// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

// eBPF программа для мониторинга производительности GPU
// Отслеживает активность GPU и использование ресурсов

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

// Максимальное количество отслеживаемых GPU устройств
#define MAX_GPU_DEVICES 8

// Структура для хранения информации о производительности GPU
struct gpu_stats {
    __u64 gpu_usage_ns;
    __u64 memory_usage_bytes;
    __u64 compute_units_active;
    __u64 last_timestamp;
    __u64 power_usage_uw;
    __u32 temperature_celsius;  // Температура GPU в градусах Цельсия
    __u32 max_temperature_celsius;  // Максимальная температура GPU
};

// Карта для хранения статистики по GPU устройствам
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, MAX_GPU_DEVICES);
    __type(key, __u32);  // Идентификатор GPU устройства
    __type(value, struct gpu_stats);
} gpu_stats_map SEC(".maps");

// Карта для хранения общего времени использования GPU
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, __u64);
} total_gpu_usage_map SEC(".maps");

// Точка входа для отслеживания активности GPU
SEC("tracepoint/drm/drm_gpu_sched_run_job")
int trace_gpu_activity(struct trace_event_raw_drm_gpu_sched_run_job *ctx)
{
    __u32 gpu_id = 0; // В реальной реализации нужно получить реальный GPU ID
    struct gpu_stats *stats;
    
    // Получаем текущее время
    __u64 timestamp = bpf_ktime_get_ns();
    
    // Получаем статистику для этого GPU устройства
    stats = bpf_map_lookup_elem(&gpu_stats_map, &gpu_id);
    if (!stats) {
        // Инициализируем новую запись
        struct gpu_stats new_stats = {0};
        new_stats.last_timestamp = timestamp;
        bpf_map_update_elem(&gpu_stats_map, &gpu_id, &new_stats, BPF_ANY);
        return 0;
    }
    
    // Рассчитываем дельту времени
    __u64 delta = timestamp - stats->last_timestamp;
    stats->last_timestamp = timestamp;
    
    // Увеличиваем использование GPU (в наносекундах)
    __sync_fetch_and_add(&stats->gpu_usage_ns, delta);
    
    // Увеличиваем общее время использования GPU
    __u32 total_key = 0;
    __u64 *total_usage = bpf_map_lookup_elem(&total_gpu_usage_map, &total_key);
    if (total_usage) {
        __sync_fetch_and_add(total_usage, delta);
    }
    
    // Обновляем температуру GPU (симуляция)
    // В реальной реализации нужно получить реальную температуру из ядра
    __u32 current_temp = 50; // Базовая температура
    if (stats->gpu_usage_ns > 1000000000) { // Если GPU активно используется
        current_temp = 65 + (stats->gpu_usage_ns / 1000000000) % 20; // 65-85°C
    }
    
    stats->temperature_celsius = current_temp;
    
    // Обновляем максимальную температуру
    if (current_temp > stats->max_temperature_celsius) {
        stats->max_temperature_celsius = current_temp;
    }
    
    return 0;
}

// Точка входа для отслеживания использования памяти GPU
SEC("tracepoint/drm/drm_gem_object_create")
int trace_gpu_memory(struct trace_event_raw_drm_gem_object_create *ctx)
{
    __u32 gpu_id = 0; // В реальной реализации нужно получить реальный GPU ID
    struct gpu_stats *stats;
    
    // Получаем статистику для этого GPU устройства
    stats = bpf_map_lookup_elem(&gpu_stats_map, &gpu_id);
    if (!stats) {
        // Инициализируем новую запись
        struct gpu_stats new_stats = {0};
        new_stats.last_timestamp = bpf_ktime_get_ns();
        bpf_map_update_elem(&gpu_stats_map, &gpu_id, &new_stats, BPF_ANY);
        return 0;
    }
    
    // Увеличиваем использование памяти GPU
    // В реальной реализации нужно получить реальный размер объекта
    __u64 memory_increase = 4096; // Пример: 4KB увеличение (реально нужно получить из ctx)
    __sync_fetch_and_add(&stats->memory_usage_bytes, memory_increase);
    
    return 0;
}

// Точка входа для отслеживания вычислительных задач GPU
SEC("tracepoint/drm/drm_gpu_sched_job_start")
int trace_gpu_compute_start(struct trace_event_raw_drm_gpu_sched_job_start *ctx)
{
    __u32 gpu_id = 0; // В реальной реализации нужно получить реальный GPU ID
    struct gpu_stats *stats;
    
    // Получаем статистику для этого GPU устройства
    stats = bpf_map_lookup_elem(&gpu_stats_map, &gpu_id);
    if (!stats) {
        // Инициализируем новую запись
        struct gpu_stats new_stats = {0};
        new_stats.last_timestamp = bpf_ktime_get_ns();
        bpf_map_update_elem(&gpu_stats_map, &gpu_id, &new_stats, BPF_ANY);
        return 0;
    }
    
    // Увеличиваем количество активных вычислительных единиц
    __sync_fetch_and_add(&stats->compute_units_active, 1);
    
    return 0;
}

// Точка входа для отслеживания завершения вычислительных задач GPU
SEC("tracepoint/drm/drm_gpu_sched_job_end")
int trace_gpu_compute_end(struct trace_event_raw_drm_gpu_sched_job_end *ctx)
{
    __u32 gpu_id = 0; // В реальной реализации нужно получить реальный GPU ID
    struct gpu_stats *stats;
    
    // Получаем статистику для этого GPU устройства
    stats = bpf_map_lookup_elem(&gpu_stats_map, &gpu_id);
    if (!stats) {
        return 0;
    }
    
    // Уменьшаем количество активных вычислительных единиц
    if (stats->compute_units_active > 0) {
        __sync_fetch_and_sub(&stats->compute_units_active, 1);
    }
    
    return 0;
}

// Точка входа для отслеживания потребления энергии GPU
// Используем общий tracepoint для отслеживания изменений в энергопотреблении
SEC("tracepoint/power/power_start")
int trace_gpu_power_usage(struct trace_event_raw_power_start *ctx)
{
    __u32 gpu_id = 0; // В реальной реализации нужно получить реальный GPU ID
    struct gpu_stats *stats;
    
    // Получаем статистику для этого GPU устройства
    stats = bpf_map_lookup_elem(&gpu_stats_map, &gpu_id);
    if (!stats) {
        // Инициализируем новую запись
        struct gpu_stats new_stats = {0};
        new_stats.last_timestamp = bpf_ktime_get_ns();
        bpf_map_update_elem(&gpu_stats_map, &gpu_id, &new_stats, BPF_ANY);
        return 0;
    }
    
    // Увеличиваем потребление энергии
    // В реальной реализации нужно получить реальное значение энергопотребления
    __u64 power_increase = 1000; // Пример: 1000 микроватт (реально нужно получить из ctx)
    
    // Масштабируем энергопотребление в зависимости от использования GPU
    if (stats->gpu_usage_ns > 0) {
        __u64 usage_factor = stats->gpu_usage_ns / 1000000; // Масштабирующий фактор
        if (usage_factor > 100) usage_factor = 100; // Ограничиваем максимальный фактор
        power_increase = 1000 + (usage_factor * 50); // 1000-6000 микроватт
    }
    
    __sync_fetch_and_add(&stats->power_usage_uw, power_increase);
    
    return 0;
}

// Лицензия для eBPF программы
char _license[] SEC("license") = "GPL";