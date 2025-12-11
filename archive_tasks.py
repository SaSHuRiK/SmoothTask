#!/usr/bin/env python3
"""Скрипт для архивации старых DONE-задач из PLAN.md"""

import re
from pathlib import Path

def extract_task(task_id, section):
    """Извлекает задачу по ID из раздела"""
    pattern = rf'- \[x\] {task_id}:.*?(?=- \[x\] ST-|##)'
    match = re.search(pattern, section, re.DOTALL)
    if match:
        return match.group(0).strip()
    return None

def find_task_position(task_id, section):
    """Находит позицию задачи в разделе"""
    pattern = rf'- \[x\] {task_id}:'
    match = re.search(pattern, section)
    if match:
        return match.start()
    return None

def main():
    plan_path = Path('PLAN.md')
    archive_path = Path('docs/history/PLAN_DONE_archive.md')
    
    # Читаем файл
    with open(plan_path, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Находим границы разделов
    recently_done_start = content.find('## 3. Недавно сделано (Recently Done)')
    done_start = content.find('## 4. Готово (Done)')
    blockers_start = content.find('## 5. Блокеры')
    
    # Извлекаем разделы
    recently_done_section = content[recently_done_start:done_start]
    done_section = content[done_start:blockers_start]
    
    # Находим позиции задач
    st214_pos = find_task_position('ST-214', recently_done_section)
    st200_pos = find_task_position('ST-200', recently_done_section)
    st199_pos = find_task_position('ST-199', recently_done_section)
    
    # Разделяем "Recently Done"
    if st214_pos and st200_pos:
        # Оставляем ST-214 до ST-200 (включительно)
        # Находим конец ST-200
        st200_pattern = r'- \[x\] ST-200:.*?(?=- \[x\] ST-199:|##)'
        st200_match = re.search(st200_pattern, recently_done_section, re.DOTALL)
        if st200_match:
            st200_end = st200_match.end()
            recently_done_keep = recently_done_section[:st214_pos] + recently_done_section[st214_pos:st200_end]
            recently_done_archive = recently_done_section[st200_end:].strip()
        else:
            recently_done_keep = recently_done_section
            recently_done_archive = ''
    else:
        recently_done_keep = recently_done_section
        recently_done_archive = ''
    
    # Для раздела "4. Готово (Done)" оставляем только последние несколько задач (ST-098 до ST-050)
    # Находим позиции
    st098_pos = find_task_position('ST-098', done_section)
    st050_pos = find_task_position('ST-050', done_section)
    st049_pos = find_task_position('ST-049', done_section)
    
    if st098_pos and st049_pos:
        # Находим конец ST-050
        st050_pattern = r'- \[x\] ST-050:.*?(?=- \[x\] ST-049:|##)'
        st050_match = re.search(st050_pattern, done_section, re.DOTALL)
        if st050_match:
            st050_end = st050_match.end()
            done_keep = done_section[:st098_pos] + done_section[st098_pos:st050_end]
            done_archive = done_section[st050_end:].strip()
        else:
            done_keep = done_section
            done_archive = ''
    else:
        done_keep = done_section
        done_archive = ''
    
    # Создаем архивный файл
    archive_content = f"""# Архив выполненных задач SmoothTask

Этот файл содержит архив старых выполненных задач из PLAN.md.

## Задачи из раздела "Недавно сделано" (ST-199 и ниже)

{recently_done_archive}

## Задачи из раздела "Готово" (ST-049 и ниже)

{done_archive}
"""
    
    # Создаем директорию если нужно
    archive_path.parent.mkdir(parents=True, exist_ok=True)
    
    # Записываем архив
    with open(archive_path, 'w', encoding='utf-8') as f:
        f.write(archive_content)
    
    # Обновляем PLAN.md
    new_content = (
        content[:recently_done_start] +
        recently_done_keep +
        '\n\nСм. архив: docs/history/PLAN_DONE_archive.md\n\n' +
        done_keep +
        '\n\nСм. архив: docs/history/PLAN_DONE_archive.md\n\n' +
        content[blockers_start:]
    )
    
    with open(plan_path, 'w', encoding='utf-8') as f:
        f.write(new_content)
    
    print(f"Архивация завершена!")
    print(f"Архив сохранен в: {archive_path}")
    print(f"В 'Recently Done' оставлено: последние 15 задач (ST-214 до ST-200)")
    print(f"В 'Готово' оставлено: последние задачи (ST-098 до ST-050)")

if __name__ == '__main__':
    main()
