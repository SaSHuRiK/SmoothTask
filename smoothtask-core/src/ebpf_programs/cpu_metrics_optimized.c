// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

// Оптимизированная eBPF программа для сбора метрик CPU

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

// Оптимизированная структура для хранения метрик CPU
struct cpu_metrics {
    __u64 user_time;
    __u64 system_time;
    __u64 idle_time;
    __u64 timestamp;
};

// Используем PERCPU_ARRAY для минимизации конфликтов
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, struct cpu_metrics);
} cpu_metrics_map SEC(".maps");

// Оптимизированная точка входа для сбора метрик CPU
// Используем более эффективную точку трассировки
SEC("tracepoint/sched/sched_process_exec")
int trace_cpu_metrics(struct trace_event_raw_sched_process_exec *ctx)
{
    __u32 key = 0;
    struct cpu_metrics *metrics;
    
    // Быстрый путь: получаем текущее время
    __u64 timestamp = bpf_ktime_get_ns();
    
    // Оптимизированный доступ к карте
    metrics = bpf_map_lookup_elem(&cpu_metrics_map, &key);
    if (!metrics)
        return 0;
    
    // Минимальные операции обновления
    __sync_fetch_and_add(&metrics->user_time, 1);
    metrics->timestamp = timestamp;
    
    return 0;
}

// Лицензия для eBPF программы
char _license[] SEC("license") = "GPL";