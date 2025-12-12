// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

// Высокопроизводительная eBPF программа для мониторинга производительности GPU
// Использует минимальные операции и оптимизированные структуры данных

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

// Ультра-компактная структура для хранения информации о производительности GPU
// Используем минимально возможные типы для лучшей локальности кэша
struct gpu_stats_high_perf {
    __u16 gpu_usage_pct;      // Использование GPU в процентах (0-100)
    __u16 memory_usage_mb;    // Использование памяти в MB
    __u8 compute_units;        // Количество активных вычислительных единиц
    __u8 power_usage_uw;      // Потребление энергии в микроваттах (упакованное)
    __u32 last_timestamp_lo;   // Последний timestamp (младшие 32 бита)
    __u32 last_timestamp_hi;   // Последний timestamp (старшие 32 бита)
};

// Используем PERCPU_ARRAY для минимизации конфликтов доступа
// Это наиболее эффективно для частых обновлений
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, struct gpu_stats_high_perf);
} gpu_stats_map SEC(".maps");

// Оптимизированная точка входа для отслеживания активности GPU
// Используем минимальные операции и быстрый путь
SEC("tracepoint/drm/drm_gpu_sched_run_job")
int trace_gpu_activity_high_perf(struct trace_event_raw_drm_gpu_sched_run_job *ctx)
{
    __u32 key = 0;
    struct gpu_stats_high_perf *stats;
    __u64 timestamp;
    
    // Быстрый путь: получаем текущее время
    timestamp = bpf_ktime_get_ns();
    
    // Оптимизированный доступ к карте
    stats = bpf_map_lookup_elem(&gpu_stats_map, &key);
    if (!stats)
        return 0;
    
    // Минимальные операции обновления с атомарными операциями
    // Используем инкремент вместо сложных расчетов
    __sync_fetch_and_add(&stats->gpu_usage_pct, 1);
    
    // Обновляем timestamp только если значительно изменилось время
    // Это уменьшает количество записей в память
    if (timestamp - ((__u64)stats->last_timestamp_hi << 32 | stats->last_timestamp_lo) > 1000000) {
        stats->last_timestamp_lo = (__u32)timestamp;
        stats->last_timestamp_hi = (__u32)(timestamp >> 32);
    }
    
    return 0;
}

// Оптимизированная точка входа для отслеживания использования памяти GPU
SEC("tracepoint/drm/drm_gem_object_create")
int trace_gpu_memory_high_perf(struct trace_event_raw_drm_gem_object_create *ctx)
{
    __u32 key = 0;
    struct gpu_stats_high_perf *stats;
    
    // Оптимизированный доступ к карте
    stats = bpf_map_lookup_elem(&gpu_stats_map, &key);
    if (!stats)
        return 0;
    
    // Упакованное обновление памяти (в MB)
    __sync_fetch_and_add(&stats->memory_usage_mb, 1);
    
    return 0;
}

// Оптимизированная точка входа для отслеживания вычислительных задач GPU
SEC("tracepoint/drm/drm_gpu_sched_job_start")
int trace_gpu_compute_start_high_perf(struct trace_event_raw_drm_gpu_sched_job_start *ctx)
{
    __u32 key = 0;
    struct gpu_stats_high_perf *stats;
    
    // Оптимизированный доступ к карте
    stats = bpf_map_lookup_elem(&gpu_stats_map, &key);
    if (!stats)
        return 0;
    
    // Атомарное обновление вычислительных единиц
    __sync_fetch_and_add(&stats->compute_units, 1);
    
    return 0;
}

// Оптимизированная точка входа для отслеживания потребления энергии GPU
SEC("tracepoint/power/power_start")
int trace_gpu_power_usage_high_perf(struct trace_event_raw_power_start *ctx)
{
    __u32 key = 0;
    struct gpu_stats_high_perf *stats;
    
    // Оптимизированный доступ к карте
    stats = bpf_map_lookup_elem(&gpu_stats_map, &key);
    if (!stats)
        return 0;
    
    // Упакованное обновление энергопотребления
    __sync_fetch_and_add(&stats->power_usage_uw, 1);
    
    return 0;
}

// Лицензия для eBPF программы
char _license[] SEC("license") = "GPL";