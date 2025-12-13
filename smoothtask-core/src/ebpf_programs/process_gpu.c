// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2025 SmoothTask Authors */

// eBPF программа для мониторинга использования GPU на уровне процессов
// Отслеживает использование GPU ресурсов процессами через DRM и GPU tracepoints

#include "vmlinux.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>

// Структура для хранения статистики использования GPU процесса
struct process_gpu_stats {
    __u32 pid;                  // PID процесса
    __u32 tgid;                 // TGID (группа потоков)
    __u64 gpu_time_ns;          // Время использования GPU в наносекундах
    __u64 memory_usage_bytes;   // Использование памяти GPU в байтах
    __u64 compute_units_used;   // Количество использованных вычислительных единиц
    __u64 last_update_ns;       // Последнее обновление в наносекундах
    __u32 gpu_id;              // Идентификатор GPU устройства
    __u32 temperature_celsius;  // Температура GPU во время использования
};

// Карта для хранения статистики использования GPU процессами
// Используем BPF_MAP_TYPE_HASH для эффективного доступа
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 10240);
    __type(key, __u32);           // PID как ключ
    __type(value, struct process_gpu_stats);
} process_gpu_map SEC(".maps");

// Карта для хранения глобальной статистики использования GPU
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 128);
    __type(key, __u32);           // GPU ID как ключ
    __type(value, __u64);         // Общее время использования GPU
} global_gpu_usage_map SEC(".maps");

// Прикрепляемся к точке трассировки DRM для отслеживания запуска задач GPU
SEC("tracepoint/drm/drm_gpu_sched_run_job")
int trace_gpu_job_start(struct trace_event_raw_drm_gpu_sched_run_job *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u32 tgid = bpf_get_current_pid_tgid();
    __u32 gpu_id = 0; // В реальной реализации нужно получить GPU ID из контекста
    __u64 current_time = bpf_ktime_get_ns();

    // Инициализируем или обновляем запись для процесса
    struct process_gpu_stats *stats = bpf_map_lookup_elem(&process_gpu_map, &pid);
    if (!stats) {
        // Создаем новую запись
        struct process_gpu_stats new_stats = {};
        new_stats.pid = pid;
        new_stats.tgid = tgid;
        new_stats.gpu_id = gpu_id;
        new_stats.last_update_ns = current_time;
        bpf_map_update_elem(&process_gpu_map, &pid, &new_stats, BPF_ANY);
        return 0;
    }

    // Обновляем существующую запись
    stats->last_update_ns = current_time;
    stats->gpu_id = gpu_id;

    return 0;
}

// Прикрепляемся к точке трассировки DRM для отслеживания завершения задач GPU
SEC("tracepoint/drm/drm_gpu_sched_job_end")
int trace_gpu_job_end(struct trace_event_raw_drm_gpu_sched_job_end *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u64 current_time = bpf_ktime_get_ns();

    // Получаем статистику процесса
    struct process_gpu_stats *stats = bpf_map_lookup_elem(&process_gpu_map, &pid);
    if (!stats) {
        return 0;
    }

    // Рассчитываем время использования GPU
    __u64 delta = current_time - stats->last_update_ns;
    __sync_fetch_and_add(&stats->gpu_time_ns, delta);

    // Увеличиваем количество использованных вычислительных единиц
    __sync_fetch_and_add(&stats->compute_units_used, 1);

    // Обновляем глобальную статистику для GPU
    __u64 *global_usage = bpf_map_lookup_elem(&global_gpu_usage_map, &stats->gpu_id);
    if (global_usage) {
        __sync_fetch_and_add(global_usage, delta);
    } else {
        __u64 initial_usage = delta;
        bpf_map_update_elem(&global_gpu_usage_map, &stats->gpu_id, &initial_usage, BPF_ANY);
    }

    return 0;
}

// Прикрепляемся к точке трассировки DRM для отслеживания использования памяти GPU
SEC("tracepoint/drm/drm_gem_object_create")
int trace_gpu_memory_alloc(struct trace_event_raw_drm_gem_object_create *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u64 memory_increase = 4096; // Пример: 4KB увеличение (в реальности нужно получить из ctx)

    // Получаем или создаем статистику процесса
    struct process_gpu_stats *stats = bpf_map_lookup_elem(&process_gpu_map, &pid);
    if (!stats) {
        __u32 tgid = bpf_get_current_pid_tgid();
        struct process_gpu_stats new_stats = {};
        new_stats.pid = pid;
        new_stats.tgid = tgid;
        new_stats.memory_usage_bytes = memory_increase;
        new_stats.last_update_ns = bpf_ktime_get_ns();
        bpf_map_update_elem(&process_gpu_map, &pid, &new_stats, BPF_ANY);
        return 0;
    }

    // Увеличиваем использование памяти
    __sync_fetch_and_add(&stats->memory_usage_bytes, memory_increase);

    return 0;
}

// Прикрепляемся к точке трассировки DRM для отслеживания освобождения памяти GPU
SEC("tracepoint/drm/drm_gem_object_free")
int trace_gpu_memory_free(struct trace_event_raw_drm_gem_object_free *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u64 memory_decrease = 4096; // Пример: 4KB уменьшение (в реальности нужно получить из ctx)

    // Получаем статистику процесса
    struct process_gpu_stats *stats = bpf_map_lookup_elem(&process_gpu_map, &pid);
    if (!stats) {
        return 0;
    }

    // Уменьшаем использование памяти (но не ниже 0)
    if (stats->memory_usage_bytes >= memory_decrease) {
        __sync_fetch_and_sub(&stats->memory_usage_bytes, memory_decrease);
    }

    return 0;
}

// Прикрепляемся к точке трассировки sched/sched_process_exec для отслеживания новых процессов
SEC("tracepoint/sched/sched_process_exec")
int trace_process_exec(struct trace_event_raw_sched_process_exec *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u32 tgid = bpf_get_current_pid_tgid();

    // Инициализируем запись для нового процесса
    struct process_gpu_stats stats = {};
    stats.pid = pid;
    stats.tgid = tgid;
    stats.last_update_ns = bpf_ktime_get_ns();

    // Сохраняем в карту
    bpf_map_update_elem(&process_gpu_map, &pid, &stats, BPF_ANY);

    return 0;
}

// Прикрепляемся к точке трассировки sched/sched_process_exit для отслеживания завершения процессов
SEC("tracepoint/sched/sched_process_exit")
int trace_process_exit(struct trace_event_raw_sched_process_exit *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;

    // Удаляем запись при завершении процесса
    bpf_map_delete_elem(&process_gpu_map, &pid);

    return 0;
}

// Лицензия
char _license[] SEC("license") = "GPL";