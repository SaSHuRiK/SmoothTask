// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

// eBPF программа для мониторинга температуры CPU

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

// Структура для хранения температуры CPU
struct cpu_temperature {
    __u32 temperature_celsius;
    __u32 max_temperature_celsius;
    __u64 timestamp;
    __u32 cpu_id;
};

// Карта для хранения температуры CPU по идентификатору CPU
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
    __uint(max_entries, 256); // Поддержка до 256 CPU ядер
    __type(key, __u32);
    __type(value, struct cpu_temperature);
} cpu_temperature_map SEC(".maps");

// Точка входа для мониторинга температуры CPU
// Используем точку трассировки для обновления температуры
SEC("tracepoint/thermal/thermal_zone_trip")
int trace_cpu_temperature(struct trace_event_raw_thermal_zone_trip *ctx)
{
    __u32 cpu_id = bpf_get_smp_processor_id();
    struct cpu_temperature *temp;
    
    // Получаем текущее время
    __u64 timestamp = bpf_ktime_get_ns();
    
    // Получаем доступ к карте температуры
    temp = bpf_map_lookup_elem(&cpu_temperature_map, &cpu_id);
    if (!temp)
        return 0;
    
    // Обновляем температуру
    // В реальной системе нужно получить температуру из соответствующего источника
    // Для тестирования используем фиксированное значение
    temp->temperature_celsius = 50; // Примерное значение
    temp->max_temperature_celsius = 80; // Максимальное значение
    temp->timestamp = timestamp;
    temp->cpu_id = cpu_id;
    
    return 0;
}

// Лицензия для eBPF программы
char _license[] SEC("license") = "GPL";