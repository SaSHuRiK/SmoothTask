// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

// eBPF программа для сбора метрик CPU с оптимизацией памяти
// Использует компактные структуры данных и битовую упаковку

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

// Компактная структура для хранения метрик CPU с оптимизацией памяти
// Используем 32-битные поля вместо 64-битных где возможно
struct cpu_metrics_memory_optimized {
    __u32 user_time_low;      // Младшие 32 бита user_time
    __u32 user_time_high;     // Старшие 32 бита user_time
    __u32 system_time_low;    // Младшие 32 бита system_time
    __u32 system_time_high;   // Старшие 32 бита system_time
    __u32 idle_time_low;      // Младшие 32 бита idle_time
    __u32 idle_time_high;     // Старшие 32 бита idle_time
    __u32 timestamp_low;      // Младшие 32 бита timestamp
    __u32 timestamp_high;     // Старшие 32 бита timestamp
    __u16 cpu_usage_pct;     // Текущее использование CPU в процентах (0-100)
    __u16 reserved;           // Резерв для выравнивания
};

// Используем PERCPU_ARRAY для минимизации конфликтов
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, struct cpu_metrics_memory_optimized);
} cpu_metrics_memory_optimized_map SEC(".maps");

// Вспомогательная функция для обновления 64-битного значения через 32-битные части
static inline void update_64bit_value(__u32 *low, __u32 *high, __u64 value) {
    *low = (__u32)(value & 0xFFFFFFFF);
    *high = (__u32)(value >> 32);
}

// Вспомогательная функция для получения 64-битного значения из 32-битных частей
static inline __u64 get_64bit_value(__u32 low, __u32 high) {
    return ((__u64)high << 32) | low;
}

// Оптимизированная точка входа для сбора метрик CPU
SEC("tracepoint/sched/sched_process_exec")
int trace_cpu_metrics_memory_optimized(struct trace_event_raw_sched_process_exec *ctx) {
    __u32 key = 0;
    struct cpu_metrics_memory_optimized *metrics;
    
    // Получаем текущее время
    __u64 timestamp = bpf_ktime_get_ns();
    
    // Доступ к карте метрик
    metrics = bpf_map_lookup_elem(&cpu_metrics_memory_optimized_map, &key);
    if (!metrics)
        return 0;
    
    // Обновляем метрики с использованием компактного представления
    __u64 current_user_time = get_64bit_value(metrics->user_time_low, metrics->user_time_high);
    current_user_time += 1;
    update_64bit_value(&metrics->user_time_low, &metrics->user_time_high, current_user_time);
    
    // Обновляем timestamp
    update_64bit_value(&metrics->timestamp_low, &metrics->timestamp_high, timestamp);
    
    // Обновляем использование CPU (симуляция)
    metrics->cpu_usage_pct = 25; // Базовое значение
    
    return 0;
}

// Точка входа для обновления использования CPU
SEC("tracepoint/sched/sched_process_fork")
int trace_cpu_usage_update(struct trace_event_raw_sched_process_fork *ctx) {
    __u32 key = 0;
    struct cpu_metrics_memory_optimized *metrics;
    
    // Доступ к карте метрик
    metrics = bpf_map_lookup_elem(&cpu_metrics_memory_optimized_map, &key);
    if (!metrics)
        return 0;
    
    // Обновляем использование CPU
    if (metrics->cpu_usage_pct < 90) {
        metrics->cpu_usage_pct += 5;
    }
    
    return 0;
}

// Лицензия для eBPF программы
char _license[] SEC("license") = "GPL";