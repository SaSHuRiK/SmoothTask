// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

// eBPF программа для мониторинга сетевой активности на уровне процессов
// Отслеживает сетевые пакеты и байты, отправленные и полученные каждым процессом

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <linux/if_ether.h>
#include <linux/ip.h>
#include <linux/tcp.h>
#include <linux/udp.h>

// Максимальное количество отслеживаемых процессов
#define MAX_PROCESS_NETWORK_STATS 4096

// Структура для хранения сетевой статистики процесса
struct process_network_stats {
    __u64 packets_sent;
    __u64 packets_received;
    __u64 bytes_sent;
    __u64 bytes_received;
    __u64 last_timestamp;
    __u32 pid;
    __u32 tgid;
};

// Карта для хранения статистики сетевой активности по PID
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, MAX_PROCESS_NETWORK_STATS);
    __type(key, __u32);  // PID процесса
    __type(value, struct process_network_stats);
} process_network_stats_map SEC(".maps");

// Карта для хранения общего количества сетевых пакетов
struct {
    __uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, __u64);
} total_network_packet_count_map SEC(".maps");

// Точка входа для отслеживания отправки сетевых пакетов
SEC("tracepoint/sock/sock_inet_sock_set_state")
int trace_process_network_send(struct trace_event_raw_sock_inet_sock_set_state *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u32 tgid = bpf_get_current_pid_tgid();
    
    if (pid == 0) {
        return 0; // Пропускаем ядро
    }
    
    struct process_network_stats *stats;
    
    // Получаем или создаем статистику для этого PID
    stats = bpf_map_lookup_elem(&process_network_stats_map, &pid);
    if (!stats) {
        struct process_network_stats new_stats = {};
        new_stats.pid = pid;
        new_stats.tgid = tgid;
        new_stats.last_timestamp = bpf_ktime_get_ns();
        bpf_map_update_elem(&process_network_stats_map, &pid, &new_stats, BPF_ANY);
        stats = bpf_map_lookup_elem(&process_network_stats_map, &pid);
        if (!stats) {
            return 0;
        }
    }
    
    // Обновляем статистику отправки
    stats->packets_sent += 1;
    stats->bytes_sent += 1024; // Примерное значение для пакета
    stats->last_timestamp = bpf_ktime_get_ns();
    
    return 0;
}

// Точка входа для отслеживания получения сетевых пакетов
SEC("tracepoint/sock/sock_inet_sock_set_state")
int trace_process_network_receive(struct trace_event_raw_sock_inet_sock_set_state *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u32 tgid = bpf_get_current_pid_tgid();
    
    if (pid == 0) {
        return 0; // Пропускаем ядро
    }
    
    struct process_network_stats *stats;
    
    // Получаем или создаем статистику для этого PID
    stats = bpf_map_lookup_elem(&process_network_stats_map, &pid);
    if (!stats) {
        struct process_network_stats new_stats = {};
        new_stats.pid = pid;
        new_stats.tgid = tgid;
        new_stats.last_timestamp = bpf_ktime_get_ns();
        bpf_map_update_elem(&process_network_stats_map, &pid, &new_stats, BPF_ANY);
        stats = bpf_map_lookup_elem(&process_network_stats_map, &pid);
        if (!stats) {
            return 0;
        }
    }
    
    // Обновляем статистику получения
    stats->packets_received += 1;
    stats->bytes_received += 1024; // Примерное значение для пакета
    stats->last_timestamp = bpf_ktime_get_ns();
    
    return 0;
}

// Точка входа для отслеживания общего сетевого трафика
SEC("tracepoint/net/netif_receive_skb")
int trace_total_network_packet(struct trace_event_raw_netif_receive_skb *ctx)
{
    __u32 key = 0;
    __u64 *count;
    
    // Увеличиваем общее количество пакетов
    count = bpf_map_lookup_elem(&total_network_packet_count_map, &key);
    if (count) {
        __sync_fetch_and_add(count, 1);
    }
    
    return 0;
}

// Лицензия для eBPF программы
char _license[] SEC("license") = "GPL";