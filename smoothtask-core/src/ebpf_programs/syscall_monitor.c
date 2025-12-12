// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

// eBPF программа для мониторинга системных вызовов

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

// Структура для хранения информации о системных вызовах
struct syscall_info {
    __u64 count;
    __u64 timestamp;
};

// Карта для хранения счетчика системных вызовов
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, struct syscall_info);
} syscall_count_map SEC(".maps");

// Точка входа для отслеживания системных вызовов
SEC("tracepoint/syscalls/sys_enter_*")
int trace_syscall_entry(struct trace_event_raw_sys_enter *ctx)
{
    __u32 key = 0;
    struct syscall_info *info;
    
    // Получаем текущее время
    __u64 timestamp = bpf_ktime_get_ns();
    
    // Получаем доступ к карте
    info = bpf_map_lookup_elem(&syscall_count_map, &key);
    if (!info)
        return 0;
    
    // Увеличиваем счетчик системных вызовов
    __sync_fetch_and_add(&info->count, 1);
    info->timestamp = timestamp;
    
    return 0;
}

// Лицензия для eBPF программы
char _license[] SEC("license") = "GPL";