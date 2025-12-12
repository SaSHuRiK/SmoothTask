// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

// Оптимизированная eBPF программа для мониторинга операций с файловой системой
// Использует более эффективные структуры данных и методы сбора

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

// Оптимизированная структура для хранения информации о файлах
// Используем более компактное представление
struct file_stats_optimized {
    __u32 read_count;          // Количество операций чтения
    __u32 write_count;         // Количество операций записи
    __u32 open_count;          // Количество операций открытия
    __u32 close_count;         // Количество операций закрытия
    __u64 bytes_read;          // Количество прочитанных байт
    __u64 bytes_written;       // Количество записанных байт
};

// Используем PERCPU_ARRAY для минимизации конфликтов
// Это более эффективно чем HASH для частых операций с файлами
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, struct file_stats_optimized);
} file_stats_map SEC(".maps");

// Оптимизированная точка входа для отслеживания операций открытия файлов
SEC("tracepoint/syscalls/sys_enter_open")
int trace_file_open_optimized(struct trace_event_raw_sys_enter *ctx)
{
    __u32 key = 0;
    struct file_stats_optimized *stats;
    
    // Оптимизированный доступ к карте
    stats = bpf_map_lookup_elem(&file_stats_map, &key);
    if (!stats)
        return 0;
    
    // Атомарное увеличение счетчика открытия
    __sync_fetch_and_add(&stats->open_count, 1);
    
    return 0;
}

// Оптимизированная точка входа для отслеживания операций чтения файлов
SEC("tracepoint/syscalls/sys_enter_read")
int trace_file_read_optimized(struct trace_event_raw_sys_enter *ctx)
{
    __u32 key = 0;
    struct file_stats_optimized *stats;
    
    // Оптимизированный доступ к карте
    stats = bpf_map_lookup_elem(&file_stats_map, &key);
    if (!stats)
        return 0;
    
    // Атомарное увеличение счетчика чтения
    __sync_fetch_and_add(&stats->read_count, 1);
    
    // В реальной реализации здесь можно добавить анализ размера чтения
    // Для оптимизации пока пропускаем сложные расчеты
    
    return 0;
}

// Оптимизированная точка входа для отслеживания операций записи в файлы
SEC("tracepoint/syscalls/sys_enter_write")
int trace_file_write_optimized(struct trace_event_raw_sys_enter *ctx)
{
    __u32 key = 0;
    struct file_stats_optimized *stats;
    
    // Оптимизированный доступ к карте
    stats = bpf_map_lookup_elem(&file_stats_map, &key);
    if (!stats)
        return 0;
    
    // Атомарное увеличение счетчика записи
    __sync_fetch_and_add(&stats->write_count, 1);
    
    // В реальной реализации здесь можно добавить анализ размера записи
    // Для оптимизации пока пропускаем сложные расчеты
    
    return 0;
}

// Оптимизированная точка входа для отслеживания операций закрытия файлов
SEC("tracepoint/syscalls/sys_enter_close")
int trace_file_close_optimized(struct trace_event_raw_sys_enter *ctx)
{
    __u32 key = 0;
    struct file_stats_optimized *stats;
    
    // Оптимизированный доступ к карте
    stats = bpf_map_lookup_elem(&file_stats_map, &key);
    if (!stats)
        return 0;
    
    // Атомарное увеличение счетчика закрытия
    __sync_fetch_and_add(&stats->close_count, 1);
    
    return 0;
}

// Лицензия для eBPF программы
char _license[] SEC("license") = "GPL";