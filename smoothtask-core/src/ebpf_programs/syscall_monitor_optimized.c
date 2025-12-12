// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

// Оптимизированная eBPF программа для мониторинга системных вызовов

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

// Оптимизированная структура для хранения информации о системных вызовах
struct syscall_info {
    __u64 count;
    __u64 timestamp;
};

// Используем более эффективную карту с меньшими накладными расходами
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, struct syscall_info);
} syscall_count_map SEC(".maps");

// Оптимизированная точка входа для отслеживания системных вызовов
// Используем более специфичную точку трассировки для уменьшения нагрузки
SEC("tracepoint/syscalls/sys_enter_execve")
int trace_syscall_entry(struct trace_event_raw_sys_enter *ctx)
{
    __u32 key = 0;
    struct syscall_info *info;
    
    // Быстрый путь: получаем текущее время
    __u64 timestamp = bpf_ktime_get_ns();
    
    // Оптимизированный доступ к карте
    info = bpf_map_lookup_elem(&syscall_count_map, &key);
    if (!info)
        return 0;
    
    // Атомарное увеличение счетчика для минимизации конфликтов
    __sync_fetch_and_add(&info->count, 1);
    info->timestamp = timestamp;
    
    return 0;
}

// Лицензия для eBPF программы
char _license[] SEC("license") = "GPL";