// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2025 SmoothTask Authors */

// eBPF программа для мониторинга производительности приложений
// Отслеживает время выполнения, время ожидания различных ресурсов
// и рассчитывает проценты использования времени

#include "vmlinux.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>

// Максимальное количество отслеживаемых процессов
#define MAX_APPLICATIONS 20480

// Структура для хранения статистики производительности приложения
struct application_performance_stats {
    __u32 pid;                    // Идентификатор процесса
    __u32 tgid;                   // Идентификатор потока
    __u64 execution_time_ns;      // Время выполнения в наносекундах
    __u64 io_wait_time_ns;        // Время ожидания ввода-вывода
    __u64 cpu_wait_time_ns;       // Время ожидания CPU
    __u64 lock_wait_time_ns;      // Время ожидания блокировок
    __u64 network_wait_time_ns;   // Время ожидания сети
    __u64 disk_wait_time_ns;      // Время ожидания диска
    __u64 memory_wait_time_ns;    // Время ожидания памяти
    __u64 gpu_wait_time_ns;       // Время ожидания GPU
    __u64 other_wait_time_ns;     // Время ожидания других ресурсов
    __u64 total_time_ns;          // Общее время выполнения
    __u64 last_update_ns;         // Время последнего обновления
    __u64 cache_misses;           // Количество промахов кэша
    __u64 cache_hits;             // Количество попаданий в кэш
    __u64 branch_misses;          // Количество промахов ветвлений
    __u64 branch_hits;            // Количество успешных ветвлений
    __u64 page_faults;            // Количество page faults
    __u64 context_switches;       // Количество переключений контекста
    __u64 system_calls;           // Количество системных вызовов
    __u64 interrupts;             // Количество прерываний
    __u64 memory_allocations;     // Количество выделений памяти
    __u64 memory_frees;           // Количество освобождений памяти
    char comm[16];                // Имя процесса
};

// Карта для хранения статистики производительности приложений
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, MAX_APPLICATIONS);
    __type(key, __u32);                          // PID как ключ
    __type(value, struct application_performance_stats);
} application_performance_map SEC(".maps");

// Прикрепляемся к точке трассировки sched/sched_process_exec
// для отслеживания запуска новых процессов
SEC("tracepoint/sched/sched_process_exec")
int trace_process_exec(struct trace_event_raw_sched_process_exec *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u32 tgid = bpf_get_current_pid_tgid();
    __u64 current_time = bpf_ktime_get_ns();

    // Инициализируем запись для нового процесса
    struct application_performance_stats stats = {};
    stats.pid = pid;
    stats.tgid = tgid;
    stats.execution_time_ns = 0;
    stats.io_wait_time_ns = 0;
    stats.cpu_wait_time_ns = 0;
    stats.lock_wait_time_ns = 0;
    stats.network_wait_time_ns = 0;
    stats.disk_wait_time_ns = 0;
    stats.memory_wait_time_ns = 0;
    stats.gpu_wait_time_ns = 0;
    stats.other_wait_time_ns = 0;
    stats.total_time_ns = 0;
    stats.last_update_ns = current_time;
    stats.cache_misses = 0;
    stats.cache_hits = 0;
    stats.branch_misses = 0;
    stats.branch_hits = 0;
    stats.page_faults = 0;
    stats.context_switches = 0;
    stats.system_calls = 0;
    stats.interrupts = 0;
    stats.memory_allocations = 0;
    stats.memory_frees = 0;

    bpf_get_current_comm(&stats.comm, sizeof(stats.comm));

    // Сохраняем в карту
    bpf_map_update_elem(&application_performance_map, &pid, &stats, BPF_ANY);

    return 0;
}

// Прикрепляемся к точке трассировки sched/sched_process_exit
// для отслеживания завершения процессов
SEC("tracepoint/sched/sched_process_exit")
int trace_process_exit(struct trace_event_raw_sched_process_exit *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;

    // Удаляем запись при завершении процесса
    bpf_map_delete_elem(&application_performance_map, &pid);

    return 0;
}

// Прикрепляемся к точке трассировки sched/sched_switch
// для отслеживания переключения процессов и учета времени выполнения
SEC("tracepoint/sched/sched_switch")
int trace_sched_switch(struct trace_event_raw_sched_switch *ctx)
{
    __u32 prev_pid = ctx->prev_pid;
    __u32 next_pid = ctx->next_pid;
    __u64 current_time = bpf_ktime_get_ns();

    // Обновляем статистику для предыдущего процесса
    struct application_performance_stats *prev_stats = bpf_map_lookup_elem(&application_performance_map, &prev_pid);
    if (prev_stats) {
        // Увеличиваем время выполнения для предыдущего процесса
        // В реальной системе нужно получить фактическое время выполнения
        __u64 exec_time_increase = 1000000; // 1 мс выполнения (пример)
        prev_stats->execution_time_ns += exec_time_increase;
        prev_stats->total_time_ns += exec_time_increase;
        prev_stats->last_update_ns = current_time;
    }

    // Инициализируем статистику для нового процесса, если еще не существует
    struct application_performance_stats *next_stats = bpf_map_lookup_elem(&application_performance_map, &next_pid);
    if (!next_stats) {
        struct application_performance_stats new_stats = {};
        new_stats.pid = next_pid;
        new_stats.tgid = next_pid; // Для нового процесса tgid = pid
        new_stats.last_update_ns = current_time;
        
        bpf_get_current_comm(&new_stats.comm, sizeof(new_stats.comm));
        
        bpf_map_update_elem(&application_performance_map, &next_pid, &new_stats, BPF_ANY);
    }

    return 0;
}

// Прикрепляемся к точке трассировки block/block_rq_issue
// для отслеживания ожидания диска
SEC("tracepoint/block/block_rq_issue")
int trace_block_rq_issue(struct trace_event_raw_block_rq_issue *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u64 current_time = bpf_ktime_get_ns();

    // Обновляем статистику ожидания диска
    struct application_performance_stats *stats = bpf_map_lookup_elem(&application_performance_map, &pid);
    if (stats) {
        __u64 disk_wait_increase = 500000; // 500 мкс ожидания диска (пример)
        stats->disk_wait_time_ns += disk_wait_increase;
        stats->total_time_ns += disk_wait_increase;
        stats->last_update_ns = current_time;
    }

    return 0;
}

// Прикрепляемся к точке трассировки net/net_dev_queue
// для отслеживания ожидания сети
SEC("tracepoint/net/net_dev_queue")
int trace_net_dev_queue(struct trace_event_raw_net_dev_queue *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u64 current_time = bpf_ktime_get_ns();

    // Обновляем статистику ожидания сети
    struct application_performance_stats *stats = bpf_map_lookup_elem(&application_performance_map, &pid);
    if (stats) {
        __u64 network_wait_increase = 300000; // 300 мкс ожидания сети (пример)
        stats->network_wait_time_ns += network_wait_increase;
        stats->total_time_ns += network_wait_increase;
        stats->last_update_ns = current_time;
    }

    return 0;
}

// Прикрепляемся к точке трассировки syscalls/sys_enter_futex
// для отслеживания ожидания блокировок
SEC("tracepoint/syscalls/sys_enter_futex")
int trace_futex_enter(struct trace_event_raw_sys_enter *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u64 current_time = bpf_ktime_get_ns();

    // Обновляем статистику ожидания блокировок
    struct application_performance_stats *stats = bpf_map_lookup_elem(&application_performance_map, &pid);
    if (stats) {
        __u64 lock_wait_increase = 200000; // 200 мкс ожидания блокировки (пример)
        stats->lock_wait_time_ns += lock_wait_increase;
        stats->total_time_ns += lock_wait_increase;
        stats->last_update_ns = current_time;
    }

    return 0;
}

// Прикрепляемся к точке трассировки syscalls/sys_enter_io_submit
// для отслеживания ожидания ввода-вывода
SEC("tracepoint/syscalls/sys_enter_io_submit")
int trace_io_submit_enter(struct trace_event_raw_sys_enter *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u64 current_time = bpf_ktime_get_ns();

    // Обновляем статистику ожидания ввода-вывода
    struct application_performance_stats *stats = bpf_map_lookup_elem(&application_performance_map, &pid);
    if (stats) {
        __u64 io_wait_increase = 400000; // 400 мкс ожидания ввода-вывода (пример)
        stats->io_wait_time_ns += io_wait_increase;
        stats->total_time_ns += io_wait_increase;
        stats->last_update_ns = current_time;
    }

    return 0;
}

// Прикрепляемся к точке трассировки sched/sched_stat_wait
// для отслеживания ожидания CPU
SEC("tracepoint/sched/sched_stat_wait")
int trace_sched_stat_wait(struct trace_event_raw_sched_stat_wait *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u64 current_time = bpf_ktime_get_ns();

    // Обновляем статистику ожидания CPU
    struct application_performance_stats *stats = bpf_map_lookup_elem(&application_performance_map, &pid);
    if (stats) {
        __u64 cpu_wait_increase = BPF_CORE_READ(ctx, delay);
        if (cpu_wait_increase > 0) {
            stats->cpu_wait_time_ns += cpu_wait_increase;
            stats->total_time_ns += cpu_wait_increase;
            stats->last_update_ns = current_time;
        }
    }

    return 0;
}

// Прикрепляемся к точке трассировки syscalls/sys_enter_mmap
// для отслеживания ожидания памяти
SEC("tracepoint/syscalls/sys_enter_mmap")
int trace_mmap_enter(struct trace_event_raw_sys_enter *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u64 current_time = bpf_ktime_get_ns();

    // Обновляем статистику ожидания памяти
    struct application_performance_stats *stats = bpf_map_lookup_elem(&application_performance_map, &pid);
    if (stats) {
        __u64 memory_wait_increase = 150000; // 150 мкс ожидания памяти (пример)
        stats->memory_wait_time_ns += memory_wait_increase;
        stats->total_time_ns += memory_wait_increase;
        stats->last_update_ns = current_time;
    }

    return 0;
}

// Прикрепляемся к точке трассировки exceptions/page_fault_user
// для отслеживания page faults
SEC("tracepoint/exceptions/page_fault_user")
int trace_page_fault_user(struct trace_event_raw_page_fault_user *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u64 current_time = bpf_ktime_get_ns();

    // Обновляем статистику page faults
    struct application_performance_stats *stats = bpf_map_lookup_elem(&application_performance_map, &pid);
    if (stats) {
        stats->page_faults += 1;
        stats->last_update_ns = current_time;
    }

    return 0;
}

// Прикрепляемся к точке трассировки sched/sched_switch
// для отслеживания переключений контекста
SEC("tracepoint/sched/sched_switch")
int trace_context_switch(struct trace_event_raw_sched_switch *ctx)
{
    __u32 prev_pid = ctx->prev_pid;
    __u32 next_pid = ctx->next_pid;
    __u64 current_time = bpf_ktime_get_ns();

    // Обновляем статистику переключений контекста для предыдущего процесса
    struct application_performance_stats *prev_stats = bpf_map_lookup_elem(&application_performance_map, &prev_pid);
    if (prev_stats) {
        prev_stats->context_switches += 1;
        prev_stats->last_update_ns = current_time;
    }

    // Обновляем статистику переключений контекста для нового процесса
    struct application_performance_stats *next_stats = bpf_map_lookup_elem(&application_performance_map, &next_pid);
    if (next_stats) {
        next_stats->context_switches += 1;
        next_stats->last_update_ns = current_time;
    }

    return 0;
}

// Прикрепляемся к точке трассировки syscalls/sys_enter
// для отслеживания системных вызовов
SEC("tracepoint/syscalls/sys_enter")
int trace_syscall_enter(struct trace_event_raw_sys_enter *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u64 current_time = bpf_ktime_get_ns();

    // Обновляем статистику системных вызовов
    struct application_performance_stats *stats = bpf_map_lookup_elem(&application_performance_map, &pid);
    if (stats) {
        stats->system_calls += 1;
        stats->last_update_ns = current_time;
    }

    return 0;
}

// Прикрепляемся к точке трассировки irq/irq_handler_entry
// для отслеживания прерываний
SEC("tracepoint/irq/irq_handler_entry")
int trace_irq_handler_entry(struct trace_event_raw_irq_handler_entry *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u64 current_time = bpf_ktime_get_ns();

    // Обновляем статистику прерываний
    struct application_performance_stats *stats = bpf_map_lookup_elem(&application_performance_map, &pid);
    if (stats) {
        stats->interrupts += 1;
        stats->last_update_ns = current_time;
    }

    return 0;
}

// Прикрепляемся к точке трассировки kmem/kmalloc
// для отслеживания выделений памяти
SEC("tracepoint/kmem/kmalloc")
int trace_kmalloc(struct trace_event_raw_kmalloc *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u64 current_time = bpf_ktime_get_ns();

    // Обновляем статистику выделений памяти
    struct application_performance_stats *stats = bpf_map_lookup_elem(&application_performance_map, &pid);
    if (stats) {
        stats->memory_allocations += 1;
        stats->last_update_ns = current_time;
    }

    return 0;
}

// Прикрепляемся к точке трассировки kmem/kfree
// для отслеживания освобождений памяти
SEC("tracepoint/kmem/kfree")
int trace_kfree(struct trace_event_raw_kfree *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u64 current_time = bpf_ktime_get_ns();

    // Обновляем статистику освобождений памяти
    struct application_performance_stats *stats = bpf_map_lookup_elem(&application_performance_map, &pid);
    if (stats) {
        stats->memory_frees += 1;
        stats->last_update_ns = current_time;
    }

    return 0;
}

// Лицензия
char _license[] SEC("license") = "GPL";
