// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

// eBPF программа для тестирования сбора метрик CPU

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

// Структура для хранения тестовых метрик CPU
struct test_cpu_metrics {
    __u64 user_time;
    __u64 system_time;
    __u64 idle_time;
    __u64 timestamp;
};

// Карта для хранения тестовых метрик CPU
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, struct test_cpu_metrics);
} test_cpu_metrics_map SEC(".maps");

// Точка входа для тестирования
SEC("kprobe/run_local_timer")
int kprobe_run_local_timer(struct pt_regs *ctx)
{
    __u32 key = 0;
    struct test_cpu_metrics *metrics;
    
    // Получаем текущее время
    __u64 timestamp = bpf_ktime_get_ns();
    
    // Получаем доступ к карте метрик
    metrics = bpf_map_lookup_elem(&test_cpu_metrics_map, &key);
    if (!metrics)
        return 0;
    
    // Обновляем тестовые метрики
    metrics->user_time += 100;
    metrics->system_time += 50;
    metrics->idle_time += 200;
    metrics->timestamp = timestamp;
    
    return 0;
}

// Лицензия для eBPF программы
char _license[] SEC("license") = "GPL";