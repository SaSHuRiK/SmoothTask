// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

// eBPF программа для мониторинга процесс-специфичных метрик
// Отслеживает системные вызовы, использование ресурсов и производительность процессов

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <linux/sched.h>
#include <linux/fs.h>

// Максимальное количество отслеживаемых процессов
#define MAX_PROCESSES 1024

// Структура для хранения информации о процессе
struct process_info {
    __u32 pid;            // Идентификатор процесса
    __u32 tgid;           // Идентификатор потока
    __u32 ppid;           // Идентификатор родительского процесса
    __u64 cpu_time;       // Время CPU в наносекундах
    __u64 memory_usage;   // Использование памяти в байтах
    __u64 syscall_count;  // Количество системных вызовов
    __u64 io_bytes;       // Количество байт ввода-вывода
    __u64 start_time;     // Время начала процесса
    __u64 last_activity;  // Время последней активности
    char comm[16];        // Имя процесса
};

// Карта для хранения информации о процессах
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, MAX_PROCESSES);
    __type(key, __u32);  // PID процесса
    __type(value, struct process_info);
} process_map SEC(".maps");

// Карта для хранения статистики системных вызовов по процессам
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, MAX_PROCESSES);
    __type(key, __u32);  // PID процесса
    __type(value, __u64); // Количество системных вызовов
} syscall_stats_map SEC(".maps");

// Карта для хранения статистики использования CPU по процессам
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, MAX_PROCESSES);
    __type(key, __u32);  // PID процесса
    __type(value, __u64); // Время CPU в наносекундах
} cpu_stats_map SEC(".maps");

// Точка входа для отслеживания системных вызовов
SEC("tracepoint/raw_syscalls/sys_enter")
int trace_syscall_entry(struct trace_event_raw_sys_enter *ctx) {
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u32 tgid = bpf_get_current_pid_tgid();
    
    // Обновляем статистику системных вызовов
    __u64 *count = bpf_map_lookup_elem(&syscall_stats_map, &pid);
    if (count) {
        __sync_fetch_and_add(count, 1);
    } else {
        __u64 new_count = 1;
        bpf_map_update_elem(&syscall_stats_map, &pid, &new_count, BPF_ANY);
    }
    
    // Обновляем информацию о процессе
    struct process_info proc_info = {};
    proc_info.pid = pid;
    proc_info.tgid = tgid;
    proc_info.syscall_count = 1;
    proc_info.last_activity = bpf_ktime_get_ns();
    
    bpf_get_current_comm(&proc_info.comm, sizeof(proc_info.comm));
    
    bpf_map_update_elem(&process_map, &pid, &proc_info, BPF_ANY);
    
    return 0;
}

// Точка входа для отслеживания завершения системных вызовов
SEC("tracepoint/raw_syscalls/sys_exit")
int trace_syscall_exit(struct trace_event_raw_sys_exit *ctx) {
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    
    // Обновляем время последней активности процесса
    struct process_info *proc_info = bpf_map_lookup_elem(&process_map, &pid);
    if (proc_info) {
        proc_info->last_activity = bpf_ktime_get_ns();
    }
    
    return 0;
}

// Точка входа для отслеживания планировщика
SEC("tracepoint/sched/sched_process_exec")
int trace_process_exec(struct trace_event_raw_sched_process_exec *ctx) {
    __u32 pid = ctx->pid;
    
    // Обновляем информацию о процессе при выполнении
    struct process_info proc_info = {};
    proc_info.pid = pid;
    proc_info.tgid = ctx->pid; // Для exec tgid = pid
    proc_info.start_time = bpf_ktime_get_ns();
    proc_info.last_activity = bpf_ktime_get_ns();
    
    bpf_get_current_comm(&proc_info.comm, sizeof(proc_info.comm));
    
    bpf_map_update_elem(&process_map, &pid, &proc_info, BPF_ANY);
    
    return 0;
}

// Точка входа для отслеживания создания процессов
SEC("tracepoint/sched/sched_process_fork")
int trace_process_fork(struct trace_event_raw_sched_process_fork *ctx) {
    __u32 pid = ctx->child_pid;
    __u32 ppid = ctx->parent_pid;
    
    // Создаем новую запись для дочернего процесса
    struct process_info proc_info = {};
    proc_info.pid = pid;
    proc_info.tgid = pid; // Для нового процесса tgid = pid
    proc_info.ppid = ppid;
    proc_info.start_time = bpf_ktime_get_ns();
    proc_info.last_activity = bpf_ktime_get_ns();
    
    bpf_get_current_comm(&proc_info.comm, sizeof(proc_info.comm));
    
    bpf_map_update_elem(&process_map, &pid, &proc_info, BPF_ANY);
    
    return 0;
}

// Точка входа для отслеживания завершения процессов
SEC("tracepoint/sched/sched_process_exit")
int trace_process_exit(struct trace_event_raw_sched_process_exit *ctx) {
    __u32 pid = ctx->pid;
    
    // Удаляем запись о процессе при завершении
    bpf_map_delete_elem(&process_map, &pid);
    bpf_map_delete_elem(&syscall_stats_map, &pid);
    bpf_map_delete_elem(&cpu_stats_map, &pid);
    
    return 0;
}

// Лицензия для eBPF программы
char _license[] SEC("license") = "GPL";