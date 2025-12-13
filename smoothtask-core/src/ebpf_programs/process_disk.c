// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

// eBPF программа для мониторинга дисковой активности на уровне процессов
// Отслеживает операции чтения/записи на диск для каждого процесса

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <linux/fs.h>

// Максимальное количество отслеживаемых процессов
#define MAX_PROCESS_DISK_STATS 4096

// Структура для хранения статистики дисковой активности процесса
struct process_disk_stats {
    __u64 bytes_read;
    __u64 bytes_written;
    __u64 read_operations;
    __u64 write_operations;
    __u64 last_timestamp;
    __u32 pid;
    __u32 tgid;
};

// Карта для хранения статистики дисковой активности по PID
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, MAX_PROCESS_DISK_STATS);
    __type(key, __u32);  // PID процесса
    __type(value, struct process_disk_stats);
} process_disk_stats_map SEC(".maps");

// Карта для хранения общего количества операций ввода-вывода
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, __u64);
} total_io_operations_count_map SEC(".maps");

// Точка входа для отслеживания операций чтения с диска
SEC("tracepoint/block/block_rq_issue")
int trace_process_disk_read(struct trace_event_raw_block_rq_issue *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u32 tgid = bpf_get_current_pid_tgid();
    
    if (pid == 0) {
        return 0; // Пропускаем ядро
    }
    
    // Проверяем, что это операция чтения
    if (ctx->rwbs != 0 && (ctx->rwbs & 1) == 0) {
        return 0; // Не операция чтения
    }
    
    struct process_disk_stats *stats;
    
    // Получаем или создаем статистику для этого PID
    stats = bpf_map_lookup_elem(&process_disk_stats_map, &pid);
    if (!stats) {
        struct process_disk_stats new_stats = {};
        new_stats.pid = pid;
        new_stats.tgid = tgid;
        new_stats.last_timestamp = bpf_ktime_get_ns();
        bpf_map_update_elem(&process_disk_stats_map, &pid, &new_stats, BPF_ANY);
        stats = bpf_map_lookup_elem(&process_disk_stats_map, &pid);
        if (!stats) {
            return 0;
        }
    }
    
    // Обновляем статистику чтения
    stats->bytes_read += ctx->bytes;
    stats->read_operations += 1;
    stats->last_timestamp = bpf_ktime_get_ns();
    
    return 0;
}

// Точка входа для отслеживания операций записи на диск
SEC("tracepoint/block/block_rq_issue")
int trace_process_disk_write(struct trace_event_raw_block_rq_issue *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u32 tgid = bpf_get_current_pid_tgid();
    
    if (pid == 0) {
        return 0; // Пропускаем ядро
    }
    
    // Проверяем, что это операция записи
    if (ctx->rwbs != 2 && (ctx->rwbs & 2) == 0) {
        return 0; // Не операция записи
    }
    
    struct process_disk_stats *stats;
    
    // Получаем или создаем статистику для этого PID
    stats = bpf_map_lookup_elem(&process_disk_stats_map, &pid);
    if (!stats) {
        struct process_disk_stats new_stats = {};
        new_stats.pid = pid;
        new_stats.tgid = tgid;
        new_stats.last_timestamp = bpf_ktime_get_ns();
        bpf_map_update_elem(&process_disk_stats_map, &pid, &new_stats, BPF_ANY);
        stats = bpf_map_lookup_elem(&process_disk_stats_map, &pid);
        if (!stats) {
            return 0;
        }
    }
    
    // Обновляем статистику записи
    stats->bytes_written += ctx->bytes;
    stats->write_operations += 1;
    stats->last_timestamp = bpf_ktime_get_ns();
    
    return 0;
}

// Точка входа для отслеживания общего количества операций ввода-вывода
SEC("tracepoint/block/block_rq_complete")
int trace_total_io_operations(struct trace_event_raw_block_rq_complete *ctx)
{
    __u32 key = 0;
    __u64 *count;
    
    // Увеличиваем общее количество операций ввода-вывода
    count = bpf_map_lookup_elem(&total_io_operations_count_map, &key);
    if (count) {
        __sync_fetch_and_add(count, 1);
    }
    
    return 0;
}

// Лицензия для eBPF программы
char _license[] SEC("license") = "GPL";