// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

// eBPF программа для тестирования сбора метрик памяти

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

// Структура для хранения тестовых метрик памяти
struct test_memory_metrics {
    __u64 total_memory;
    __u64 used_memory;
    __u64 free_memory;
    __u64 cached_memory;
};

// Карта для хранения тестовых метрик памяти
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, struct test_memory_metrics);
} test_memory_metrics_map SEC(".maps");

// Точка входа для тестирования
SEC("kprobe/run_local_timer")
int kprobe_run_local_timer(struct pt_regs *ctx)
{
    __u32 key = 0;
    struct test_memory_metrics *metrics;
    
    // Получаем доступ к карте метрик
    metrics = bpf_map_lookup_elem(&test_memory_metrics_map, &key);
    if (!metrics)
        return 0;
    
    // Обновляем тестовые метрики памяти
    metrics->total_memory = 8 * 1024 * 1024 * 1024; // 8 GB
    metrics->used_memory = 4 * 1024 * 1024 * 1024;   // 4 GB used
    metrics->free_memory = 2 * 1024 * 1024 * 1024;   // 2 GB free
    metrics->cached_memory = 2 * 1024 * 1024 * 1024; // 2 GB cached
    
    return 0;
}

// Лицензия для eBPF программы
char _license[] SEC("license") = "GPL";