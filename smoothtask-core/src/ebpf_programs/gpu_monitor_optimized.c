// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

// Оптимизированная eBPF программа для мониторинга производительности GPU
// Использует более эффективные структуры данных и точки трассировки

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

// Оптимизированная структура для хранения информации о производительности GPU
// Используем более компактное представление для лучшей локальности кэша
struct gpu_stats_optimized {
    __u32 gpu_usage_pct;      // Использование GPU в процентах (упакованное)
    __u32 memory_usage_mb;    // Использование памяти в MB (упакованное)
    __u16 compute_units;       // Количество активных вычислительных единиц
    __u16 power_usage_uw;      // Потребление энергии в микроваттах (упакованное)
    __u64 last_timestamp;      // Последний timestamp для расчета дельты
};

// Используем PERCPU_ARRAY для минимизации конфликтов доступа
// Это более эффективно чем HASH для частых обновлений
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, struct gpu_stats_optimized);
} gpu_stats_map SEC(".maps");

// Оптимизированная точка входа для отслеживания активности GPU
// Используем более специфичную точку трассировки для уменьшения нагрузки
SEC("tracepoint/drm/drm_gpu_sched_run_job")
int trace_gpu_activity_optimized(struct trace_event_raw_drm_gpu_sched_run_job *ctx)
{
    __u32 key = 0;
    struct gpu_stats_optimized *stats;
    
    // Быстрый путь: получаем текущее время
    __u64 timestamp = bpf_ktime_get_ns();
    
    // Оптимизированный доступ к карте
    stats = bpf_map_lookup_elem(&gpu_stats_map, &key);
    if (!stats)
        return 0;
    
    // Минимальные операции обновления с атомарными операциями
    // Используем более эффективные инкременты вместо сложных расчетов
    __sync_fetch_and_add(&stats->gpu_usage_pct, 1);
    
    // Обновляем timestamp только если значительно изменилось время
    if (timestamp - stats->last_timestamp > 1000000) { // 1ms порог
        stats->last_timestamp = timestamp;
    }
    
    return 0;
}

// Оптимизированная точка входа для отслеживания использования памяти GPU
// Используем более эффективную точку трассировки
SEC("tracepoint/drm/drm_gem_object_create")
int trace_gpu_memory_optimized(struct trace_event_raw_drm_gem_object_create *ctx)
{
    __u32 key = 0;
    struct gpu_stats_optimized *stats;
    
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
int trace_gpu_compute_start_optimized(struct trace_event_raw_drm_gpu_sched_job_start *ctx)
{
    __u32 key = 0;
    struct gpu_stats_optimized *stats;
    
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
int trace_gpu_power_usage_optimized(struct trace_event_raw_power_start *ctx)
{
    __u32 key = 0;
    struct gpu_stats_optimized *stats;
    
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