use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, State};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WorktreeItem {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub branch: Option<String>,
    pub path: String,
    #[serde(default)]
    pub is_stale: bool,
    #[serde(default)]
    pub stale_reason: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ProjectItem {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub is_stale: bool,
    #[serde(default)]
    pub stale_reason: Option<String>,
    #[serde(default)]
    pub worktrees: Vec<WorktreeItem>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TerminalTabItem {
    pub id: String,
    pub title: String,
    pub cwd: String,
    pub project_name: Option<String>,
    pub worktree_name: Option<String>,
    pub tab_order: i32,
    #[serde(default)]
    pub is_stale: bool,
    #[serde(default)]
    pub stale_reason: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LayoutState {
    pub show_left: bool,
    pub show_right: bool,
    pub active_tab_id: Option<String>,
    pub selected_project_name: Option<String>,
    pub selected_worktree_name: Option<String>,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            show_left: true,
            show_right: true,
            active_tab_id: None,
            selected_project_name: None,
            selected_worktree_name: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WorkspaceState {
    pub layout: LayoutState,
    pub projects: Vec<ProjectItem>,
    pub tabs: Vec<TerminalTabItem>,
}

pub struct DatabaseManager {
    conn: Arc<Mutex<Connection>>,
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

impl DatabaseManager {
    pub fn new(db_path: &Path) -> Result<Self, rusqlite::Error> {
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let conn = Connection::open(db_path)?;
        Self::init_schema(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    #[allow(dead_code)]
    pub fn new_in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        Self::init_schema(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn init_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
        // Optimize for concurrent local reads and crash resilience
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS workspace_layout (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                show_left INTEGER NOT NULL DEFAULT 1,
                show_right INTEGER NOT NULL DEFAULT 1,
                active_tab_id TEXT,
                selected_project_name TEXT,
                selected_worktree_name TEXT,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                path TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS worktrees (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                name TEXT NOT NULL,
                branch TEXT,
                path TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS terminal_tabs (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                cwd TEXT NOT NULL,
                project_name TEXT,
                worktree_name TEXT,
                tab_order INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            ",
        )?;

        // Seed default layout row if empty
        conn.execute(
            "INSERT OR IGNORE INTO workspace_layout (id, show_left, show_right, updated_at)
             VALUES (1, 1, 1, ?1);",
            params![current_timestamp()],
        )?;

        Ok(())
    }

    pub fn get_workspace_state(&self) -> Result<WorkspaceState, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Database lock poisoned: {}", e))?;

        // 1. Fetch layout
        let layout = conn
            .query_row(
                "SELECT show_left, show_right, active_tab_id, selected_project_name, selected_worktree_name 
                 FROM workspace_layout WHERE id = 1;",
                [],
                |row| {
                    let show_left: i32 = row.get(0)?;
                    let show_right: i32 = row.get(1)?;
                    let active_tab_id: Option<String> = row.get(2)?;
                    let selected_project_name: Option<String> = row.get(3)?;
                    let selected_worktree_name: Option<String> = row.get(4)?;

                    Ok(LayoutState {
                        show_left: show_left != 0,
                        show_right: show_right != 0,
                        active_tab_id,
                        selected_project_name,
                        selected_worktree_name,
                    })
                },
            )
            .optional()
            .map_err(|e| format!("Failed to read layout: {}", e))?
            .unwrap_or_default();

        // 2. Fetch projects and worktrees with path reconciliation
        let mut projects = Vec::new();
        let mut proj_stmt = conn
            .prepare("SELECT id, name, path FROM projects ORDER BY name ASC;")
            .map_err(|e| format!("Failed to prepare projects query: {}", e))?;

        let proj_rows = proj_stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let path: String = row.get(2)?;
                Ok((id, name, path))
            })
            .map_err(|e| format!("Failed to query projects: {}", e))?;

        for proj_res in proj_rows {
            let (proj_id, name, path) =
                proj_res.map_err(|e| format!("Failed to parse project row: {}", e))?;
            let path_exists = Path::new(&path).exists();
            let is_stale = !path_exists;
            let stale_reason = if is_stale {
                Some("Project directory not found on filesystem".to_string())
            } else {
                None
            };

            // Fetch worktrees for this project
            let mut wt_stmt = conn
                .prepare(
                    "SELECT id, name, branch, path FROM worktrees WHERE project_id = ?1 ORDER BY name ASC;",
                )
                .map_err(|e| format!("Failed to prepare worktrees query: {}", e))?;

            let wt_rows = wt_stmt
                .query_map(params![proj_id], |row| {
                    let wt_id: String = row.get(0)?;
                    let wt_name: String = row.get(1)?;
                    let branch: Option<String> = row.get(2)?;
                    let wt_path: String = row.get(3)?;
                    Ok((wt_id, wt_name, branch, wt_path))
                })
                .map_err(|e| format!("Failed to query worktrees: {}", e))?;

            let mut worktrees = Vec::new();
            for wt_res in wt_rows {
                let (wt_id, wt_name, branch, wt_path) =
                    wt_res.map_err(|e| format!("Failed to parse worktree row: {}", e))?;
                let wt_exists = Path::new(&wt_path).exists();
                let wt_is_stale = !wt_exists;
                let wt_stale_reason = if wt_is_stale {
                    Some("Worktree directory not found or removed".to_string())
                } else {
                    None
                };

                worktrees.push(WorktreeItem {
                    id: wt_id,
                    project_id: proj_id.clone(),
                    name: wt_name,
                    branch,
                    path: wt_path,
                    is_stale: wt_is_stale,
                    stale_reason: wt_stale_reason,
                });
            }

            projects.push(ProjectItem {
                id: proj_id,
                name,
                path,
                is_stale,
                stale_reason,
                worktrees,
            });
        }

        // 3. Fetch terminal tabs with path reconciliation
        let mut tabs = Vec::new();
        let mut tab_stmt = conn
            .prepare(
                "SELECT id, title, cwd, project_name, worktree_name, tab_order 
                 FROM terminal_tabs ORDER BY tab_order ASC, created_at ASC;",
            )
            .map_err(|e| format!("Failed to prepare terminal tabs query: {}", e))?;

        let tab_rows = tab_stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let cwd: String = row.get(2)?;
                let project_name: Option<String> = row.get(3)?;
                let worktree_name: Option<String> = row.get(4)?;
                let tab_order: i32 = row.get(5)?;
                Ok((id, title, cwd, project_name, worktree_name, tab_order))
            })
            .map_err(|e| format!("Failed to query terminal tabs: {}", e))?;

        for tab_res in tab_rows {
            let (id, title, cwd, project_name, worktree_name, tab_order) =
                tab_res.map_err(|e| format!("Failed to parse tab row: {}", e))?;

            let cwd_exists = Path::new(&cwd).exists();
            let is_stale = !cwd_exists;
            let stale_reason = if is_stale {
                Some("Remembered working directory does not exist".to_string())
            } else {
                None
            };

            tabs.push(TerminalTabItem {
                id,
                title,
                cwd,
                project_name,
                worktree_name,
                tab_order,
                is_stale,
                stale_reason,
            });
        }

        Ok(WorkspaceState {
            layout,
            projects,
            tabs,
        })
    }

    pub fn save_layout(&self, layout: LayoutState) -> Result<(), String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Database lock poisoned: {}", e))?;

        let now = current_timestamp();
        conn.execute(
            "INSERT INTO workspace_layout (id, show_left, show_right, active_tab_id, selected_project_name, selected_worktree_name, updated_at)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                show_left = excluded.show_left,
                show_right = excluded.show_right,
                active_tab_id = excluded.active_tab_id,
                selected_project_name = excluded.selected_project_name,
                selected_worktree_name = excluded.selected_worktree_name,
                updated_at = excluded.updated_at;",
            params![
                if layout.show_left { 1 } else { 0 },
                if layout.show_right { 1 } else { 0 },
                layout.active_tab_id,
                layout.selected_project_name,
                layout.selected_worktree_name,
                now
            ],
        )
        .map_err(|e| format!("Failed to save layout state: {}", e))?;

        Ok(())
    }

    pub fn save_projects(&self, projects: &[ProjectItem]) -> Result<(), String> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| format!("Database lock poisoned: {}", e))?;

        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to begin projects transaction: {}", e))?;

        let now = current_timestamp();

        // Clear existing projects & worktrees to replace with synced list
        tx.execute("DELETE FROM projects;", [])
            .map_err(|e| format!("Failed to clear projects: {}", e))?;

        for p in projects {
            tx.execute(
                "INSERT INTO projects (id, name, path, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5);",
                params![p.id, p.name, p.path, now, now],
            )
            .map_err(|e| format!("Failed to insert project {}: {}", p.name, e))?;

            for wt in &p.worktrees {
                tx.execute(
                    "INSERT INTO worktrees (id, project_id, name, branch, path, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);",
                    params![wt.id, p.id, wt.name, wt.branch, wt.path, now, now],
                )
                .map_err(|e| format!("Failed to insert worktree {}: {}", wt.name, e))?;
            }
        }

        tx.commit()
            .map_err(|e| format!("Failed to commit projects transaction: {}", e))?;

        Ok(())
    }

    pub fn save_tabs(&self, tabs: &[TerminalTabItem]) -> Result<(), String> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| format!("Database lock poisoned: {}", e))?;

        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to begin tabs transaction: {}", e))?;

        let now = current_timestamp();

        tx.execute("DELETE FROM terminal_tabs;", [])
            .map_err(|e| format!("Failed to clear terminal tabs: {}", e))?;

        for (idx, tab) in tabs.iter().enumerate() {
            let order = if tab.tab_order >= 0 {
                tab.tab_order
            } else {
                idx as i32
            };

            tx.execute(
                "INSERT INTO terminal_tabs (id, title, cwd, project_name, worktree_name, tab_order, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);",
                params![
                    tab.id,
                    tab.title,
                    tab.cwd,
                    tab.project_name,
                    tab.worktree_name,
                    order,
                    now,
                    now
                ],
            )
            .map_err(|e| format!("Failed to insert terminal tab {}: {}", tab.id, e))?;
        }

        tx.commit()
            .map_err(|e| format!("Failed to commit tabs transaction: {}", e))?;

        Ok(())
    }

    pub fn upsert_tab(&self, tab: &TerminalTabItem) -> Result<(), String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Database lock poisoned: {}", e))?;

        let now = current_timestamp();
        conn.execute(
            "INSERT INTO terminal_tabs (id, title, cwd, project_name, worktree_name, tab_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                cwd = excluded.cwd,
                project_name = excluded.project_name,
                worktree_name = excluded.worktree_name,
                tab_order = excluded.tab_order,
                updated_at = excluded.updated_at;",
            params![
                tab.id,
                tab.title,
                tab.cwd,
                tab.project_name,
                tab.worktree_name,
                tab.tab_order,
                now,
                now
            ],
        )
        .map_err(|e| format!("Failed to upsert tab {}: {}", tab.id, e))?;

        Ok(())
    }

    pub fn delete_tab(&self, id: &str) -> Result<(), String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Database lock poisoned: {}", e))?;

        conn.execute("DELETE FROM terminal_tabs WHERE id = ?1;", params![id])
            .map_err(|e| format!("Failed to delete tab {}: {}", id, e))?;

        Ok(())
    }
}

pub fn get_default_db_path(app: &AppHandle) -> PathBuf {
    let app_data = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("trunks"));
    app_data.join("workspace.db")
}

// Tauri Command Handlers

#[tauri::command]
pub fn db_get_workspace_state(
    state: State<'_, DatabaseManager>,
) -> Result<WorkspaceState, String> {
    state.get_workspace_state()
}

#[tauri::command]
pub fn db_save_layout(
    state: State<'_, DatabaseManager>,
    layout: LayoutState,
) -> Result<(), String> {
    state.save_layout(layout)
}

#[tauri::command]
pub fn db_save_projects(
    state: State<'_, DatabaseManager>,
    projects: Vec<ProjectItem>,
) -> Result<(), String> {
    state.save_projects(&projects)
}

#[tauri::command]
pub fn db_save_tabs(
    state: State<'_, DatabaseManager>,
    tabs: Vec<TerminalTabItem>,
) -> Result<(), String> {
    state.save_tabs(&tabs)
}

#[tauri::command]
pub fn db_upsert_tab(
    state: State<'_, DatabaseManager>,
    tab: TerminalTabItem,
) -> Result<(), String> {
    state.upsert_tab(&tab)
}

#[tauri::command]
pub fn db_delete_tab(
    state: State<'_, DatabaseManager>,
    id: String,
) -> Result<(), String> {
    state.delete_tab(&id)
}

#[tauri::command]
pub fn db_check_path(path: String) -> Result<bool, String> {
    Ok(Path::new(&path).exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_init_and_default_layout() {
        let db = DatabaseManager::new_in_memory().expect("Failed to create in-memory db");
        let state = db.get_workspace_state().expect("Failed to get workspace state");

        assert!(state.layout.show_left);
        assert!(state.layout.show_right);
        assert_eq!(state.layout.active_tab_id, None);
        assert_eq!(state.projects.len(), 0);
        assert_eq!(state.tabs.len(), 0);
    }

    #[test]
    fn test_layout_persistence() {
        let db = DatabaseManager::new_in_memory().expect("Failed to create in-memory db");

        let updated_layout = LayoutState {
            show_left: false,
            show_right: true,
            active_tab_id: Some("tab-test-123".to_string()),
            selected_project_name: Some("trunks".to_string()),
            selected_worktree_name: Some("feature/recovery".to_string()),
        };

        db.save_layout(updated_layout.clone())
            .expect("Failed to save layout");

        let state = db.get_workspace_state().expect("Failed to read state");
        assert_eq!(state.layout.show_left, false);
        assert_eq!(state.layout.show_right, true);
        assert_eq!(state.layout.active_tab_id.as_deref(), Some("tab-test-123"));
        assert_eq!(state.layout.selected_project_name.as_deref(), Some("trunks"));
        assert_eq!(state.layout.selected_worktree_name.as_deref(), Some("feature/recovery"));
    }

    #[test]
    fn test_projects_and_worktree_persistence_and_stale_reconciliation() {
        let db = DatabaseManager::new_in_memory().expect("Failed to create in-memory db");

        let temp_dir = std::env::temp_dir().join("trunks_test_wt_existing");
        let _ = std::fs::create_dir_all(&temp_dir);

        let projects = vec![ProjectItem {
            id: "proj-1".to_string(),
            name: "Test Project".to_string(),
            path: temp_dir.to_string_lossy().to_string(),
            is_stale: false,
            stale_reason: None,
            worktrees: vec![
                WorktreeItem {
                    id: "wt-valid".to_string(),
                    project_id: "proj-1".to_string(),
                    name: "main".to_string(),
                    branch: Some("main".to_string()),
                    path: temp_dir.to_string_lossy().to_string(),
                    is_stale: false,
                    stale_reason: None,
                },
                WorktreeItem {
                    id: "wt-deleted".to_string(),
                    project_id: "proj-1".to_string(),
                    name: "deleted-branch".to_string(),
                    branch: Some("feat/deleted".to_string()),
                    path: "/non_existent/path/for/trunks_test".to_string(),
                    is_stale: false,
                    stale_reason: None,
                },
            ],
        }];

        db.save_projects(&projects).expect("Failed to save projects");

        let state = db.get_workspace_state().expect("Failed to get state");
        assert_eq!(state.projects.len(), 1);
        let p = &state.projects[0];
        assert_eq!(p.is_stale, false);
        assert_eq!(p.worktrees.len(), 2);

        // Valid worktree should NOT be stale
        let valid_wt = p.worktrees.iter().find(|w| w.id == "wt-valid").unwrap();
        assert_eq!(valid_wt.is_stale, false);
        assert_eq!(valid_wt.stale_reason, None);

        // Deleted worktree MUST be flagged as stale without losing metadata (AC-M0-08)
        let deleted_wt = p.worktrees.iter().find(|w| w.id == "wt-deleted").unwrap();
        assert_eq!(deleted_wt.is_stale, true);
        assert!(deleted_wt.stale_reason.is_some());
        assert_eq!(deleted_wt.name, "deleted-branch");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_tabs_ordering_and_incremental_upsert_delete() {
        let db = DatabaseManager::new_in_memory().expect("Failed to create in-memory db");

        let temp_dir = std::env::temp_dir();

        let tab1 = TerminalTabItem {
            id: "tab-1".to_string(),
            title: "Terminal 1".to_string(),
            cwd: temp_dir.to_string_lossy().to_string(),
            project_name: Some("trunks".to_string()),
            worktree_name: Some("main".to_string()),
            tab_order: 0,
            is_stale: false,
            stale_reason: None,
        };

        let tab2 = TerminalTabItem {
            id: "tab-2".to_string(),
            title: "Terminal 2".to_string(),
            cwd: "/non_existent/stale/tab_dir".to_string(),
            project_name: Some("trunks".to_string()),
            worktree_name: Some("feat/stale".to_string()),
            tab_order: 1,
            is_stale: false,
            stale_reason: None,
        };

        db.save_tabs(&[tab1.clone(), tab2.clone()]).expect("Failed to save tabs");

        let state = db.get_workspace_state().expect("Failed to get state");
        assert_eq!(state.tabs.len(), 2);
        assert_eq!(state.tabs[0].id, "tab-1");
        assert_eq!(state.tabs[0].is_stale, false);
        assert_eq!(state.tabs[1].id, "tab-2");
        assert_eq!(state.tabs[1].is_stale, true); // Reconciled as stale

        // Test incremental upsert
        let mut tab1_modified = tab1;
        tab1_modified.title = "Renamed Tab 1".to_string();
        db.upsert_tab(&tab1_modified).expect("Failed to upsert tab");

        let state2 = db.get_workspace_state().expect("Failed to get state");
        assert_eq!(state2.tabs[0].title, "Renamed Tab 1");

        // Test delete
        db.delete_tab("tab-1").expect("Failed to delete tab");
        let state3 = db.get_workspace_state().expect("Failed to get state");
        assert_eq!(state3.tabs.len(), 1);
        assert_eq!(state3.tabs[0].id, "tab-2");
    }
}
