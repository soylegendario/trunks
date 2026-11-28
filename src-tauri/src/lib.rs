mod db;
mod git;
mod pty;
mod snapshots;

use db::{
    db_check_path, db_delete_tab, db_get_snapshots_enabled, db_get_workspace_state, db_save_layout, db_save_projects,
    db_save_tabs, db_set_snapshots_enabled, db_upsert_tab, get_default_db_path, DatabaseManager,
};
use pty::{get_system_info, pty_create, pty_kill, pty_list, pty_resize, pty_write, PtyManager};
use git::{
    git_create_worktree, git_get_diff, git_get_status, git_list_branches, git_list_files,
    git_cancel_commit_history, git_get_commit_diff, git_get_commit_history, git_list_worktrees,
    git_read_file,
    git_remove_worktree, git_validate_repo,
};
use snapshots::{
    default_snapshot_dir, snapshot_clear_all, snapshot_delete, snapshot_load, snapshot_reconcile,
    snapshot_save, snapshots_disable_and_clear, SnapshotStore,
};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(PtyManager::new())
        .setup(|app| {
            let db_path = get_default_db_path(app.handle());
            let db_manager = DatabaseManager::new(&db_path)
                .map_err(|e| format!("Failed to initialize database at {:?}: {}", db_path, e))?;
            app.manage(db_manager);
            app.manage(SnapshotStore::new(default_snapshot_dir(app.handle())));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            pty_create,
            pty_write,
            pty_resize,
            pty_kill,
            pty_list,
            db_get_workspace_state,
            db_save_layout,
            db_save_projects,
            db_save_tabs,
            db_upsert_tab,
            db_delete_tab,
            db_check_path,
            db_get_snapshots_enabled,
            db_set_snapshots_enabled,
            snapshot_save,
            snapshot_load,
            snapshot_delete,
            snapshot_clear_all,
            snapshots_disable_and_clear,
            snapshot_reconcile,
            git_validate_repo,
            git_list_worktrees,
            git_list_branches,
            git_create_worktree,
            git_remove_worktree,
            git_get_status,
            git_get_commit_history,
            git_get_commit_diff,
            git_cancel_commit_history,
            git_get_diff,
            git_list_files,
            git_read_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
