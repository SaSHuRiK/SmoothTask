// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

// eBPF программа для мониторинга операций с файловой системой
// Отслеживает операции открытия, чтения, записи и закрытия файлов

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

// Максимальное количество отслеживаемых файлов
#define MAX_FILES 1024

// Структура для хранения информации о файлах
struct file_stats {
    __u64 read_count;
    __u64 write_count;
    __u64 open_count;
    __u64 close_count;
    __u64 bytes_read;
    __u64 bytes_written;
    __u64 last_access_time;
};

// Карта для хранения статистики по файлам
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, MAX_FILES);
    __type(key, __u32);  // Идентификатор файла (упрощенно)
    __type(value, struct file_stats);
} file_stats_map SEC(".maps");

// Карта для хранения общего количества операций с файлами
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, __u64);
} total_file_ops_map SEC(".maps");

// Точка входа для отслеживания операций открытия файлов
SEC("tracepoint/syscalls/sys_enter_open")
int trace_file_open(struct trace_event_raw_sys_enter *ctx)
{
    __u32 key = 0;
    __u64 *count;
    
    // Увеличиваем общее количество операций с файлами
    count = bpf_map_lookup_elem(&total_file_ops_map, &key);
    if (count) {
        __sync_fetch_and_add(count, 1);
    }
    
    // В реальной реализации здесь будет анализ параметров вызова
    // Пока что это заглушка
    
    return 0;
}

// Точка входа для отслеживания операций чтения файлов
SEC("tracepoint/syscalls/sys_enter_read")
int trace_file_read(struct trace_event_raw_sys_enter *ctx)
{
    __u32 key = 0;
    __u64 *count;
    
    // Увеличиваем общее количество операций с файлами
    count = bpf_map_lookup_elem(&total_file_ops_map, &key);
    if (count) {
        __sync_fetch_and_add(count, 1);
    }
    
    // В реальной реализации здесь будет анализ параметров вызова
    // Пока что это заглушка
    
    return 0;
}

// Точка входа для отслеживания операций записи в файлы
SEC("tracepoint/syscalls/sys_enter_write")
int trace_file_write(struct trace_event_raw_sys_enter *ctx)
{
    __u32 key = 0;
    __u64 *count;
    
    // Увеличиваем общее количество операций с файлами
    count = bpf_map_lookup_elem(&total_file_ops_map, &key);
    if (count) {
        __sync_fetch_and_add(count, 1);
    }
    
    // В реальной реализации здесь будет анализ параметров вызова
    // Пока что это заглушка
    
    return 0;
}

// Точка входа для отслеживания операций закрытия файлов
SEC("tracepoint/syscalls/sys_enter_close")
int trace_file_close(struct trace_event_raw_sys_enter *ctx)
{
    __u32 key = 0;
    __u64 *count;
    
    // Увеличиваем общее количество операций с файлами
    count = bpf_map_lookup_elem(&total_file_ops_map, &key);
    if (count) {
        __sync_fetch_and_add(count, 1);
    }
    
    // В реальной реализации здесь будет анализ параметров вызова
    // Пока что это заглушка
    
    return 0;
}

// Лицензия для eBPF программы
char _license[] SEC("license") = "GPL";