//! Opt-in terminal snapshot store: one file per tab under `<app_data>/snapshots/`.
//!
//! Security constraints (see `docs/superpowers/specs/2026-11-22-m3-terminal-snapshots-design.md`):
//! - the opt-in flag is enforced here, not only in the UI;
//! - `tab_id` is validated before any path is built from it;
//! - snapshot content is never logged nor included in error messages.

use crate::db::DatabaseManager;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, State};

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub data: String,
    pub created_at: i64,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Max stored bytes of terminal text per snapshot; larger buffers are trimmed from the oldest lines.
    pub snapshot_bytes: usize,
    /// Max accepted payload from the frontend; larger payloads are rejected outright.
    pub payload_bytes: usize,
    /// Max total bytes across all snapshot files; oldest snapshots are evicted first.
    pub total_bytes: u64,
}

impl Default for Limits {
    // Provisional values: Task 4 sets them from measurements in the real app.
    fn default() -> Self {
        Self {
            snapshot_bytes: 2 * 1024 * 1024,
            payload_bytes: 16 * 1024 * 1024,
            total_bytes: 64 * 1024 * 1024,
        }
    }
}

pub struct SnapshotStore {
    dir: PathBuf,
    limits: Limits,
    /// Serialises mutations: concurrent saves of one tab would otherwise share a temp file.
    write_lock: Mutex<()>,
}

const MAGIC: &str = "TRUNKS-SNAPSHOT";
const FORMAT_VERSION: u64 = 1;
const EXT: &str = "snap";
const TMP_SUFFIX: &str = "snap.tmp";
const MAX_ID_LEN: usize = 64;

/// A tab id becomes a file name, so only a conservative charset is accepted.
pub fn validate_tab_id(id: &str) -> Result<(), String> {
    let ok = !id.is_empty()
        && id.len() <= MAX_ID_LEN
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if ok {
        Ok(())
    } else {
        Err("Invalid snapshot tab id".to_string())
    }
}

/// Keeps the newest `max` bytes of `data`, starting at a line boundary when one exists.
fn trim_oldest_lines(data: &str, max: usize) -> &str {
    if data.len() <= max {
        return data;
    }
    let mut start = data.len() - max;
    while !data.is_char_boundary(start) {
        start += 1;
    }
    // Already at a line start: nothing partial to discard.
    if start == 0 || data.as_bytes()[start - 1] == b'\n' {
        return &data[start..];
    }
    match data[start..].find('\n') {
        Some(i) => &data[start + i + 1..],
        None => &data[start..],
    }
}

/// Parses `MAGIC\n<json header>\n<body>`. Returns `None` for anything unrecognised.
fn parse(bytes: &[u8]) -> Option<Snapshot> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut parts = text.splitn(3, '\n');
    if parts.next()? != MAGIC {
        return None;
    }
    let header: serde_json::Value = serde_json::from_str(parts.next()?).ok()?;
    if header.get("version")?.as_u64()? != FORMAT_VERSION {
        return None;
    }
    Some(Snapshot {
        data: parts.next()?.to_string(),
        created_at: header.get("created_at")?.as_i64()?,
        cols: u16::try_from(header.get("cols")?.as_u64()?).ok()?,
        rows: u16::try_from(header.get("rows")?.as_u64()?).ok()?,
    })
}

fn io_err(what: &str, e: std::io::Error) -> String {
    // Never include paths or content beyond the error kind.
    format!("Snapshot {} failed: {}", what, e.kind())
}

impl SnapshotStore {
    pub fn new(dir: PathBuf) -> Self {
        Self::with_limits(dir, Limits::default())
    }

    pub fn with_limits(dir: PathBuf, limits: Limits) -> Self {
        Self { dir, limits, write_lock: Mutex::new(()) }
    }

    fn file_path(&self, tab_id: &str) -> PathBuf {
        self.dir.join(format!("{}.{}", tab_id, EXT))
    }

    fn ensure_dir(&self) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| io_err("directory creation", e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&self.dir, std::fs::Permissions::from_mode(0o700))
                .map_err(|e| io_err("directory permissions", e))?;
        }
        // Windows: relies on the per-user app data ACLs; hardening is best effort and untested.
        Ok(())
    }

    pub fn save(
        &self,
        enabled: bool,
        tab_id: &str,
        cols: u16,
        rows: u16,
        data: &str,
        now: i64,
    ) -> Result<(), String> {
        if !enabled {
            return Err("Terminal history is disabled".to_string());
        }
        validate_tab_id(tab_id)?;
        if data.len() > self.limits.payload_bytes {
            return Err("Snapshot payload too large".to_string());
        }
        let body = trim_oldest_lines(data, self.limits.snapshot_bytes);
        let header = serde_json::json!({
            "version": FORMAT_VERSION,
            "created_at": now,
            "cols": cols,
            "rows": rows,
        });
        let mut content = format!("{}\n{}\n", MAGIC, header).into_bytes();
        content.extend_from_slice(body.as_bytes());

        let _guard = self.write_lock.lock().unwrap_or_else(|e| e.into_inner());
        self.ensure_dir()?;
        let final_path = self.file_path(tab_id);
        let tmp_path = self.dir.join(format!("{}.{}", tab_id, TMP_SUFFIX));
        let _ = std::fs::remove_file(&tmp_path);

        let write = || -> std::io::Result<()> {
            use std::io::Write;
            let mut opts = std::fs::OpenOptions::new();
            opts.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                opts.mode(0o600);
            }
            let mut f = opts.open(&tmp_path)?;
            f.write_all(&content)?;
            f.sync_all()?;
            std::fs::rename(&tmp_path, &final_path)
        };
        if let Err(e) = write() {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(io_err("write", e));
        }
        self.enforce_total_cap(tab_id);
        Ok(())
    }

    pub fn load(&self, tab_id: &str) -> Result<Option<Snapshot>, String> {
        validate_tab_id(tab_id)?;
        match std::fs::read(self.file_path(tab_id)) {
            Ok(bytes) => Ok(parse(&bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(io_err("read", e)),
        }
    }

    pub fn delete(&self, tab_id: &str) -> Result<(), String> {
        validate_tab_id(tab_id)?;
        let _guard = self.write_lock.lock().unwrap_or_else(|e| e.into_inner());
        match std::fs::remove_file(self.file_path(tab_id)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(io_err("delete", e)),
        }
    }

    pub fn clear_all(&self) -> Result<(), String> {
        let _guard = self.write_lock.lock().unwrap_or_else(|e| e.into_inner());
                for (path, _) in self.owned_files()? {
            std::fs::remove_file(&path).map_err(|e| io_err("clear", e))?;
        }
        Ok(())
    }

    /// Removes snapshot files whose tab id is not in `live_tab_ids`. Returns how many were removed.
    pub fn reconcile(&self, live_tab_ids: &[String]) -> Result<usize, String> {
        let _guard = self.write_lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut removed = 0;
        for (path, id) in self.owned_files()? {
            let is_snapshot = path.extension().and_then(|e| e.to_str()) == Some(EXT);
            if is_snapshot && live_tab_ids.iter().any(|l| l == &id) {
                continue;
            }
            std::fs::remove_file(&path).map_err(|e| io_err("reconcile", e))?;
            removed += 1;
        }
        Ok(removed)
    }

    /// Files this store owns (`<valid id>.snap` and leftover `<valid id>.snap.tmp`); foreign files are ignored.
    fn owned_files(&self) -> Result<Vec<(PathBuf, String)>, String> {
        let entries = match std::fs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(io_err("listing", e)),
        };
        let mut out = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let id = name
                .strip_suffix(&format!(".{}", TMP_SUFFIX))
                .or_else(|| name.strip_suffix(&format!(".{}", EXT)));
            if let Some(id) = id {
                if validate_tab_id(id).is_ok() {
                    out.push((entry.path(), id.to_string()));
                }
            }
        }
        Ok(out)
    }

    /// Evicts oldest snapshots (by header `created_at`) until the total fits; never evicts `keep_id`.
    /// Best effort: eviction failures must not fail the save that triggered them.
    fn enforce_total_cap(&self, keep_id: &str) {
        let Ok(files) = self.owned_files() else { return };
        let mut items: Vec<(i64, String, PathBuf, u64)> = files
            .into_iter()
            .filter(|(p, _)| p.extension().and_then(|e| e.to_str()) == Some(EXT))
            .filter_map(|(path, id)| {
                let size = std::fs::metadata(&path).ok()?.len();
                let created = std::fs::read(&path)
                    .ok()
                    .and_then(|b| parse(&b))
                    .map(|s| s.created_at)
                    .unwrap_or(0);
                Some((created, id, path, size))
            })
            .collect();
        let mut total: u64 = items.iter().map(|i| i.3).sum();
        items.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
        for (_, id, path, size) in items {
            if total <= self.limits.total_bytes {
                break;
            }
            if id == keep_id {
                continue;
            }
            if std::fs::remove_file(&path).is_ok() {
                total = total.saturating_sub(size);
            }
        }
    }
}

/// Saves only if the persisted opt-in flag is on (the frontend flag is never trusted).
pub fn save_with_opt_in(
    store: &SnapshotStore,
    db: &DatabaseManager,
    tab_id: &str,
    cols: u16,
    rows: u16,
    data: &str,
) -> Result<(), String> {
    let enabled = db.snapshots_enabled()?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    store.save(enabled, tab_id, cols, rows, data, now)
}

pub fn default_snapshot_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("trunks"))
        .join("snapshots")
}

// Tauri command handlers. Async so file I/O never runs on the main thread.

#[tauri::command]
pub async fn snapshot_save(
    store: State<'_, SnapshotStore>,
    db: State<'_, DatabaseManager>,
    tab_id: String,
    cols: u16,
    rows: u16,
    data: String,
) -> Result<(), String> {
    save_with_opt_in(&store, &db, &tab_id, cols, rows, &data)
}

#[tauri::command]
pub async fn snapshot_load(
    store: State<'_, SnapshotStore>,
    tab_id: String,
) -> Result<Option<Snapshot>, String> {
    store.load(&tab_id)
}

#[tauri::command]
pub async fn snapshot_delete(store: State<'_, SnapshotStore>, tab_id: String) -> Result<(), String> {
    store.delete(&tab_id)
}

#[tauri::command]
pub async fn snapshot_clear_all(store: State<'_, SnapshotStore>) -> Result<(), String> {
    store.clear_all()
}

#[tauri::command]
pub async fn snapshot_reconcile(
    store: State<'_, SnapshotStore>,
    live_tab_ids: Vec<String>,
) -> Result<usize, String> {
    store.reconcile(&live_tab_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    /// Unique temp dir removed on drop (no extra dev-dependency allowed by the plan).
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let p = std::env::temp_dir().join(format!("trunks-snap-test-{}-{}", std::process::id(), n));
            let _ = fs::remove_dir_all(&p);
            TempDir(p)
        }
        fn store(&self) -> SnapshotStore {
            SnapshotStore::new(self.0.join("snapshots"))
        }
        fn snap_dir(&self) -> PathBuf {
            self.0.join("snapshots")
        }
        fn file_count(&self) -> usize {
            fs::read_dir(self.snap_dir()).map(|d| d.count()).unwrap_or(0)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const ID_A: &str = "0b9f3c1e-7d2a-4c55-9a10-1f2e3d4c5b6a";
    const ID_B: &str = "7a1d9e00-1111-4222-8333-444455556666";

    #[test]
    fn rejects_invalid_ids() {
        let long = "a".repeat(65);
        for bad in ["", "..", ".", "a/b", "a\\b", "../x", "a.b", "a b", "a\0b", long.as_str()] {
            assert!(validate_tab_id(bad).is_err(), "should reject {:?}", bad);
        }
    }

    #[test]
    fn accepts_uuid_like_ids() {
        assert!(validate_tab_id(ID_A).is_ok());
        assert!(validate_tab_id("tab_1-x").is_ok());
        assert!(validate_tab_id(&"a".repeat(64)).is_ok());
    }

    #[test]
    fn save_load_roundtrip() {
        let t = TempDir::new();
        let s = t.store();
        s.save(true, ID_A, 120, 40, "hello\r\nworld\x1b[31mred\x1b[0m\n", 1000).unwrap();
        let got = s.load(ID_A).unwrap().expect("snapshot present");
        assert_eq!(
            got,
            Snapshot {
                data: "hello\r\nworld\x1b[31mred\x1b[0m\n".to_string(),
                created_at: 1000,
                cols: 120,
                rows: 40
            }
        );
    }

    #[test]
    fn load_missing_returns_none() {
        let t = TempDir::new();
        assert_eq!(t.store().load(ID_A).unwrap(), None);
    }

    #[test]
    fn overwrite_replaces_and_leaves_no_temp_files() {
        let t = TempDir::new();
        let s = t.store();
        s.save(true, ID_A, 80, 24, "first", 1).unwrap();
        s.save(true, ID_A, 80, 24, "second", 2).unwrap();
        assert_eq!(s.load(ID_A).unwrap().unwrap().data, "second");
        assert_eq!(t.file_count(), 1, "only the final .snap file may remain");
    }

    #[test]
    fn oversize_snapshot_is_trimmed_from_oldest_lines() {
        let t = TempDir::new();
        let s = SnapshotStore::with_limits(
            t.snap_dir(),
            Limits { snapshot_bytes: 100, payload_bytes: 10_000, total_bytes: 1_000_000 },
        );
        let data: String = (0..50).map(|i| format!("line-{:03}\r\n", i)).collect();
        s.save(true, ID_A, 80, 24, &data, 1).unwrap();
        let got = s.load(ID_A).unwrap().unwrap().data;
        assert!(got.len() <= 100, "len {} exceeds cap", got.len());
        assert!(got.ends_with("line-049\r\n"), "newest line must survive");
        assert!(got.starts_with("line-"), "must start at a line boundary");
        assert!(!got.contains("line-000"), "oldest line must be dropped");
    }

    #[test]
    fn payload_over_limit_is_rejected_and_writes_nothing() {
        let t = TempDir::new();
        let s = SnapshotStore::with_limits(
            t.snap_dir(),
            Limits { snapshot_bytes: 100, payload_bytes: 200, total_bytes: 1_000_000 },
        );
        let err = s.save(true, ID_A, 80, 24, &"x".repeat(201), 1).unwrap_err();
        assert!(!err.contains("xxxx"), "error must not echo content");
        assert_eq!(t.file_count(), 0);
    }

    #[test]
    fn save_is_refused_when_opt_in_is_off_and_writes_nothing() {
        let t = TempDir::new();
        let s = t.store();
        assert!(s.save(false, ID_A, 80, 24, "secret", 1).is_err());
        assert!(!t.snap_dir().exists(), "not even the directory may be created");
    }

    #[test]
    fn save_rejects_invalid_id_and_writes_nothing() {
        let t = TempDir::new();
        let s = t.store();
        assert!(s.save(true, "../escape", 80, 24, "x", 1).is_err());
        assert!(!t.0.join("escape.snap").exists());
        assert_eq!(t.file_count(), 0);
    }

    #[test]
    fn load_and_delete_reject_invalid_id() {
        let t = TempDir::new();
        let s = t.store();
        assert!(s.load("../x").is_err());
        assert!(s.delete("../x").is_err());
    }

    #[test]
    fn unknown_header_version_returns_none() {
        let t = TempDir::new();
        let s = t.store();
        fs::create_dir_all(t.snap_dir()).unwrap();
        fs::write(
            t.snap_dir().join(format!("{}.snap", ID_A)),
            "TRUNKS-SNAPSHOT\n{\"version\":99,\"created_at\":1,\"cols\":80,\"rows\":24}\nbody",
        )
        .unwrap();
        assert_eq!(s.load(ID_A).unwrap(), None);
    }

    #[test]
    fn corrupt_file_returns_none_not_error() {
        let t = TempDir::new();
        let s = t.store();
        fs::create_dir_all(t.snap_dir()).unwrap();
        fs::write(t.snap_dir().join(format!("{}.snap", ID_A)), "garbage").unwrap();
        assert_eq!(s.load(ID_A).unwrap(), None);
    }

    #[test]
    fn delete_is_idempotent() {
        let t = TempDir::new();
        let s = t.store();
        s.save(true, ID_A, 80, 24, "x", 1).unwrap();
        s.delete(ID_A).unwrap();
        s.delete(ID_A).unwrap();
        assert_eq!(s.load(ID_A).unwrap(), None);
    }

    #[test]
    fn clear_all_removes_every_snapshot_and_tolerates_missing_dir() {
        let t = TempDir::new();
        let s = t.store();
        s.clear_all().unwrap();
        s.save(true, ID_A, 80, 24, "a", 1).unwrap();
        s.save(true, ID_B, 80, 24, "b", 2).unwrap();
        s.clear_all().unwrap();
        assert_eq!(t.file_count(), 0);
    }

    #[test]
    fn reconcile_removes_orphans_only() {
        let t = TempDir::new();
        let s = t.store();
        s.save(true, ID_A, 80, 24, "a", 1).unwrap();
        s.save(true, ID_B, 80, 24, "b", 2).unwrap();
        let removed = s.reconcile(&[ID_A.to_string()]).unwrap();
        assert_eq!(removed, 1);
        assert!(s.load(ID_A).unwrap().is_some());
        assert_eq!(s.load(ID_B).unwrap(), None);
    }

    #[test]
    fn reconcile_leaves_foreign_files_alone() {
        let t = TempDir::new();
        let s = t.store();
        fs::create_dir_all(t.snap_dir()).unwrap();
        fs::write(t.snap_dir().join("notes.txt"), "keep").unwrap();
        s.reconcile(&[]).unwrap();
        assert!(t.snap_dir().join("notes.txt").exists());
    }

    #[test]
    fn total_cap_evicts_oldest_snapshot_first() {
        let t = TempDir::new();
        let s = SnapshotStore::with_limits(
            t.snap_dir(),
            Limits { snapshot_bytes: 1000, payload_bytes: 10_000, total_bytes: 400 },
        );
        let body = "y".repeat(150);
        s.save(true, "t1", 80, 24, &body, 10).unwrap();
        s.save(true, "t2", 80, 24, &body, 20).unwrap();
        s.save(true, "t3", 80, 24, &body, 30).unwrap();
        assert_eq!(s.load("t1").unwrap(), None, "oldest must be evicted");
        assert!(s.load("t2").unwrap().is_some() || s.load("t3").unwrap().is_some());
        assert!(s.load("t3").unwrap().is_some(), "newest must survive");
    }

    #[test]
    fn concurrent_saves_to_same_tab_never_fail_or_corrupt() {
        let t = TempDir::new();
        let s = t.store();
        std::thread::scope(|scope| {
            for w in 0..8 {
                let s = &s;
                scope.spawn(move || {
                    for i in 0..40 {
                        s.save(true, ID_A, 80, 24, &format!("w{}-i{}\n", w, i), 1).unwrap();
                    }
                });
            }
        });
        let got = s.load(ID_A).unwrap().expect("snapshot must be readable");
        assert!(got.data.starts_with('w'));
        assert_eq!(t.file_count(), 1);
    }

    #[test]
    fn save_with_opt_in_follows_persisted_flag() {
        let t = TempDir::new();
        let s = t.store();
        let db = DatabaseManager::new_in_memory().unwrap();
        assert!(save_with_opt_in(&s, &db, ID_A, 80, 24, "secret").is_err());
        assert!(!t.snap_dir().exists(), "flag off: nothing may be written");
        db.set_snapshots_enabled(true).unwrap();
        save_with_opt_in(&s, &db, ID_A, 80, 24, "ok").unwrap();
        assert_eq!(s.load(ID_A).unwrap().unwrap().data, "ok");
        db.set_snapshots_enabled(false).unwrap();
        assert!(save_with_opt_in(&s, &db, ID_A, 80, 24, "later").is_err());
        assert_eq!(s.load(ID_A).unwrap().unwrap().data, "ok", "refused save must not overwrite");
    }

    #[cfg(unix)]
    #[test]
    fn unix_permissions_are_0700_dir_and_0600_files() {
        use std::os::unix::fs::PermissionsExt;
        let t = TempDir::new();
        let s = t.store();
        s.save(true, ID_A, 80, 24, "x", 1).unwrap();
        let dir_mode = fs::metadata(t.snap_dir()).unwrap().permissions().mode() & 0o777;
        let file_mode = fs::metadata(t.snap_dir().join(format!("{}.snap", ID_A)))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(dir_mode, 0o700);
        assert_eq!(file_mode, 0o600);
    }
}
