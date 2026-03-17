#![allow(dead_code)]
#![allow(clippy::enum_variant_names)]
use rusqlite::{params, Connection, Result};

use serde::{Deserialize, Serialize};

// ── Data Models ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub goal: String,
    pub status: String,
    pub progress: i32,
    pub current_task: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub project_id: i64,
    pub description: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    pub id: i64,
    pub project_id: i64,
    pub date: String,
    pub notes: String,
    pub next_action: String,
    pub created_at: String,
}
// ── Database Initialization ──────────────────────────────────

pub fn initialize_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            goal TEXT DEFAULT '',
            status TEXT DEFAULT 'active',
            progress INTEGER DEFAULT 0,
            current_task TEXT DEFAULT '',
            created_at DATETIME DEFAULT (datetime('now','localtime')),
            updated_at DATETIME DEFAULT (datetime('now','localtime'))
        );

        CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL,
            description TEXT NOT NULL,
            status TEXT DEFAULT 'todo',
            created_at DATETIME DEFAULT (datetime('now','localtime')),
            FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL,
            date DATE DEFAULT (date('now','localtime')),
            notes TEXT DEFAULT '',
            next_action TEXT DEFAULT '',
            created_at DATETIME DEFAULT (datetime('now','localtime')),
            FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
        );
        ",
    )?;
    // Enable foreign keys
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(())
}

pub fn open_db(path: &str) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    initialize_db(&conn)?;
    Ok(conn)
}

// ── Project CRUD ─────────────────────────────────────────────

pub fn create_project(conn: &Connection, name: &str, goal: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO projects (name, goal) VALUES (?1, ?2)",
        params![name, goal],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_all_projects(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, goal, status, progress, current_task, created_at, updated_at
         FROM projects WHERE status != 'archived' ORDER BY updated_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            goal: row.get(2)?,
            status: row.get(3)?,
            progress: row.get(4)?,
            current_task: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;
    rows.collect()
}

pub fn get_project(conn: &Connection, id: i64) -> Result<Project> {
    conn.query_row(
        "SELECT id, name, goal, status, progress, current_task, created_at, updated_at
         FROM projects WHERE id = ?1",
        params![id],
        |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                goal: row.get(2)?,
                status: row.get(3)?,
                progress: row.get(4)?,
                current_task: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )
}

pub fn update_project(
    conn: &Connection,
    id: i64,
    name: &str,
    goal: &str,
    status: &str,
    progress: i32,
    current_task: &str,
) -> Result<()> {
    conn.execute(
        "UPDATE projects SET name=?1, goal=?2, status=?3, progress=?4, current_task=?5,
         updated_at=datetime('now','localtime') WHERE id=?6",
        params![name, goal, status, progress, current_task, id],
    )?;
    Ok(())
}

pub fn archive_project(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE projects SET status='archived', updated_at=datetime('now','localtime') WHERE id=?1",
        params![id],
    )?;
    Ok(())
}

pub fn delete_project(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM projects WHERE id=?1", params![id])?;
    Ok(())
}

fn touch_project(conn: &Connection, project_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE projects SET updated_at=datetime('now','localtime') WHERE id=?1",
        params![project_id],
    )?;
    Ok(())
}

// ── Task CRUD ────────────────────────────────────────────────

pub fn create_task(conn: &Connection, project_id: i64, description: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO tasks (project_id, description) VALUES (?1, ?2)",
        params![project_id, description],
    )?;
    touch_project(conn, project_id)?;
    Ok(conn.last_insert_rowid())
}

pub fn get_tasks_for_project(conn: &Connection, project_id: i64) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, description, status, created_at
         FROM tasks WHERE project_id=?1 ORDER BY status ASC, created_at ASC",
    )?;
    let rows = stmt.query_map(params![project_id], |row| {
        Ok(Task {
            id: row.get(0)?,
            project_id: row.get(1)?,
            description: row.get(2)?,
            status: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;
    rows.collect()
}

pub fn toggle_task_status(conn: &Connection, task_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE tasks SET status = CASE WHEN status='todo' THEN 'done' ELSE 'todo' END WHERE id=?1",
        params![task_id],
    )?;
    // Touch parent project
    let project_id: i64 = conn.query_row(
        "SELECT project_id FROM tasks WHERE id=?1",
        params![task_id],
        |row| row.get(0),
    )?;
    touch_project(conn, project_id)?;
    Ok(())
}

pub fn delete_task(conn: &Connection, task_id: i64) -> Result<()> {
    let project_id: i64 = conn.query_row(
        "SELECT project_id FROM tasks WHERE id=?1",
        params![task_id],
        |row| row.get(0),
    )?;
    conn.execute("DELETE FROM tasks WHERE id=?1", params![task_id])?;
    touch_project(conn, project_id)?;
    Ok(())
}

// ── Log CRUD ─────────────────────────────────────────────────

pub fn create_log(
    conn: &Connection,
    project_id: i64,
    notes: &str,
    next_action: &str,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO logs (project_id, notes, next_action) VALUES (?1, ?2, ?3)",
        params![project_id, notes, next_action],
    )?;
    touch_project(conn, project_id)?;
    Ok(conn.last_insert_rowid())
}

pub fn get_logs_for_project(conn: &Connection, project_id: i64) -> Result<Vec<Log>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, date, notes, next_action, created_at
         FROM logs WHERE project_id=?1 ORDER BY id DESC",
    )?;
    let rows = stmt.query_map(params![project_id], |row| {
        Ok(Log {
            id: row.get(0)?,
            project_id: row.get(1)?,
            date: row.get(2)?,
            notes: row.get(3)?,
            next_action: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;
    rows.collect()
}

pub fn get_latest_log(conn: &Connection, project_id: i64) -> Result<Option<Log>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, date, notes, next_action, created_at
         FROM logs WHERE project_id=?1 ORDER BY id DESC LIMIT 1",
    )?;
    let mut rows = stmt.query_map(params![project_id], |row| {
        Ok(Log {
            id: row.get(0)?,
            project_id: row.get(1)?,
            date: row.get(2)?,
            notes: row.get(3)?,
            next_action: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;
    match rows.next() {
        Some(log) => Ok(Some(log?)),
        None => Ok(None),
    }
}

pub fn delete_log(conn: &Connection, log_id: i64) -> Result<()> {
    conn.execute("DELETE FROM logs WHERE id=?1", params![log_id])?;
    Ok(())
}

// ── Search ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum SearchResult {
    ProjectResult(Project),
    TaskResult(Task),
    LogResult(Log),
}

pub fn search_all(conn: &Connection, query: &str) -> Result<Vec<SearchResult>> {
    let pattern = format!("%{query}%");
    let mut results = Vec::new();

    // Search projects
    {
        let mut stmt = conn.prepare(
            "SELECT id, name, goal, status, progress, current_task, created_at, updated_at
             FROM projects WHERE (name LIKE ?1 OR goal LIKE ?1) AND status != 'archived'",
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                goal: row.get(2)?,
                status: row.get(3)?,
                progress: row.get(4)?,
                current_task: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        for row in rows {
            results.push(SearchResult::ProjectResult(row?));
        }
    }

    // Search tasks
    {
        let mut stmt = conn.prepare(
            "SELECT t.id, t.project_id, t.description, t.status, t.created_at
             FROM tasks t JOIN projects p ON t.project_id = p.id
             WHERE t.description LIKE ?1 AND p.status != 'archived'",
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            Ok(Task {
                id: row.get(0)?,
                project_id: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        for row in rows {
            results.push(SearchResult::TaskResult(row?));
        }
    }

    // Search logs
    {
        let mut stmt = conn.prepare(
            "SELECT l.id, l.project_id, l.date, l.notes, l.next_action, l.created_at
             FROM logs l JOIN projects p ON l.project_id = p.id
             WHERE (l.notes LIKE ?1 OR l.next_action LIKE ?1) AND p.status != 'archived'",
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            Ok(Log {
                id: row.get(0)?,
                project_id: row.get(1)?,
                date: row.get(2)?,
                notes: row.get(3)?,
                next_action: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        for row in rows {
            results.push(SearchResult::LogResult(row?));
        }
    }

    Ok(results)
}

// ── Export / Import ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub projects: Vec<Project>,
    pub tasks: Vec<Task>,
    pub logs: Vec<Log>,
}

pub fn get_all_projects_including_archived(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, goal, status, progress, current_task, created_at, updated_at
         FROM projects ORDER BY updated_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            goal: row.get(2)?,
            status: row.get(3)?,
            progress: row.get(4)?,
            current_task: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;
    rows.collect()
}

pub fn export_all_to_json(conn: &Connection) -> Result<String, Box<dyn std::error::Error>> {
    let projects = get_all_projects_including_archived(conn)?;
    let mut tasks = Vec::new();
    let mut logs = Vec::new();
    for p in &projects {
        let p_tasks = get_tasks_for_project(conn, p.id)?;
        tasks.extend(p_tasks);
        let p_logs = get_logs_for_project(conn, p.id)?;
        logs.extend(p_logs);
    }
    let data = ExportData { projects, tasks, logs };
    Ok(serde_json::to_string_pretty(&data)?)
}

pub fn import_all_from_json(conn: &Connection, json: &str) -> Result<(), Box<dyn std::error::Error>> {
    let data: ExportData = serde_json::from_str(json)?;
    conn.execute_batch("BEGIN TRANSACTION;")?;
    for p in data.projects {
        let old_id = p.id;
        conn.execute(
            "INSERT INTO projects (name, goal, status, progress, current_task, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![p.name, p.goal, p.status, p.progress, p.current_task, p.created_at, p.updated_at],
        )?;
        let new_id = conn.last_insert_rowid();
        
        for t in data.tasks.iter().filter(|t| t.project_id == old_id) {
            conn.execute(
                "INSERT INTO tasks (project_id, description, status, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![new_id, t.description, t.status, t.created_at],
            )?;
        }
        
        for l in data.logs.iter().filter(|l| l.project_id == old_id) {
            conn.execute(
                "INSERT INTO logs (project_id, date, notes, next_action, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                 params![new_id, l.date, l.notes, l.next_action, l.created_at],
            )?;
        }
    }
    conn.execute_batch("COMMIT;")?;
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        initialize_db(&conn).unwrap();
        conn
    }

    #[test]
    fn test_create_and_get_project() {
        let conn = setup();
        let id = create_project(&conn, "Test Project", "Build something").unwrap();
        let project = get_project(&conn, id).unwrap();
        assert_eq!(project.name, "Test Project");
        assert_eq!(project.goal, "Build something");
        assert_eq!(project.status, "active");
        assert_eq!(project.progress, 0);
    }

    #[test]
    fn test_get_all_projects_excludes_archived() {
        let conn = setup();
        create_project(&conn, "Active", "").unwrap();
        let id2 = create_project(&conn, "Archived", "").unwrap();
        archive_project(&conn, id2).unwrap();
        let projects = get_all_projects(&conn).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "Active");
    }

    #[test]
    fn test_update_project() {
        let conn = setup();
        let id = create_project(&conn, "Old", "old goal").unwrap();
        update_project(&conn, id, "New", "new goal", "active", 50, "current").unwrap();
        let project = get_project(&conn, id).unwrap();
        assert_eq!(project.name, "New");
        assert_eq!(project.goal, "new goal");
        assert_eq!(project.progress, 50);
        assert_eq!(project.current_task, "current");
    }

    #[test]
    fn test_delete_project() {
        let conn = setup();
        let id = create_project(&conn, "Del", "").unwrap();
        delete_project(&conn, id).unwrap();
        assert!(get_project(&conn, id).is_err());
    }

    #[test]
    fn test_create_and_get_tasks() {
        let conn = setup();
        let pid = create_project(&conn, "P", "").unwrap();
        create_task(&conn, pid, "Task 1").unwrap();
        create_task(&conn, pid, "Task 2").unwrap();
        let tasks = get_tasks_for_project(&conn, pid).unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].description, "Task 1");
        assert_eq!(tasks[0].status, "todo");
    }

    #[test]
    fn test_toggle_task_status() {
        let conn = setup();
        let pid = create_project(&conn, "P", "").unwrap();
        let tid = create_task(&conn, pid, "Task").unwrap();
        toggle_task_status(&conn, tid).unwrap();
        let tasks = get_tasks_for_project(&conn, pid).unwrap();
        assert_eq!(tasks.iter().find(|t| t.id == tid).unwrap().status, "done");
        toggle_task_status(&conn, tid).unwrap();
        let tasks = get_tasks_for_project(&conn, pid).unwrap();
        assert_eq!(tasks.iter().find(|t| t.id == tid).unwrap().status, "todo");
    }

    #[test]
    fn test_delete_task() {
        let conn = setup();
        let pid = create_project(&conn, "P", "").unwrap();
        let tid = create_task(&conn, pid, "Task").unwrap();
        delete_task(&conn, tid).unwrap();
        let tasks = get_tasks_for_project(&conn, pid).unwrap();
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_create_and_get_logs() {
        let conn = setup();
        let pid = create_project(&conn, "P", "").unwrap();
        create_log(&conn, pid, "Did work", "Next step").unwrap();
        let logs = get_logs_for_project(&conn, pid).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].notes, "Did work");
        assert_eq!(logs[0].next_action, "Next step");
    }

    #[test]
    fn test_get_latest_log() {
        let conn = setup();
        let pid = create_project(&conn, "P", "").unwrap();
        create_log(&conn, pid, "First", "").unwrap();
        create_log(&conn, pid, "Second", "next").unwrap();
        let latest = get_latest_log(&conn, pid).unwrap().unwrap();
        assert_eq!(latest.notes, "Second");
    }

    #[test]
    fn test_get_latest_log_none() {
        let conn = setup();
        let pid = create_project(&conn, "P", "").unwrap();
        let latest = get_latest_log(&conn, pid).unwrap();
        assert!(latest.is_none());
    }

    #[test]
    fn test_delete_log() {
        let conn = setup();
        let pid = create_project(&conn, "P", "").unwrap();
        let lid = create_log(&conn, pid, "note", "").unwrap();
        delete_log(&conn, lid).unwrap();
        let logs = get_logs_for_project(&conn, pid).unwrap();
        assert!(logs.is_empty());
    }

    #[test]
    fn test_search_projects() {
        let conn = setup();
        create_project(&conn, "Rust AI Tool", "build AI").unwrap();
        create_project(&conn, "Web App", "frontend").unwrap();
        let results = search_all(&conn, "AI").unwrap();
        assert_eq!(results.len(), 1);
        match &results[0] {
            SearchResult::ProjectResult(p) => assert_eq!(p.name, "Rust AI Tool"),
            _ => panic!("Expected ProjectResult"),
        }
    }

    #[test]
    fn test_search_tasks() {
        let conn = setup();
        let pid = create_project(&conn, "P", "").unwrap();
        create_task(&conn, pid, "Implement login").unwrap();
        create_task(&conn, pid, "Design page").unwrap();
        let results = search_all(&conn, "login").unwrap();
        assert_eq!(results.len(), 1);
        match &results[0] {
            SearchResult::TaskResult(t) => assert_eq!(t.description, "Implement login"),
            _ => panic!("Expected TaskResult"),
        }
    }

    #[test]
    fn test_search_logs() {
        let conn = setup();
        let pid = create_project(&conn, "P", "").unwrap();
        create_log(&conn, pid, "Fixed authentication bug", "Deploy").unwrap();
        let results = search_all(&conn, "authentication").unwrap();
        assert_eq!(results.len(), 1);
        match &results[0] {
            SearchResult::LogResult(l) => assert!(l.notes.contains("authentication")),
            _ => panic!("Expected LogResult"),
        }
    }

    #[test]
    fn test_cascade_delete() {
        let conn = setup();
        let pid = create_project(&conn, "P", "").unwrap();
        create_task(&conn, pid, "Task").unwrap();
        create_log(&conn, pid, "Log", "").unwrap();
        delete_project(&conn, pid).unwrap();
        let tasks = get_tasks_for_project(&conn, pid).unwrap();
        let logs = get_logs_for_project(&conn, pid).unwrap();
        assert!(tasks.is_empty());
        assert!(logs.is_empty());
    }

    #[test]
    fn test_query_performance() {
        let conn = setup();
        // Insert 100 projects
        for i in 0..100 {
            create_project(&conn, &format!("Project {i}"), &format!("goal {i}")).unwrap();
        }
        let start = std::time::Instant::now();
        let _projects = get_all_projects(&conn).unwrap();
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 10,
            "Query took {}ms, should be <10ms",
            elapsed.as_millis()
        );
    }
}
