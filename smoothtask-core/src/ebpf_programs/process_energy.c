// SPDX-License-Identifier: GPL-2.0 OR BSD-3-Clause
/* Copyright (c) 2025 SmoothTask Authors */

// eBPF программа для мониторинга энергопотребления процессов
// Использует RAPL (Running Average Power Limit) интерфейсы для отслеживания
// потребления энергии на уровне процессов

#include "vmlinux.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>

// Структура для хранения статистики энергопотребления процесса
struct process_energy_stats {
    __u32 pid;
    __u32 tgid;
    __u64 energy_uj;      // Потребление энергии в микроджоулях
    __u64 last_update_ns; // Последнее обновление в наносекундах
    __u32 cpu_id;        // CPU, на котором выполняется процесс
};

// Карта для хранения статистики энергопотребления процессов
// Используем BPF_MAP_TYPE_HASH для эффективного доступа
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 10240);
    __type(key, __u32);           // PID как ключ
    __type(value, struct process_energy_stats);
} process_energy_map SEC(".maps");

// Карта для хранения глобальной статистики энергопотребления
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 128);
    __type(key, __u32);           // CPU ID как ключ
    __type(value, __u64);         // Общее потребление энергии на CPU
} global_energy_map SEC(".maps");

// Прикрепляемся к точке трассировки sched/sched_process_exec
// для отслеживания запуска новых процессов
SEC("tracepoint/sched/sched_process_exec")
int trace_process_exec(struct trace_event_raw_sched_process_exec *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u32 tgid = bpf_get_current_pid_tgid();
    __u32 cpu_id = bpf_get_smp_processor_id();
    __u64 current_time = bpf_ktime_get_ns();

    // Инициализируем запись для нового процесса
    struct process_energy_stats stats = {};
    stats.pid = pid;
    stats.tgid = tgid;
    stats.energy_uj = 0;
    stats.last_update_ns = current_time;
    stats.cpu_id = cpu_id;

    // Сохраняем в карту
    bpf_map_update_elem(&process_energy_map, &pid, &stats, BPF_ANY);

    return 0;
}

// Прикрепляемся к точке трассировки sched/sched_process_exit
// для отслеживания завершения процессов
SEC("tracepoint/sched/sched_process_exit")
int trace_process_exit(struct trace_event_raw_sched_process_exit *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;

    // Удаляем запись при завершении процесса
    bpf_map_delete_elem(&process_energy_map, &pid);

    return 0;
}

// Прикрепляемся к точке трассировки power/power_start
// для отслеживания потребления энергии
SEC("tracepoint/power/power_start")
int trace_power_usage(struct trace_event_raw_power_start *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u32 tgid = bpf_get_current_pid_tgid();
    __u32 cpu_id = bpf_get_smp_processor_id();
    __u64 current_time = bpf_ktime_get_ns();

    // Получаем текущую статистику процесса
    struct process_energy_stats *stats = bpf_map_lookup_elem(&process_energy_map, &pid);
    if (!stats) {
        // Если записи нет, создаем новую
        struct process_energy_stats new_stats = {};
        new_stats.pid = pid;
        new_stats.tgid = tgid;
        new_stats.energy_uj = 0;
        new_stats.last_update_ns = current_time;
        new_stats.cpu_id = cpu_id;
        bpf_map_update_elem(&process_energy_map, &pid, &new_stats, BPF_ANY);
        return 0;
    }

    // Увеличиваем потребление энергии
    // В реальной системе нужно получить фактическое значение из события
    // Для примера используем фиксированное значение
    __u64 energy_increase = 1000; // 1000 микроджоулей (1 миллиджоуль)

    // В реальной реализации можно использовать данные из ctx
    // Например: energy_increase = BPF_CORE_READ(ctx, energy_uj);

    stats->energy_uj += energy_increase;
    stats->last_update_ns = current_time;

    // Обновляем глобальную статистику для CPU
    __u64 *global_energy = bpf_map_lookup_elem(&global_energy_map, &cpu_id);
    if (global_energy) {
        *global_energy += energy_increase;
    } else {
        __u64 initial_energy = energy_increase;
        bpf_map_update_elem(&global_energy_map, &cpu_id, &initial_energy, BPF_ANY);
    }

    return 0;
}

// Прикрепляемся к точке трассировки sched/sched_switch
// для отслеживания переключения процессов
SEC("tracepoint/sched/sched_switch")
int trace_sched_switch(struct trace_event_raw_sched_switch *ctx)
{
    // Можно использовать для более точного учета энергопотребления
    // при переключении процессов
    return 0;
}

// Лицензия
char _license[] SEC("license") = "GPL";