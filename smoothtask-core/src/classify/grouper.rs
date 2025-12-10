//! Группировка процессов в AppGroup.
//!
//! Process Grouper анализирует список процессов и группирует их в AppGroup
//! на основе общих признаков: родительский процесс, cgroup, исполняемый файл.

use crate::logging::snapshots::{AppGroupRecord, ProcessRecord};
use std::collections::{HashMap, HashSet};

/// Группирует процессы в AppGroup.
///
/// Алгоритм группировки:
/// 1. Сначала группируем по cgroup_path (если есть)
/// 2. Затем по дереву процессов (root process и его потомки)
/// 3. Для процессов без cgroup и без родителя - группируем по exe
pub struct ProcessGrouper;

impl ProcessGrouper {
    /// Группирует процессы в AppGroup.
    ///
    /// # Аргументы
    ///
    /// * `processes` - список процессов для группировки
    ///
    /// # Возвращает
    ///
    /// Список AppGroupRecord с сгруппированными процессами.
    pub fn group_processes(processes: &[ProcessRecord]) -> Vec<AppGroupRecord> {
        if processes.is_empty() {
            return vec![];
        }

        // Строим индекс процессов по PID для быстрого поиска
        let pid_to_process: HashMap<i32, &ProcessRecord> =
            processes.iter().map(|p| (p.pid, p)).collect();

        // Шаг 1: Группировка по cgroup_path
        let mut cgroup_groups: HashMap<String, Vec<i32>> = HashMap::new();
        let mut processes_without_cgroup = Vec::new();

        for process in processes {
            if let Some(ref cgroup) = process.cgroup_path {
                // Нормализуем cgroup_path: берём только до user.slice или session.slice
                let normalized = Self::normalize_cgroup(cgroup);
                cgroup_groups
                    .entry(normalized)
                    .or_insert_with(Vec::new)
                    .push(process.pid);
            } else {
                processes_without_cgroup.push(process.pid);
            }
        }

        // Шаг 2: Для процессов без cgroup группируем по дереву процессов
        let mut process_to_group: HashMap<i32, String> = HashMap::new();
        let mut group_counter = 0u64;

        // Обрабатываем группы по cgroup
        for (_cgroup_path, pids) in &cgroup_groups {
            let group_id = format!("cgroup-{}", group_counter);
            group_counter += 1;

            // Внутри cgroup группируем по дереву процессов
            let tree_groups = Self::group_by_process_tree(pids, &pid_to_process);
            let tree_groups_len = tree_groups.len();
            for (tree_id, tree_pids) in tree_groups {
                let final_group_id = if tree_groups_len > 1 {
                    format!("{}-tree-{}", group_id, tree_id)
                } else {
                    group_id.clone()
                };

                for pid in tree_pids {
                    process_to_group.insert(pid, final_group_id.clone());
                }
            }
        }

        // Обрабатываем процессы без cgroup
        let tree_groups_no_cgroup =
            Self::group_by_process_tree(&processes_without_cgroup, &pid_to_process);
        for (_tree_id, tree_pids) in tree_groups_no_cgroup {
            let group_id = format!("tree-{}", group_counter);
            group_counter += 1;

            for pid in tree_pids {
                process_to_group.insert(pid, group_id.clone());
            }
        }

        // Шаг 3: Создаём AppGroupRecord из групп
        let mut groups_map: HashMap<String, Vec<i32>> = HashMap::new();
        for (pid, group_id) in process_to_group {
            groups_map
                .entry(group_id)
                .or_insert_with(Vec::new)
                .push(pid);
        }

        // Преобразуем в AppGroupRecord
        let mut app_groups = Vec::new();
        for (group_id, pids) in groups_map {
            // Находим root_pid (процесс с минимальным PID в группе, у которого нет родителя в группе)
            let root_pid = Self::find_root_pid(&pids, &pid_to_process);

            // Определяем app_name из exe корневого процесса
            let app_name = pid_to_process
                .get(&root_pid)
                .and_then(|p| p.exe.as_ref())
                .and_then(|exe| exe.split('/').last().map(|name| name.to_string()));

            // Агрегируем метрики группы
            let mut total_cpu_share = None;
            let mut total_io_read_bytes = Some(0u64);
            let mut total_io_write_bytes = Some(0u64);
            let mut total_rss_mb = Some(0u64);
            let mut has_gui_window = false;
            let mut is_focused_group = false;
            let mut tags_set = HashSet::new();

            for pid in &pids {
                if let Some(proc) = pid_to_process.get(pid) {
                    // CPU share - берём максимальное значение
                    if let Some(cpu) = proc.cpu_share_10s {
                        total_cpu_share =
                            Some(total_cpu_share.map(|t: f64| t.max(cpu)).unwrap_or(cpu));
                    }

                    // IO и память - суммируем
                    if let Some(io_read) = proc.io_read_bytes {
                        *total_io_read_bytes.as_mut().unwrap() += io_read;
                    }
                    if let Some(io_write) = proc.io_write_bytes {
                        *total_io_write_bytes.as_mut().unwrap() += io_write;
                    }
                    if let Some(rss) = proc.rss_mb {
                        *total_rss_mb.as_mut().unwrap() += rss;
                    }

                    // GUI и фокус
                    if proc.has_gui_window {
                        has_gui_window = true;
                    }
                    if proc.is_focused_window {
                        is_focused_group = true;
                    }

                    // Теги - объединяем
                    for tag in &proc.tags {
                        tags_set.insert(tag.clone());
                    }
                }
            }

            // Преобразуем суммы IO/RSS обратно в Option, если они остались 0
            let total_io_read_bytes = if total_io_read_bytes == Some(0) {
                None
            } else {
                total_io_read_bytes
            };
            let total_io_write_bytes = if total_io_write_bytes == Some(0) {
                None
            } else {
                total_io_write_bytes
            };
            let total_rss_mb = if total_rss_mb == Some(0) {
                None
            } else {
                total_rss_mb
            };

            app_groups.push(AppGroupRecord {
                app_group_id: group_id,
                root_pid,
                process_ids: pids,
                app_name,
                total_cpu_share,
                total_io_read_bytes,
                total_io_write_bytes,
                total_rss_mb,
                has_gui_window,
                is_focused_group,
                tags: tags_set.into_iter().collect(),
                priority_class: None, // будет заполнено позже в policy engine
            });
        }

        app_groups
    }

    /// Нормализует cgroup_path, оставляя только до user.slice, session.slice или app.slice.
    ///
    /// Функция обрезает cgroup_path, останавливаясь на первом вхождении одного из следующих элементов:
    /// - `session-*.scope` (например, `session-2.scope`)
    /// - `app.slice`
    /// - `system.slice` (останавливается сразу после него)
    ///
    /// # Аргументы
    ///
    /// * `cgroup_path` - путь к cgroup для нормализации
    ///
    /// # Возвращает
    ///
    /// Нормализованный путь к cgroup, обрезанный до соответствующего уровня.
    ///
    /// # Примеры
    ///
    /// ```
    /// use smoothtask_core::classify::grouper::ProcessGrouper;
    ///
    /// // Останавливается на session-*.scope
    /// assert_eq!(
    ///     ProcessGrouper::normalize_cgroup("/user.slice/user-1000.slice/session-2.scope/app"),
    ///     "/user.slice/user-1000.slice/session-2.scope"
    /// );
    ///
    /// // Останавливается на app.slice
    /// assert_eq!(
    ///     ProcessGrouper::normalize_cgroup("/user.slice/user-1000.slice/app.slice/firefox.service"),
    ///     "/user.slice/user-1000.slice/app.slice"
    /// );
    ///
    /// // Останавливается на system.slice
    /// assert_eq!(
    ///     ProcessGrouper::normalize_cgroup("/system.slice/systemd.service"),
    ///     "/system.slice"
    /// );
    ///
    /// // Если нет специальных элементов, возвращает весь путь
    /// assert_eq!(
    ///     ProcessGrouper::normalize_cgroup("/user.slice/user-1000.slice/custom.slice"),
    ///     "/user.slice/user-1000.slice/custom.slice"
    /// );
    /// ```
    ///
    /// # Примечания
    ///
    /// - Функция всегда возвращает путь, начинающийся с `/`
    /// - Пустые части пути (двойные слэши) игнорируются
    /// - Если путь не содержит специальных элементов, возвращается весь путь
    fn normalize_cgroup(cgroup_path: &str) -> String {
        let parts: Vec<&str> = cgroup_path.split('/').filter(|s| !s.is_empty()).collect();
        let mut normalized = Vec::new();

        for part in parts {
            normalized.push(part);
            // Останавливаемся на session-*.scope или app.slice
            if part.starts_with("session-") && part.ends_with(".scope") {
                break;
            }
            if part == "app.slice" {
                break;
            }
            // Для system.slice останавливаемся сразу после него
            if part == "system.slice" {
                break;
            }
        }

        format!("/{}", normalized.join("/"))
    }

    /// Группирует процессы по дереву процессов (root и потомки).
    ///
    /// Возвращает HashMap, где ключ - ID группы дерева, значение - список PID.
    fn group_by_process_tree(
        pids: &[i32],
        pid_to_process: &HashMap<i32, &ProcessRecord>,
    ) -> HashMap<usize, Vec<i32>> {
        let mut groups: HashMap<usize, Vec<i32>> = HashMap::new();
        let mut processed = HashSet::new();
        let mut group_id_counter = 0usize;

        for pid in pids {
            if processed.contains(pid) {
                continue;
            }

            // Находим root процесса (процесс, у которого родитель не в списке)
            let root_pid = Self::find_root_in_subset(*pid, pids, pid_to_process);

            // Собираем всех потомков root_pid в этой группе
            let mut group_pids = Vec::new();
            let mut to_process = vec![root_pid];
            let mut visited = HashSet::new();

            while let Some(current_pid) = to_process.pop() {
                if visited.contains(&current_pid) || !pids.contains(&current_pid) {
                    continue;
                }
                visited.insert(current_pid);
                group_pids.push(current_pid);
                processed.insert(current_pid);

                // Добавляем детей текущего процесса
                for (child_pid, child_proc) in pid_to_process.iter() {
                    if child_proc.ppid == current_pid && pids.contains(child_pid) {
                        to_process.push(*child_pid);
                    }
                }
            }

            if !group_pids.is_empty() {
                groups.insert(group_id_counter, group_pids);
                group_id_counter += 1;
            }
        }

        groups
    }

    /// Находит корневой процесс в подмножестве процессов.
    ///
    /// Корневой процесс - это процесс, у которого родитель либо отсутствует,
    /// либо не входит в данное подмножество.
    ///
    /// # Алгоритм
    ///
    /// Функция поднимается по дереву процессов от `start_pid` к родителям,
    /// пока не найдёт процесс, у которого родитель:
    /// - не входит в `subset`, или
    /// - является init (PID 1)
    ///
    /// # Аргументы
    ///
    /// * `start_pid` - PID процесса, с которого начинается поиск
    /// * `subset` - подмножество PID, в котором ищется root
    /// * `pid_to_process` - маппинг PID -> ProcessRecord для быстрого доступа
    ///
    /// # Возвращает
    ///
    /// PID корневого процесса в подмножестве.
    ///
    /// # Примеры
    ///
    /// ```
    /// // Если у процесса 1000 родитель 500, а у 500 родитель 1 (init),
    /// // то для subset [1000, 500] root будет 500
    /// ```
    fn find_root_in_subset(
        start_pid: i32,
        subset: &[i32],
        pid_to_process: &HashMap<i32, &ProcessRecord>,
    ) -> i32 {
        let mut current_pid = start_pid;
        let subset_set: HashSet<i32> = subset.iter().copied().collect();

        loop {
            if let Some(proc) = pid_to_process.get(&current_pid) {
                // Если родитель не в подмножестве или это init (PID 1), то это root
                if !subset_set.contains(&proc.ppid) || proc.ppid == 1 {
                    return current_pid;
                }
                current_pid = proc.ppid;
            } else {
                // Процесс не найден, возвращаем текущий
                return current_pid;
            }
        }
    }

    /// Находит root_pid для группы процессов.
    ///
    /// Root_pid - это процесс, у которого нет родителя в группе (или родитель - init).
    /// Если такого процесса нет, возвращается минимальный PID из группы.
    ///
    /// # Алгоритм
    ///
    /// 1. Проходим по всем PID в группе
    /// 2. Ищем процесс, у которого родитель не входит в группу или является init (PID 1)
    /// 3. Если такой процесс найден, возвращаем его PID
    /// 4. Если не найден, возвращаем минимальный PID из группы
    ///
    /// # Аргументы
    ///
    /// * `pids` - список PID процессов в группе
    /// * `pid_to_process` - маппинг PID -> ProcessRecord для быстрого доступа
    ///
    /// # Возвращает
    ///
    /// PID корневого процесса группы.
    ///
    /// # Примеры
    ///
    /// ```
    /// // Для группы [1000, 1001, 1002], где:
    /// // - 1000 имеет родителя 500 (не в группе)
    /// // - 1001 имеет родителя 1000 (в группе)
    /// // - 1002 имеет родителя 1000 (в группе)
    /// // root_pid будет 1000
    /// ```
    fn find_root_pid(pids: &[i32], pid_to_process: &HashMap<i32, &ProcessRecord>) -> i32 {
        let pids_set: HashSet<i32> = pids.iter().copied().collect();

        // Ищем процесс, у которого родитель не в группе
        for pid in pids {
            if let Some(proc) = pid_to_process.get(pid) {
                if !pids_set.contains(&proc.ppid) || proc.ppid == 1 {
                    return *pid;
                }
            }
        }

        // Если не нашли, возвращаем минимальный PID
        *pids.iter().min().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_process(
        pid: i32,
        ppid: i32,
        exe: Option<&str>,
        cgroup: Option<&str>,
    ) -> ProcessRecord {
        ProcessRecord {
            pid,
            ppid,
            uid: 1000,
            gid: 1000,
            exe: exe.map(|s| s.to_string()),
            cmdline: None,
            cgroup_path: cgroup.map(|s| s.to_string()),
            systemd_unit: None,
            app_group_id: None,
            state: "R".to_string(),
            start_time: 0,
            uptime_sec: 0,
            tty_nr: 0,
            has_tty: false,
            cpu_share_1s: None,
            cpu_share_10s: Some(0.1),
            io_read_bytes: Some(1000),
            io_write_bytes: Some(500),
            rss_mb: Some(50),
            swap_mb: None,
            voluntary_ctx: None,
            involuntary_ctx: None,
            has_gui_window: false,
            is_focused_window: false,
            window_state: None,
            env_has_display: false,
            env_has_wayland: false,
            env_term: None,
            env_ssh: false,
            is_audio_client: false,
            has_active_stream: false,
            process_type: None,
            tags: vec![],
            nice: 0,
            ionice_class: None,
            ionice_prio: None,
            teacher_priority_class: None,
            teacher_score: None,
        }
    }

    #[test]
    fn test_empty_processes() {
        let groups = ProcessGrouper::group_processes(&[]);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_single_process() {
        let processes = vec![create_test_process(100, 1, Some("/usr/bin/firefox"), None)];
        let groups = ProcessGrouper::group_processes(&processes);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].process_ids, vec![100]);
        assert_eq!(groups[0].root_pid, 100);
        assert_eq!(groups[0].app_name, Some("firefox".to_string()));
    }

    #[test]
    fn test_process_tree() {
        // Процессы: 100 (root) -> 101, 102 (дети)
        let processes = vec![
            create_test_process(100, 1, Some("/usr/bin/firefox"), None),
            create_test_process(101, 100, Some("/usr/bin/firefox"), None),
            create_test_process(102, 100, Some("/usr/bin/firefox"), None),
        ];

        let groups = ProcessGrouper::group_processes(&processes);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].root_pid, 100);
        assert_eq!(groups[0].process_ids.len(), 3);
        assert!(groups[0].process_ids.contains(&100));
        assert!(groups[0].process_ids.contains(&101));
        assert!(groups[0].process_ids.contains(&102));
    }

    #[test]
    fn test_cgroup_grouping() {
        // Процессы в одном cgroup должны быть сгруппированы
        let processes = vec![
            create_test_process(
                100,
                1,
                Some("/usr/bin/firefox"),
                Some("/user.slice/user-1000.slice/app.slice/firefox.service"),
            ),
            create_test_process(
                101,
                100,
                Some("/usr/bin/firefox"),
                Some("/user.slice/user-1000.slice/app.slice/firefox.service"),
            ),
        ];

        let groups = ProcessGrouper::group_processes(&processes);

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].process_ids.len(), 2);
    }

    #[test]
    fn test_multiple_cgroups() {
        // Процессы в разных cgroup должны быть в разных группах
        let processes = vec![
            create_test_process(
                100,
                1,
                Some("/usr/bin/firefox"),
                Some("/user.slice/user-1000.slice/app.slice/firefox.service"),
            ),
            create_test_process(
                200,
                1,
                Some("/usr/bin/chrome"),
                Some("/user.slice/user-1000.slice/app.slice/chrome.service"),
            ),
        ];

        let groups = ProcessGrouper::group_processes(&processes);

        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_cgroup_normalization() {
        // Проверяем нормализацию cgroup_path
        assert_eq!(
            ProcessGrouper::normalize_cgroup("/user.slice/user-1000.slice/session-2.scope/app"),
            "/user.slice/user-1000.slice/session-2.scope"
        );

        assert_eq!(
            ProcessGrouper::normalize_cgroup(
                "/user.slice/user-1000.slice/app.slice/firefox.service"
            ),
            "/user.slice/user-1000.slice/app.slice"
        );

        assert_eq!(
            ProcessGrouper::normalize_cgroup("/system.slice/systemd.service"),
            "/system.slice"
        );
    }

    #[test]
    fn test_cgroup_normalization_edge_cases() {
        // Пустой путь
        assert_eq!(ProcessGrouper::normalize_cgroup(""), "/");

        // Путь без слэшей
        assert_eq!(
            ProcessGrouper::normalize_cgroup("user.slice"),
            "/user.slice"
        );

        // Путь с множественными слэшами
        assert_eq!(
            ProcessGrouper::normalize_cgroup("//user.slice///user-1000.slice//app.slice"),
            "/user.slice/user-1000.slice/app.slice"
        );

        // Путь, который не содержит специальных элементов
        assert_eq!(
            ProcessGrouper::normalize_cgroup("/user.slice/user-1000.slice/custom.slice/service"),
            "/user.slice/user-1000.slice/custom.slice/service"
        );

        // Путь только с system.slice
        assert_eq!(
            ProcessGrouper::normalize_cgroup("/system.slice"),
            "/system.slice"
        );

        // Путь только с app.slice
        assert_eq!(ProcessGrouper::normalize_cgroup("/app.slice"), "/app.slice");

        // Путь с session-*.scope в середине
        assert_eq!(
            ProcessGrouper::normalize_cgroup("/user.slice/session-1.scope/app.slice/service"),
            "/user.slice/session-1.scope"
        );

        // Путь с несколькими session-*.scope (останавливается на первом)
        assert_eq!(
            ProcessGrouper::normalize_cgroup("/user.slice/session-1.scope/session-2.scope/service"),
            "/user.slice/session-1.scope"
        );

        // Путь с app.slice после system.slice (останавливается на system.slice)
        assert_eq!(
            ProcessGrouper::normalize_cgroup("/system.slice/app.slice/service"),
            "/system.slice"
        );

        // Путь с session-*.scope после app.slice (останавливается на app.slice)
        assert_eq!(
            ProcessGrouper::normalize_cgroup("/user.slice/app.slice/session-1.scope/service"),
            "/user.slice/app.slice"
        );
    }

    #[test]
    fn test_aggregated_metrics() {
        let mut proc1 = create_test_process(100, 1, Some("/usr/bin/firefox"), None);
        proc1.cpu_share_10s = Some(0.5);
        proc1.io_read_bytes = Some(1000);
        proc1.io_write_bytes = Some(500);
        proc1.rss_mb = Some(100);
        proc1.has_gui_window = true;
        proc1.is_focused_window = true;
        proc1.tags = vec!["browser".to_string()];

        let mut proc2 = create_test_process(101, 100, Some("/usr/bin/firefox"), None);
        proc2.cpu_share_10s = Some(0.3);
        proc2.io_read_bytes = Some(2000);
        proc2.io_write_bytes = Some(1000);
        proc2.rss_mb = Some(50);
        proc2.tags = vec!["renderer".to_string()];

        let processes = vec![proc1, proc2];
        let groups = ProcessGrouper::group_processes(&processes);

        assert_eq!(groups.len(), 1);
        let group = &groups[0];

        // CPU share - максимум
        assert_eq!(group.total_cpu_share, Some(0.5));

        // IO и память - сумма
        assert_eq!(group.total_io_read_bytes, Some(3000));
        assert_eq!(group.total_io_write_bytes, Some(1500));
        assert_eq!(group.total_rss_mb, Some(150));

        // GUI и фокус
        assert!(group.has_gui_window);
        assert!(group.is_focused_group);

        // Теги объединены
        assert_eq!(group.tags.len(), 2);
        assert!(group.tags.contains(&"browser".to_string()));
        assert!(group.tags.contains(&"renderer".to_string()));
    }

    #[test]
    fn test_multiple_independent_trees() {
        // Два независимых дерева процессов
        let processes = vec![
            create_test_process(100, 1, Some("/usr/bin/firefox"), None),
            create_test_process(101, 100, Some("/usr/bin/firefox"), None),
            create_test_process(200, 1, Some("/usr/bin/chrome"), None),
            create_test_process(201, 200, Some("/usr/bin/chrome"), None),
        ];

        let groups = ProcessGrouper::group_processes(&processes);

        assert_eq!(groups.len(), 2);

        // Проверяем, что процессы правильно разделены
        let firefox_group = groups
            .iter()
            .find(|g| g.process_ids.contains(&100))
            .unwrap();
        assert_eq!(firefox_group.process_ids.len(), 2);
        assert!(firefox_group.process_ids.contains(&100));
        assert!(firefox_group.process_ids.contains(&101));

        let chrome_group = groups
            .iter()
            .find(|g| g.process_ids.contains(&200))
            .unwrap();
        assert_eq!(chrome_group.process_ids.len(), 2);
        assert!(chrome_group.process_ids.contains(&200));
        assert!(chrome_group.process_ids.contains(&201));
    }
}
