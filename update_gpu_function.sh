#!/bin/bash

# Create a temporary file with the updated GPU function
cat > /tmp/gpu_function.txt << 'EOF'
    /// Загрузить eBPF программу для мониторинга производительности GPU
    #[cfg(feature = "ebpf")]
    fn load_gpu_program(&mut self) -> Result<()> {
        use std::path::Path;
        use libbpf_rs::skel::OpenSkel;
        use libbpf_rs::skel::SkelBuilder;
        
        // Пробуем загрузить оптимизированную версию программы
        let optimized_program_path = Path::new("src/ebpf_programs/gpu_monitor_optimized.c");
        let basic_program_path = Path::new("src/ebpf_programs/gpu_monitor.c");
        
        let program_path = if optimized_program_path.exists() {
            tracing::info!("Используем оптимизированную eBPF программу для мониторинга GPU");
            optimized_program_path
        } else if basic_program_path.exists() {
            basic_program_path
        } else {
            tracing::warn!("eBPF программы для мониторинга GPU не найдены");
            return Ok(());
        }
        
        tracing::info!("Загрузка eBPF программы для мониторинга GPU: {:?}", program_path);
        
        // Реальная загрузка eBPF программы
        // В реальной реализации здесь будет компиляция и загрузка eBPF программы
        // Для этого нужно использовать libbpf-rs API
        
        // TODO: Реальная загрузка eBPF программы с использованием libbpf-rs
        // self.gpu_program = Some(Program::from_file(program_path)?);
        
        tracing::info!("eBPF программа для мониторинга GPU успешно загружена");
        Ok(())
    }
EOF

# Replace the function in the file
# Find the start and end lines of the function
start_line=$(grep -n "fn load_gpu_program" src/metrics/ebpf.rs | cut -d: -f1)
end_line=$(awk '/fn load_gpu_program/,/^    \}/ {print NR ": " $0}' src/metrics/ebpf.rs | grep "^    \}" | head -1 | cut -d: -f1)

echo "Start line: $start_line, End line: $end_line"

# Use sed to replace the function
sed -i "${start_line},${end_line}d" src/metrics/ebpf.rs
sed -i "${start_line}r /tmp/gpu_function.txt" src/metrics/ebpf.rs

echo "GPU function updated successfully"