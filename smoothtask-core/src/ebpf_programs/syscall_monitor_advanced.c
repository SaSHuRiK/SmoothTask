// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

// Расширенная eBPF программа для мониторинга системных вызовов
// с поддержкой анализа конкретных системных вызовов и процессов

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

// Максимальное количество отслеживаемых системных вызовов
#define MAX_SYSCALLS 256

// Структура для хранения информации о системных вызовах
struct syscall_stats {
    __u64 count;
    __u64 total_time_ns;
    __u64 last_timestamp;
};

// Карта для хранения статистики по системным вызовам
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, MAX_SYSCALLS);
    __type(key, __u32);  // Номер системного вызова
    __type(value, struct syscall_stats);
} syscall_stats_map SEC(".maps");

// Карта для хранения общего количества системных вызовов
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, __u64);
} total_syscall_count_map SEC(".maps");

// Точка входа для отслеживания начала системных вызовов
SEC("tracepoint/syscalls/sys_enter_*")
int trace_syscall_entry(struct trace_event_raw_sys_enter *ctx)
{
    __u32 key = 0;
    __u64 *count;
    
    // Увеличиваем общее количество системных вызовов
    count = bpf_map_lookup_elem(&total_syscall_count_map, &key);
    if (count) {
        __sync_fetch_and_add(count, 1);
    }
    
    // Получаем номер системного вызова
    __u32 syscall_id = (__u32)ctx->id;
    
    // Обновляем статистику для конкретного системного вызова
    struct syscall_stats *stats = bpf_map_lookup_elem(&syscall_stats_map, &syscall_id);
    if (!stats) {
        // Создаем новую запись, если ее нет
        struct syscall_stats new_stats = {0};
        new_stats.count = 1;
        new_stats.last_timestamp = bpf_ktime_get_ns();
        bpf_map_update_elem(&syscall_stats_map, &syscall_id, &new_stats, BPF_ANY);
    } else {
        // Обновляем существующую запись
        __sync_fetch_and_add(&stats->count, 1);
        stats->last_timestamp = bpf_ktime_get_ns();
    }
    
    return 0;
}

// Точка входа для отслеживания завершения системных вызовов
SEC("tracepoint/syscalls/sys_exit_*")
int trace_syscall_exit(struct trace_event_raw_sys_exit *ctx)
{
    __u32 syscall_id = (__u32)ctx->id;
    struct syscall_stats *stats = bpf_map_lookup_elem(&syscall_stats_map, &syscall_id);
    
    if (stats) {
        __u64 exit_time = bpf_ktime_get_ns();
        __u64 duration = exit_time - stats->last_timestamp;
        __sync_fetch_and_add(&stats->total_time_ns, duration);
    }
    
    return 0;
}

// Лицензия для eBPF программы
char _license[] SEC("license") = "GPL";