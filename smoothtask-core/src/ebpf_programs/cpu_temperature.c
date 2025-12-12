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
    // Пробуем получить реальную температуру из события thermal_zone_trip
    // Если это не удается, используем резервные значения
    __u32 current_temp = 50; // Значение по умолчанию
    __u32 max_temp = 80;     // Максимальное значение по умолчанию
    
    // Пробуем извлечь температуру из контекста события
    // Примечание: структура события может отличаться в зависимости от версии ядра
    // Это базовая реализация, которая может потребовать адаптации
    if (ctx) {
        // Пробуем получить температуру из поля temp (если доступно)
        // В реальных системах это может быть ctx->temp или другое поле
        // Для безопасности используем проверки
        if (ctx->temp > 0 && ctx->temp < 150) { // Разумный диапазон температур
            current_temp = ctx->temp / 1000; // Преобразуем из миллиградусов в градусы
        }
        
        // Пробуем получить максимальную температуру
        if (ctx->trip_temp > 0 && ctx->trip_temp < 150) {
            max_temp = ctx->trip_temp / 1000;
        }
    }
    
    // Обновляем структуру температуры
    temp->temperature_celsius = current_temp;
    temp->max_temperature_celsius = max_temp;
    temp->timestamp = timestamp;
    temp->cpu_id = cpu_id;
    
    // Логируем событие (в режиме отладки)
    bpf_trace_printk("CPU Temp: CPU %d, Temp: %d°C, Max: %d°C\\n", cpu_id, current_temp, max_temp);
    
    return 0;
}

// Альтернативная точка входа для мониторинга температуры CPU
// Используем kprobe для функции, которая читает температуру CPU
SEC("kprobe/thermal_zone_get_temp")
int kprobe_thermal_zone_get_temp(struct pt_regs *ctx)
{
    __u32 cpu_id = bpf_get_smp_processor_id();
    struct cpu_temperature *temp;
    
    // Получаем текущее время
    __u64 timestamp = bpf_ktime_get_ns();
    
    // Получаем доступ к карте температуры
    temp = bpf_map_lookup_elem(&cpu_temperature_map, &cpu_id);
    if (!temp)
        return 0;
    
    // Пробуем получить температуру из аргументов функции
    // Аргумент 1 (диапазон 0-4) обычно содержит указатель на структуру thermal_zone_device
    // Аргумент 2 (диапазон 5-9) обычно содержит указатель на переменную для хранения температуры
    // Это зависит от архитектуры и версии ядра
    
    __u64 temp_ptr = PT_REGS_PARM2(ctx);
    __u32 current_temp = 0;
    
    // Пробуем прочитать значение температуры по указателю
    if (bpf_probe_read(&current_temp, sizeof(current_temp), (void *)temp_ptr) == 0) {
        // Успешно прочитали температуру
        if (current_temp > 0 && current_temp < 150000) { // Разные диапазоны в зависимости от масштаба
            // Преобразуем в градусы Цельсия (может быть в миллиградусах)
            temp->temperature_celsius = current_temp / 1000;
            temp->max_temperature_celsius = temp->temperature_celsius + 20; // Добавляем запас
            temp->timestamp = timestamp;
            temp->cpu_id = cpu_id;
            
            bpf_trace_printk("CPU Temp (kprobe): CPU %d, Temp: %d°C\\n", cpu_id, temp->temperature_celsius);
        }
    }
    
    return 0;
}

// Лицензия для eBPF программы
char _license[] SEC("license") = "GPL";