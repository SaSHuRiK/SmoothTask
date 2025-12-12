//! Пример использования утилитных функций для работы с cgroups v2.
//!
//! Этот пример демонстрирует, как использовать функции из модуля utils::cgroups
//! для работы с cgroups v2 в системе.

use smoothtask_core::utils::cgroups::*;

fn main() -> anyhow::Result<()> {
    println!("=== SmoothTask cgroups v2 Utilities Example ===\n");

    // 1. Проверка доступности cgroups v2
    println!("1. Checking cgroups v2 availability...");
    if is_cgroup_v2_available() {
        println!("   ✓ cgroups v2 is available");
    } else {
        println!("   ✗ cgroups v2 is not available");
        return Ok(()); // Завершаем, если cgroups v2 недоступен
    }

    // 2. Получение корня cgroup v2
    println!("\n2. Getting cgroup root...");
    let cgroup_root = get_cgroup_root();
    println!("   cgroup root: {:?}", cgroup_root);

    // 3. Проверка доступности контроллеров
    println!("\n3. Checking controller availability...");
    let controllers = ["cpu", "memory", "io", "pids"];
    for controller in controllers {
        if is_controller_available(controller) {
            println!("   ✓ Controller '{}' is available", controller);
        } else {
            println!("   ✗ Controller '{}' is not available", controller);
        }
    }

    // 4. Создание cgroup для приложения
    println!("\n4. Creating app cgroup...");
    let app_group_id = "example-app";
    let app_cgroup_path = create_app_cgroup(app_group_id)?;
    println!("   Created cgroup at: {:?}", app_cgroup_path);

    // 5. Чтение и запись параметров cgroup
    println!("\n5. Reading and writing cgroup parameters...");
    
    // Запись cpu.weight
    write_cgroup_param(&app_cgroup_path, "cpu.weight", "200")?;
    println!("   ✓ Set cpu.weight to 200");
    
    // Чтение cpu.weight
    if let Some(weight) = read_cgroup_param(&app_cgroup_path, "cpu.weight")? {
        println!("   ✓ Read cpu.weight: {}", weight);
    }

    // 6. Управление процессами в cgroup
    println!("\n6. Managing processes in cgroup...");
    
    // Получение текущего PID
    let current_pid = std::process::id() as i32;
    println!("   Current PID: {}", current_pid);
    
    // Проверка, находится ли процесс в cgroup
    let in_cgroup = is_process_in_cgroup(current_pid, &app_cgroup_path)?;
    println!("   Process {} in cgroup: {}", current_pid, in_cgroup);
    
    // Получение списка процессов в cgroup
    let processes = get_processes_in_cgroup(&app_cgroup_path)?;
    println!("   Processes in cgroup: {:?}", processes);

    // 7. Удаление cgroup (если пустой)
    println!("\n7. Cleaning up...");
    let removed = remove_cgroup_if_empty(&app_cgroup_path)?;
    if removed {
        println!("   ✓ Removed empty cgroup");
    } else {
        println!("   ℹ Cgroup not empty or already removed");
    }

    println!("\n=== Example completed successfully! ===");
    Ok(())
}