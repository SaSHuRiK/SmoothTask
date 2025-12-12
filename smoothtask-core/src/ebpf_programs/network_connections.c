// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

// eBPF программа для мониторинга сетевых соединений
// Отслеживает активные TCP/UDP соединения и их состояние

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <linux/if_ether.h>
#include <linux/ip.h>
#include <linux/tcp.h>
#include <linux/udp.h>
#include <linux/in.h>

// Максимальное количество отслеживаемых соединений
#define MAX_CONNECTIONS 2048

// Структура для хранения информации о сетевых соединениях
struct connection_info {
    __u32 saddr;           // Источник IP адрес
    __u32 daddr;           // Назначение IP адрес
    __u16 sport;           // Источник порт
    __u16 dport;           // Назначение порт
    __u8 protocol;         // Протокол (TCP/UDP)
    __u8 state;            // Состояние соединения
    __u64 packets;         // Количество пакетов
    __u64 bytes;           // Количество байт
    __u64 start_time;      // Время начала соединения
    __u64 last_activity;   // Время последней активности
};

// Карта для хранения информации о соединениях
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, MAX_CONNECTIONS);
    __type(key, __u64);  // Уникальный идентификатор соединения
    __type(value, struct connection_info);
} connection_map SEC(".maps");

// Карта для хранения статистики по соединениям
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, MAX_CONNECTIONS);
    __type(key, __u64);  // Уникальный идентификатор соединения
    __type(value, __u64); // Количество соединений
} connection_stats_map SEC(".maps");

// Карта для хранения активных соединений
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, MAX_CONNECTIONS);
    __type(key, __u64);  // Уникальный идентификатор соединения
    __type(value, __u8); // Флаг активности
} active_connections_map SEC(".maps");

// Точка входа для отслеживания TCP соединений
SEC("tracepoint/sock/sock_inet_sock_set_state")
int trace_tcp_connection(struct trace_event_raw_sock_inet_sock_set_state *ctx) {
    __u64 conn_id = 0;
    struct connection_info conn_info = {};
    
    // Извлекаем информацию о соединении
    conn_info.saddr = ctx->saddr;
    conn_info.daddr = ctx->daddr;
    conn_info.sport = ctx->sport;
    conn_info.dport = ctx->dport;
    conn_info.protocol = IPPROTO_TCP;
    conn_info.state = ctx->newstate;
    conn_info.start_time = bpf_ktime_get_ns();
    conn_info.last_activity = bpf_ktime_get_ns();
    
    // Создаем уникальный идентификатор соединения
    conn_id = (__u64)conn_info.saddr << 32 | conn_info.daddr;
    conn_id ^= (__u64)conn_info.sport << 16 | conn_info.dport;
    
    // Сохраняем информацию о соединении
    bpf_map_update_elem(&connection_map, &conn_id, &conn_info, BPF_ANY);
    
    // Обновляем статистику соединений
    __u64 *count = bpf_map_lookup_elem(&connection_stats_map, &conn_id);
    if (count) {
        __sync_fetch_and_add(count, 1);
    } else {
        __u64 new_count = 1;
        bpf_map_update_elem(&connection_stats_map, &conn_id, &new_count, BPF_ANY);
    }
    
    // Помечаем соединение как активное
    __u8 active = 1;
    bpf_map_update_elem(&active_connections_map, &conn_id, &active, BPF_ANY);
    
    return 0;
}

// Точка входа для отслеживания UDP пакетов
SEC("tracepoint/net/netif_receive_skb")
int trace_udp_packet(struct trace_event_raw_netif_receive_skb *ctx) {
    // В реальной реализации здесь будет анализ UDP пакетов
    // Пока что это заглушка
    return 0;
}

// Точка входа для отслеживания сетевых пакетов
SEC("tracepoint/net/net_dev_queue")
int trace_network_packet(struct trace_event_raw_net_dev_queue *ctx) {
    // В реальной реализации здесь будет анализ сетевых пакетов
    // Пока что это заглушка
    return 0;
}

// Точка входа для отслеживания закрытия соединений
SEC("tracepoint/sock/sock_inet_sock_set_state")
int trace_connection_close(struct trace_event_raw_sock_inet_sock_set_state *ctx) {
    __u64 conn_id = 0;
    struct connection_info *conn_info;
    
    // Создаем уникальный идентификатор соединения
    conn_id = (__u64)ctx->saddr << 32 | ctx->daddr;
    conn_id ^= (__u64)ctx->sport << 16 | ctx->dport;
    
    // Получаем информацию о соединении
    conn_info = bpf_map_lookup_elem(&connection_map, &conn_id);
    if (conn_info) {
        // Обновляем время последней активности
        conn_info->last_activity = bpf_ktime_get_ns();
        conn_info->state = ctx->newstate;
        
        // Обновляем информацию о соединении
        bpf_map_update_elem(&connection_map, &conn_id, conn_info, BPF_ANY);
    }
    
    return 0;
}

// Точка входа для отслеживания передачи данных
SEC("tracepoint/sock/sock_inet_sock_set_state")
int trace_connection_data(struct trace_event_raw_sock_inet_sock_set_state *ctx) {
    __u64 conn_id = 0;
    struct connection_info *conn_info;
    
    // Создаем уникальный идентификатор соединения
    conn_id = (__u64)ctx->saddr << 32 | ctx->daddr;
    conn_id ^= (__u64)ctx->sport << 16 | ctx->dport;
    
    // Получаем информацию о соединении
    conn_info = bpf_map_lookup_elem(&connection_map, &conn_id);
    if (conn_info) {
        // Увеличиваем счетчики пакетов и байт
        conn_info->packets += 1;
        conn_info->bytes += 1024; // Примерное значение
        conn_info->last_activity = bpf_ktime_get_ns();
        
        // Обновляем информацию о соединении
        bpf_map_update_elem(&connection_map, &conn_id, conn_info, BPF_ANY);
    }
    
    return 0;
}

// Лицензия для eBPF программы
char _license[] SEC("license") = "GPL";