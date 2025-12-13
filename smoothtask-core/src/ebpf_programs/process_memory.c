// SPDX-License-Identifier: GPL-2.0 OR BSD-3-Clause
/* Copyright (c) 2025 SmoothTask Authors */

// eBPF program for monitoring process memory usage
// Tracks memory allocations, deallocations, and usage patterns per process

#include "vmlinux.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>

// Configuration structure for process memory monitoring
struct process_memory_config {
    bool enable_detailed_stats;
    u32 sampling_rate;
    u64 min_memory_threshold;
};

// Memory statistics structure
struct process_memory_stat {
    u32 pid;
    u64 timestamp;
    u64 rss_bytes;
    u64 vms_bytes;
    u64 shared_bytes;
    u64 swap_bytes;
    u64 heap_usage;
    u64 stack_usage;
    u64 anonymous_memory;
    u64 file_backed_memory;
    u64 major_faults;
    u64 minor_faults;
};

// BPF map for storing process memory statistics
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 10240);
    __type(key, u32); // PID
    __type(value, struct process_memory_stat);
} process_memory_stats SEC(".maps");

// BPF map for configuration
struct {
    __uint(type, BPF_MAP_TYPE_ARRAY);
    __uint(max_entries, 1);
    __type(key, u32);
    __type(value, struct process_memory_config);
} process_memory_config_map SEC(".maps");

// Helper function to get current timestamp in nanoseconds
static inline u64 get_current_timestamp() {
    return bpf_ktime_get_ns();
}

// Trace memory allocation events
SEC("tracepoint/syscalls/sys_enter_mmap")
int trace_mmap_enter(struct trace_event_raw_sys_enter *ctx) {
    u32 pid = bpf_get_current_pid_tgid() >> 32;
    
    // Check if we should sample this event
    u32 index = 0;
    struct process_memory_config *config = bpf_map_lookup_elem(&process_memory_config_map, &index);
    if (!config || !config->enable_detailed_stats) {
        return 0;
    }
    
    // Sample at configured rate
    u32 sample_key = bpf_get_prandom_u32() % config->sampling_rate;
    if (sample_key != 0) {
        return 0;
    }
    
    // Get current memory stats for this process
    struct process_memory_stat stat = {};
    stat.pid = pid;
    stat.timestamp = get_current_timestamp();
    
    // Read memory statistics from task struct
    struct task_struct *task = (struct task_struct *)bpf_get_current_task();
    
    // Read RSS (Resident Set Size)
    stat.rss_bytes = BPF_CORE_READ(task, mm, rss_stat.count[MM_FILEPAGES]) * PAGE_SIZE;
    stat.rss_bytes += BPF_CORE_READ(task, mm, rss_stat.count[MM_ANONPAGES]) * PAGE_SIZE;
    stat.rss_bytes += BPF_CORE_READ(task, mm, rss_stat.count[MM_SHMEMPAGES]) * PAGE_SIZE;
    
    // Read VMS (Virtual Memory Size)
    stat.vms_bytes = BPF_CORE_READ(task, mm, total_vm) * PAGE_SIZE;
    
    // Read shared memory
    stat.shared_bytes = BPF_CORE_READ(task, mm, shared_vm) * PAGE_SIZE;
    
    // Read swap usage
    stat.swap_bytes = BPF_CORE_READ(task, mm, swap_addresses) * PAGE_SIZE;
    
    // Read heap and stack usage (approximate)
    stat.heap_usage = BPF_CORE_READ(task, mm, start_brk) - BPF_CORE_READ(task, mm, brk);
    stat.stack_usage = BPF_CORE_READ(task, mm, start_stack) - BPF_CORE_READ(task, mm, stack.vm_start);
    
    // Read anonymous vs file-backed memory
    stat.anonymous_memory = BPF_CORE_READ(task, mm, rss_stat.count[MM_ANONPAGES]) * PAGE_SIZE;
    stat.file_backed_memory = BPF_CORE_READ(task, mm, rss_stat.count[MM_FILEPAGES]) * PAGE_SIZE;
    
    // Read page faults
    stat.major_faults = BPF_CORE_READ(task, maj_flt);
    stat.minor_faults = BPF_CORE_READ(task, min_flt);
    
    // Store statistics
    bpf_map_update_elem(&process_memory_stats, &pid, &stat, BPF_ANY);
    
    return 0;
}

// Trace memory deallocation events
SEC("tracepoint/syscalls/sys_enter_munmap")
int trace_munmap_enter(struct trace_event_raw_sys_enter *ctx) {
    u32 pid = bpf_get_current_pid_tgid() >> 32;
    
    // Check if we should sample this event
    u32 index = 0;
    struct process_memory_config *config = bpf_map_lookup_elem(&process_memory_config_map, &index);
    if (!config || !config->enable_detailed_stats) {
        return 0;
    }
    
    // Sample at configured rate
    u32 sample_key = bpf_get_prandom_u32() % config->sampling_rate;
    if (sample_key != 0) {
        return 0;
    }
    
    // Get current memory stats for this process
    struct process_memory_stat stat = {};
    stat.pid = pid;
    stat.timestamp = get_current_timestamp();
    
    // Read memory statistics from task struct
    struct task_struct *task = (struct task_struct *)bpf_get_current_task();
    
    // Read RSS (Resident Set Size)
    stat.rss_bytes = BPF_CORE_READ(task, mm, rss_stat.count[MM_FILEPAGES]) * PAGE_SIZE;
    stat.rss_bytes += BPF_CORE_READ(task, mm, rss_stat.count[MM_ANONPAGES]) * PAGE_SIZE;
    stat.rss_bytes += BPF_CORE_READ(task, mm, rss_stat.count[MM_SHMEMPAGES]) * PAGE_SIZE;
    
    // Read VMS (Virtual Memory Size)
    stat.vms_bytes = BPF_CORE_READ(task, mm, total_vm) * PAGE_SIZE;
    
    // Read shared memory
    stat.shared_bytes = BPF_CORE_READ(task, mm, shared_vm) * PAGE_SIZE;
    
    // Read swap usage
    stat.swap_bytes = BPF_CORE_READ(task, mm, swap_addresses) * PAGE_SIZE;
    
    // Read heap and stack usage (approximate)
    stat.heap_usage = BPF_CORE_READ(task, mm, start_brk) - BPF_CORE_READ(task, mm, brk);
    stat.stack_usage = BPF_CORE_READ(task, mm, start_stack) - BPF_CORE_READ(task, mm, stack.vm_start);
    
    // Read anonymous vs file-backed memory
    stat.anonymous_memory = BPF_CORE_READ(task, mm, rss_stat.count[MM_ANONPAGES]) * PAGE_SIZE;
    stat.file_backed_memory = BPF_CORE_READ(task, mm, rss_stat.count[MM_FILEPAGES]) * PAGE_SIZE;
    
    // Read page faults
    stat.major_faults = BPF_CORE_READ(task, maj_flt);
    stat.minor_faults = BPF_CORE_READ(task, min_flt);
    
    // Store statistics
    bpf_map_update_elem(&process_memory_stats, &pid, &stat, BPF_ANY);
    
    return 0;
}

// Periodic collection of memory statistics
SEC("kprobe/finish_task_switch")
int trace_task_switch(struct pt_regs *ctx) {
    u32 pid = bpf_get_current_pid_tgid() >> 32;
    
    // Check if we should collect statistics for this process
    u32 index = 0;
    struct process_memory_config *config = bpf_map_lookup_elem(&process_memory_config_map, &index);
    if (!config) {
        return 0;
    }
    
    // Only collect if memory usage is above threshold
    struct task_struct *task = (struct task_struct *)bpf_get_current_task();
    u64 rss = BPF_CORE_READ(task, mm, rss_stat.count[MM_FILEPAGES]) * PAGE_SIZE;
    rss += BPF_CORE_READ(task, mm, rss_stat.count[MM_ANONPAGES]) * PAGE_SIZE;
    rss += BPF_CORE_READ(task, mm, rss_stat.count[MM_SHMEMPAGES]) * PAGE_SIZE;
    
    if (rss < config->min_memory_threshold) {
        return 0;
    }
    
    // Collect memory statistics
    struct process_memory_stat stat = {};
    stat.pid = pid;
    stat.timestamp = get_current_timestamp();
    
    // Read memory statistics from task struct
    stat.rss_bytes = rss;
    stat.vms_bytes = BPF_CORE_READ(task, mm, total_vm) * PAGE_SIZE;
    stat.shared_bytes = BPF_CORE_READ(task, mm, shared_vm) * PAGE_SIZE;
    stat.swap_bytes = BPF_CORE_READ(task, mm, swap_addresses) * PAGE_SIZE;
    stat.heap_usage = BPF_CORE_READ(task, mm, start_brk) - BPF_CORE_READ(task, mm, brk);
    stat.stack_usage = BPF_CORE_READ(task, mm, start_stack) - BPF_CORE_READ(task, mm, stack.vm_start);
    stat.anonymous_memory = BPF_CORE_READ(task, mm, rss_stat.count[MM_ANONPAGES]) * PAGE_SIZE;
    stat.file_backed_memory = BPF_CORE_READ(task, mm, rss_stat.count[MM_FILEPAGES]) * PAGE_SIZE;
    stat.major_faults = BPF_CORE_READ(task, maj_flt);
    stat.minor_faults = BPF_CORE_READ(task, min_flt);
    
    // Store statistics
    bpf_map_update_elem(&process_memory_stats, &pid, &stat, BPF_ANY);
    
    return 0;
}

char _license[] SEC("license") = "GPL";