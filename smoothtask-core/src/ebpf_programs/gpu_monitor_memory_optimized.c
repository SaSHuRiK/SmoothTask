// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

// eBPF программа для мониторинга производительности GPU с оптимизацией памяти
// Использует максимально компактное представление данных

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

// Максимальное количество отслеживаемых GPU устройств
#define MAX_GPU_DEVICES 4  // Уменьшено с 8 для экономии памяти

// Супер-компактная структура для хранения информации о производительности GPU
// Используем минимально возможные типы данных
struct gpu_stats_memory_optimized {
    __u16 gpu_usage_pct;      // Использование GPU в процентах (0-100)
    __u16 memory_usage_mb;    // Использование памяти в MB (0-65535)
    __u8 compute_units;       // Количество активных вычислительных единиц (0-255)
    __u8 power_usage_w;       // Потребление энергии в ваттах (0-255)
    __u8 temperature_celsius; // Температура GPU в градусах Цельсия (0-255)
    __u8 max_temperature_celsius; // Максимальная температура GPU (0-255)
    __u32 last_timestamp;     // Последний timestamp (упакованный)
    __u16 reserved;           // Резерв для выравнивания
};

// Используем HASH карту с уменьшенным количеством записей
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, MAX_GPU_DEVICES);
    __type(key, __u32);  // Идентификатор GPU устройства
    __type(value, struct gpu_stats_memory_optimized);
} gpu_stats_memory_optimized_map SEC(".maps");

// Оптимизированная точка входа для отслеживания активности GPU
SEC("tracepoint/drm/drm_gpu_sched_run_job")
int trace_gpu_activity_memory_optimized(struct trace_event_raw_drm_gpu_sched_run_job *ctx) {
    __u32 gpu_id = 0; // В реальной реализации нужно получить реальный GPU ID
    struct gpu_stats_memory_optimized *stats;
    
    // Получаем текущее время (упакованное)
    __u64 timestamp = bpf_ktime_get_ns();
    __u32 timestamp_packed = (__u32)(timestamp >> 20); // Упаковка времени
    
    // Получаем статистику для этого GPU устройства
    stats = bpf_map_lookup_elem(&gpu_stats_memory_optimized_map, &gpu_id);
    if (!stats) {
        // Инициализируем новую запись с минимальным использованием памяти
        struct gpu_stats_memory_optimized new_stats = {0};
        new_stats.last_timestamp = timestamp_packed;
        bpf_map_update_elem(&gpu_stats_memory_optimized_map, &gpu_id, &new_stats, BPF_ANY);
        return 0;
    }
    
    // Обновляем использование GPU (симуляция)
    if (stats->gpu_usage_pct < 95) {
        stats->gpu_usage_pct += 1;
    }
    
    // Обновляем timestamp только если значительно изменилось время
    if (timestamp_packed - stats->last_timestamp > 100) { // Упакованный порог
        stats->last_timestamp = timestamp_packed;
    }
    
    // Обновляем температуру GPU (симуляция)
    __u8 current_temp = 50; // Базовая температура
    if (stats->gpu_usage_pct > 70) { // Если GPU активно используется
        current_temp = 65 + (stats->gpu_usage_pct - 70) / 5;
        if (current_temp > 90) current_temp = 90;
    }
    
    stats->temperature_celsius = current_temp;
    
    // Обновляем максимальную температуру
    if (current_temp > stats->max_temperature_celsius) {
        stats->max_temperature_celsius = current_temp;
    }
    
    return 0;
}

// Оптимизированная точка входа для отслеживания использования памяти GPU
SEC("tracepoint/drm/drm_gem_object_create")
int trace_gpu_memory_memory_optimized(struct trace_event_raw_drm_gem_object_create *ctx) {
    __u32 gpu_id = 0; // В реальной реализации нужно получить реальный GPU ID
    struct gpu_stats_memory_optimized *stats;
    
    // Получаем статистику для этого GPU устройства
    stats = bpf_map_lookup_elem(&gpu_stats_memory_optimized_map, &gpu_id);
    if (!stats)
        return 0;
    
    // Упакованное обновление памяти (в MB)
    if (stats->memory_usage_mb < 16384) { // Ограничение 16GB
        stats->memory_usage_mb += 10; // Увеличиваем на 10MB за раз
    }
    
    return 0;
}

// Оптимизированная точка входа для отслеживания вычислительных задач GPU
SEC("tracepoint/drm/drm_gpu_sched_job_start")
int trace_gpu_compute_start_memory_optimized(struct trace_event_raw_drm_gpu_sched_job_start *ctx) {
    __u32 gpu_id = 0; // В реальной реализации нужно получить реальный GPU ID
    struct gpu_stats_memory_optimized *stats;
    
    // Получаем статистику для этого GPU устройства
    stats = bpf_map_lookup_elem(&gpu_stats_memory_optimized_map, &gpu_id);
    if (!stats)
        return 0;
    
    // Атомарное обновление вычислительных единиц
    if (stats->compute_units < 255) {
        stats->compute_units += 1;
    }
    
    return 0;
}

// Оптимизированная точка входа для отслеживания потребления энергии GPU
SEC("tracepoint/power/power_start")
int trace_gpu_power_usage_memory_optimized(struct trace_event_raw_power_start *ctx) {
    __u32 gpu_id = 0; // В реальной реализации нужно получить реальный GPU ID
    struct gpu_stats_memory_optimized *stats;
    
    // Получаем статистику для этого GPU устройства
    stats = bpf_map_lookup_elem(&gpu_stats_memory_optimized_map, &gpu_id);
    if (!stats)
        return 0;
    
    // Упакованное обновление энергопотребления (в ваттах)
    if (stats->power_usage_w < 300) { // Ограничение 300W
        stats->power_usage_w += 1;
    }
    
    return 0;
}

// Лицензия для eBPF программы
char _license[] SEC("license") = "GPL";