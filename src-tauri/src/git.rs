//! Native Git backend. Git CLI is the single source of truth; nothing here is persisted.

use serde::Serialize;
use std::path::{Component, Path, PathBuf};

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct GitRepoInfo {
    pub path: String,
    pub name: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct GitWorktreeInfo {
    pub path: String,
    pub head: Option<String>,
    pub branch: Option<String>,
    pub bare: bool,
    pub locked: Option<String>,
    pub prunable: Option<String>,
}

/// Parses the output of `git worktree list --porcelain`.
pub fn parse_worktree_list(input: &str) -> Result<Vec<GitWorktreeInfo>, String> {
    let mut out = Vec::new();
    let mut current: Option<GitWorktreeInfo> = None;

    for line in input.lines() {
        if line.is_empty() {
            if let Some(wt) = current.take() {
                out.push(wt);
            }
            continue;
        }
        let (key, value) = match line.split_once(' ') {
            Some((k, v)) => (k, Some(v)),
            None => (line, None),
        };
        if key == "worktree" {
            if let Some(wt) = current.take() {
                out.push(wt);
            }
            let path = value.ok_or("worktree record without a path")?;
            current = Some(GitWorktreeInfo {
                path: path.to_string(),
                head: None,
                branch: None,
                bare: false,
                locked: None,
                prunable: None,
            });
            continue;
        }
        let wt = current
            .as_mut()
            .ok_or_else(|| format!("attribute '{key}' before any worktree record"))?;
        match key {
            "HEAD" => wt.head = value.map(str::to_string),
            "branch" => {
                wt.branch = value.map(|b| b.strip_prefix("refs/heads/").unwrap_or(b).to_string())
            }
            "bare" => wt.bare = true,
            "locked" => wt.locked = Some(value.unwrap_or("").to_string()),
            "prunable" => wt.prunable = Some(value.unwrap_or("").to_string()),
            _ => {} // `detached` and future attributes
        }
    }
    if let Some(wt) = current.take() {
        out.push(wt);
    }
    Ok(out)
}

/// Resolves `rel` against `root`, guaranteeing the result stays inside `root`.
/// Rejects absolute paths, drive prefixes, `..` components and any existing symlink on the way.
/// Components that do not exist yet are allowed (callers decide whether existence matters).
pub fn resolve_contained(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let mut resolved = root.to_path_buf();
    let mut still_exists = true;
    for component in Path::new(rel).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => {
                resolved.push(part);
                if !still_exists {
                    continue;
                }
                match std::fs::symlink_metadata(&resolved) {
                    Ok(meta) if meta.file_type().is_symlink() => {
                        return Err(format!("refusing to traverse symlink '{}'", resolved.display()));
                    }
                    Ok(_) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => still_exists = false,
                    Err(e) => {
                        return Err(format!("cannot inspect '{}': {e}", resolved.display()));
                    }
                }
            }
            // Prefix, RootDir and ParentDir: absolute or escaping paths.
            _ => {
                return Err(format!(
                    "path '{rel}' must be relative and stay inside the worktree"
                ));
            }
        }
    }
    Ok(resolved)
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct GitBranchInfo {
    pub name: String,
    pub is_remote: bool,
    pub is_current: bool,
    pub commit: String,
}

/// Parses `git for-each-ref --format=%(refname)%00%(objectname)%00%(HEAD)%00%(symref) refs/heads refs/remotes`.
pub fn parse_branch_refs(input: &str) -> Result<Vec<GitBranchInfo>, String> {
    let mut out = Vec::new();
    for line in input.lines().filter(|l| !l.is_empty()) {
        let f: Vec<&str> = line.split('\0').collect();
        if f.len() != 4 {
            return Err(format!("malformed branch ref record '{line}'"));
        }
        let (refname, commit, head, symref) = (f[0], f[1], f[2], f[3]);
        if !symref.is_empty() {
            continue; // symbolic refs such as refs/remotes/origin/HEAD are aliases
        }
        let (name, is_remote) = if let Some(n) = refname.strip_prefix("refs/heads/") {
            (n, false)
        } else if let Some(n) = refname.strip_prefix("refs/remotes/") {
            (n, true)
        } else {
            return Err(format!("unexpected ref '{refname}'"));
        };
        out.push(GitBranchInfo {
            name: name.to_string(),
            is_remote,
            is_current: head == "*",
            commit: commit.to_string(),
        });
    }
    Ok(out)
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GitStatusKind {
    Tracked,
    Untracked,
    Conflicted,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct GitStatusEntry {
    pub path: String,
    pub index_status: char,
    pub worktree_status: char,
    pub kind: GitStatusKind,
    pub original_path: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct GitStatusResult {
    pub entries: Vec<GitStatusEntry>,
    /// Full OID of `HEAD`; `None` on an unborn branch.
    pub head_oid: Option<String>,
}

/// Parses the output of `git status --porcelain=v2 -z`.
pub fn parse_status_porcelain_v2_z(input: &[u8]) -> Result<GitStatusResult, String> {
    let mut tokens = input.split(|b| *b == 0).filter(|t| !t.is_empty());
    let mut entries = Vec::new();

    let text = |t: &[u8]| -> Result<String, String> {
        String::from_utf8(t.to_vec()).map_err(|_| "status output is not valid UTF-8".to_string())
    };
    let xy = |field: &str| -> Result<(char, char), String> {
        let mut c = field.chars();
        match (c.next(), c.next(), c.next()) {
            (Some(i), Some(w), None) => Ok((i, w)),
            _ => Err(format!("malformed XY status field '{field}'")),
        }
    };

    while let Some(token) = tokens.next() {
        let line = text(token)?;
        // Path is always the last field and may contain spaces, so cap the split.
        match line.chars().next() {
            Some('1') => {
                let f: Vec<&str> = line.splitn(9, ' ').collect();
                if f.len() != 9 {
                    return Err(format!("malformed ordinary status record '{line}'"));
                }
                let (index_status, worktree_status) = xy(f[1])?;
                entries.push(GitStatusEntry {
                    path: f[8].to_string(),
                    index_status,
                    worktree_status,
                    kind: GitStatusKind::Tracked,
                    original_path: None,
                });
            }
            Some('2') => {
                let f: Vec<&str> = line.splitn(10, ' ').collect();
                if f.len() != 10 {
                    return Err(format!("malformed rename/copy status record '{line}'"));
                }
                let (index_status, worktree_status) = xy(f[1])?;
                let original = tokens
                    .next()
                    .ok_or_else(|| format!("rename/copy record '{line}' has no original path"))?;
                entries.push(GitStatusEntry {
                    path: f[9].to_string(),
                    index_status,
                    worktree_status,
                    kind: GitStatusKind::Tracked,
                    original_path: Some(text(original)?),
                });
            }
            Some('u') => {
                let f: Vec<&str> = line.splitn(11, ' ').collect();
                if f.len() != 11 {
                    return Err(format!("malformed unmerged status record '{line}'"));
                }
                let (index_status, worktree_status) = xy(f[1])?;
                entries.push(GitStatusEntry {
                    path: f[10].to_string(),
                    index_status,
                    worktree_status,
                    kind: GitStatusKind::Conflicted,
                    original_path: None,
                });
            }
            Some('?') => {
                let path = line
                    .strip_prefix("? ")
                    .ok_or_else(|| format!("malformed untracked record '{line}'"))?;
                entries.push(GitStatusEntry {
                    path: path.to_string(),
                    index_status: '?',
                    worktree_status: '?',
                    kind: GitStatusKind::Untracked,
                    original_path: None,
                });
            }
            Some('!') | Some('#') => {} // ignored files and headers are not reported
            _ => return Err(format!("unrecognised status record '{line}'")),
        }
    }
    Ok(GitStatusResult { entries, head_oid: None })
}

#[derive(Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GitCommitGroup {
    BranchOnly,
    Shared,
    Flat,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct GitCommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub subject: String,
    pub author: String,
    /// Committer timestamp (`%ct`); changes on rebase or amend.
    pub commit_time: i64,
    pub group: GitCommitGroup,
}

#[derive(Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GitCommitCursor {
    pub group: GitCommitGroup,
    pub offset: u32,
    pub head_oid: String,
    pub base_oid: Option<String>,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GitBaseSource {
    Auto,
    Explicit,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GitHeadState {
    Branch,
    Detached,
    Unborn,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct GitCommitHistoryPage {
    pub branch: Option<String>,
    pub head_state: GitHeadState,
    pub head_oid: Option<String>,
    pub base: Option<String>,
    pub base_source: GitBaseSource,
    pub partial_history: bool,
    pub reset: bool,
    pub commits: Vec<GitCommitInfo>,
    pub next_cursor: Option<GitCommitCursor>,
}

/// Parses `git log/rev-list --format=%H%x00%h%x00%an%x00%ct%x00%s%x1e` output.
/// Fields are NUL-separated and records end with `\x1e`; text is decoded lossily.
pub fn parse_commit_history(
    input: &[u8],
    group: GitCommitGroup,
) -> Result<Vec<GitCommitInfo>, String> {
    let mut pieces: Vec<&[u8]> = input.split(|b| *b == 0x1e).collect();
    // Whatever follows the last terminator must be blank; otherwise the output was cut short.
    let tail = pieces.pop().unwrap_or(&[]);
    if tail.iter().any(|b| !b.is_ascii_whitespace()) {
        return Err("truncated commit history record".to_string());
    }
    let lossy = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    let mut commits = Vec::with_capacity(pieces.len());
    for piece in pieces {
        // `git log` separates records with a newline.
        let start = piece
            .iter()
            .position(|b| !b.is_ascii_whitespace())
            .unwrap_or(piece.len());
        let record = &piece[start..];
        if record.is_empty() {
            continue;
        }
        let f: Vec<&[u8]> = record.splitn(5, |b| *b == 0).collect();
        if f.len() != 5 {
            return Err(format!(
                "malformed commit history record '{}'",
                lossy(record)
            ));
        }
        let commit_time = lossy(f[3])
            .parse::<i64>()
            .map_err(|_| format!("invalid commit timestamp '{}'", lossy(f[3])))?;
        commits.push(GitCommitInfo {
            hash: lossy(f[0]),
            short_hash: lossy(f[1]),
            author: lossy(f[2]),
            commit_time,
            subject: lossy(f[4]),
            group,
        });
    }
    Ok(commits)
}

/// Commits per page. Provisional: it is set after measuring representative repositories
/// (see Task 1b and the Deferred Decisions in the M2 plan).
const COMMIT_HISTORY_PAGE_SIZE: usize = 200;

const COMMIT_LOG_FORMAT: &str = "--format=%H%x00%h%x00%an%x00%ct%x00%s%x1e";

/// Resolves `refname` to a full commit OID, or `None` when it does not exist.
async fn resolve_oid(cwd: &Path, refname: &str) -> Option<String> {
    let spec = format!("{refname}^{{commit}}");
    let out = run_git(
        cwd,
        &["rev-parse", "--verify", "--quiet", "--end-of-options", &spec],
        "git rev-parse --verify",
    )
    .await
    .ok()?;
    let oid = String::from_utf8(out).ok()?.trim().to_string();
    (oid.len() >= 40 && oid.chars().all(|c| c.is_ascii_hexdigit())).then_some(oid)
}

/// Resolves a remote's symbolic `HEAD` (for example `origin/HEAD`) to `(label, oid)`.
async fn remote_default_base(cwd: &Path, remote: &str) -> Option<(String, String)> {
    let head_ref = format!("refs/remotes/{remote}/HEAD");
    let out = run_git(cwd, &["symbolic-ref", "--quiet", &head_ref], "git symbolic-ref")
        .await
        .ok()?;
    let target = String::from_utf8(out).ok()?.trim().to_string();
    let label = target.strip_prefix("refs/remotes/")?.to_string();
    let oid = resolve_oid(cwd, &target).await?;
    Some((label, oid))
}

/// Automatic base detection, in the order documented in the M2 design.
async fn detect_base(cwd: &Path, branch: &str) -> Option<(String, String)> {
    // 1. The remote of the branch's configured upstream, via its symbolic HEAD.
    let key = format!("branch.{branch}.remote");
    if let Ok(out) = run_git(cwd, &["config", "--get", &key], "git config --get").await {
        let remote = String::from_utf8_lossy(&out).trim().to_string();
        if !remote.is_empty() && remote != "." {
            if let Some(found) = remote_default_base(cwd, &remote).await {
                return Some(found);
            }
        }
    }
    // 2. Exactly one configured remote with a resolvable symbolic HEAD.
    let remotes = run_git(cwd, &["remote"], "git remote").await.unwrap_or_default();
    let mut candidates = Vec::new();
    for remote in String::from_utf8_lossy(&remotes).lines().filter(|l| !l.is_empty()) {
        if let Some(found) = remote_default_base(cwd, remote).await {
            candidates.push(found);
        }
    }
    if candidates.len() == 1 {
        return candidates.pop();
    }
    if candidates.len() > 1 {
        return None; // ambiguous remote defaults: do not guess
    }
    // 3. A local `main` or `master`, only when exactly one of them exists.
    let main = resolve_oid(cwd, "refs/heads/main").await;
    let master = resolve_oid(cwd, "refs/heads/master").await;
    match (main, master) {
        (Some(oid), None) => Some(("main".to_string(), oid)),
        (None, Some(oid)) => Some(("master".to_string(), oid)),
        _ => None,
    }
}

async fn query_commits(
    cwd: &Path,
    revs: &[String],
    group: GitCommitGroup,
    skip: usize,
    limit: usize,
) -> Result<Vec<GitCommitInfo>, String> {
    let skip = skip.to_string();
    let limit = limit.to_string();
    let mut args: Vec<&str> = vec![
        "log", "--topo-order", COMMIT_LOG_FORMAT, "--skip", &skip, "-n", &limit,
        "--end-of-options",
    ];
    args.extend(revs.iter().map(String::as_str));
    let out = run_git(cwd, &args, "git log").await?;
    parse_commit_history(&out, group)
}

async fn commit_history(
    worktree_path: &str,
    base_ref: Option<&str>,
    cursor: Option<&GitCommitCursor>,
    page_size: usize,
) -> Result<GitCommitHistoryPage, String> {
    let cwd = Path::new(worktree_path);
    let branch = run_git(cwd, &["symbolic-ref", "--quiet", "HEAD"], "git symbolic-ref HEAD")
        .await
        .ok()
        .and_then(|o| String::from_utf8(o).ok())
        .and_then(|r| r.trim().strip_prefix("refs/heads/").map(str::to_string));
    let head_oid = resolve_oid(cwd, "HEAD").await;

    let explicit = base_ref.is_some();
    let base_source = if explicit { GitBaseSource::Explicit } else { GitBaseSource::Auto };
    let mut page = GitCommitHistoryPage {
        branch: branch.clone(),
        head_state: GitHeadState::Branch,
        head_oid: head_oid.clone(),
        base: None,
        base_source,
        partial_history: false,
        reset: false,
        commits: Vec::new(),
        next_cursor: None,
    };

    // An explicit base is validated even when the history ends up flat, so a bad value
    // is always reported instead of being silently ignored.
    let mut base: Option<(String, String)> = None;
    if let Some(name) = base_ref {
        let list = git_list_branches(worktree_path.to_string()).await?;
        let info = list
            .iter()
            .find(|b| b.name == name && !b.is_remote)
            .or_else(|| list.iter().find(|b| b.name == name && b.is_remote))
            .ok_or_else(|| format!("base '{name}' is not an existing local or remote branch"))?;
        let full = if info.is_remote {
            format!("refs/remotes/{}", info.name)
        } else {
            format!("refs/heads/{}", info.name)
        };
        let oid = resolve_oid(cwd, &full)
            .await
            .ok_or_else(|| format!("base '{name}' does not resolve to a commit"))?;
        base = Some((info.name.clone(), oid));
    }

    let Some(head_oid) = head_oid else {
        // Unborn branch: nothing to list. (A detached HEAD always resolves.)
        page.head_state = GitHeadState::Unborn;
        page.base = base.map(|(label, _)| label);
        return Ok(page);
    };
    if branch.is_none() {
        page.head_state = GitHeadState::Detached;
    }
    let shallow = run_git(
        cwd,
        &["rev-parse", "--is-shallow-repository"],
        "git rev-parse --is-shallow-repository",
    )
    .await
    .map(|o| String::from_utf8_lossy(&o).trim() == "true")
    .unwrap_or(false);
    page.partial_history = shallow;

    // Detached HEAD and shallow clones are listed flat; an explicit base is still reported.
    let flat = branch.is_none() || shallow;
    if !flat && !explicit {
        base = detect_base(cwd, branch.as_deref().unwrap_or_default()).await;
    }
    page.base = base.as_ref().map(|(label, _)| label.clone());
    let base_oid = if flat { None } else { base.map(|(_, oid)| oid) };

    // Revision sets per group.
    let mut groups: Vec<(GitCommitGroup, Vec<String>)> = Vec::new();
    match &base_oid {
        None => groups.push((GitCommitGroup::Flat, vec![head_oid.clone()])),
        Some(base_oid) => {
            groups.push((
                GitCommitGroup::BranchOnly,
                vec![head_oid.clone(), format!("^{base_oid}")],
            ));
            let out = run_git(
                cwd,
                &["merge-base", "--all", &head_oid, base_oid],
                "git merge-base --all",
            )
            .await;
            // Exit status 1 means the histories are unrelated: the shared group is empty.
            let bases: Vec<String> = out
                .map(|o| String::from_utf8_lossy(&o).lines().map(str::to_string).collect())
                .unwrap_or_default();
            if !bases.is_empty() {
                groups.push((GitCommitGroup::Shared, bases));
            }
        }
    }

    // A cursor only pages the graph it was issued for. If HEAD, the base or the group no
    // longer match, serve a fresh first page and tell the UI to discard what it accumulated.
    let cursor = match cursor {
        Some(c)
            if c.head_oid == head_oid
                && c.base_oid == base_oid
                && groups.iter().any(|(g, _)| *g == c.group) =>
        {
            Some(c)
        }
        Some(_) => {
            page.reset = true;
            None
        }
        None => None,
    };
    let (start_group, start_offset) = match cursor {
        Some(c) => (Some(c.group), c.offset as usize),
        None => (None, 0),
    };
    let mut started = start_group.is_none();
    let mut budget = page_size;
    for (group, revs) in &groups {
        let mut offset = 0;
        if !started {
            if Some(*group) != start_group {
                continue;
            }
            started = true;
            offset = start_offset;
        }
        let mut got = query_commits(cwd, revs, *group, offset, budget + 1).await?;
        if got.len() > budget {
            got.truncate(budget);
            page.commits.extend(got);
            page.next_cursor = Some(GitCommitCursor {
                group: *group,
                offset: (offset + budget) as u32,
                head_oid: head_oid.clone(),
                base_oid: base_oid.clone(),
            });
            break;
        }
        budget -= got.len();
        page.commits.extend(got);
    }
    Ok(page)
}

#[tauri::command]
pub async fn git_get_commit_history(
    worktree_path: String,
    base_ref: Option<String>,
    cursor: Option<GitCommitCursor>,
    request_id: String,
) -> Result<GitCommitHistoryPage, String> {
    let work = commit_history(
        &worktree_path,
        base_ref.as_deref(),
        cursor.as_ref(),
        COMMIT_HISTORY_PAGE_SIZE,
    );
    run_cancellable(&worktree_path, &request_id, work).await
}

/// In-flight history requests: `request_id -> (worktree, cancel signal)`.
type InFlight = std::sync::Mutex<
    std::collections::HashMap<String, (String, std::sync::Arc<tokio::sync::Notify>)>,
>;

fn in_flight() -> &'static InFlight {
    static REGISTRY: std::sync::OnceLock<InFlight> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(Default::default)
}

pub const CANCELLED: &str = "commit history request cancelled";

/// Runs `work`, aborting it when `request_id` is cancelled or a newer request for the same
/// worktree starts. Dropping `work` drops the pending `git` child, which `run_git` kills
/// (`kill_on_drop`), so no orphan process survives a discarded request.
async fn run_cancellable<T>(
    worktree: &str,
    request_id: &str,
    work: impl std::future::Future<Output = Result<T, String>>,
) -> Result<T, String> {
    let signal = std::sync::Arc::new(tokio::sync::Notify::new());
    {
        let mut map = in_flight().lock().unwrap();
        map.retain(|_, (wt, old)| {
            if wt == worktree {
                old.notify_one();
                false
            } else {
                true
            }
        });
        map.insert(request_id.to_string(), (worktree.to_string(), signal.clone()));
    }
    let result = tokio::select! {
        r = work => r,
        _ = signal.notified() => Err(CANCELLED.to_string()),
    };
    let mut map = in_flight().lock().unwrap();
    if map.get(request_id).is_some_and(|(_, s)| std::sync::Arc::ptr_eq(s, &signal)) {
        map.remove(request_id);
    }
    result
}

#[tauri::command]
pub async fn git_cancel_commit_history(request_id: String) -> Result<(), String> {
    if let Some((_, signal)) = in_flight().lock().unwrap().remove(&request_id) {
        signal.notify_one();
    }
    Ok(())
}

/// Runs `git <args>` inside `cwd` without a shell and without blocking the UI thread.
/// `op` names the operation in error messages. No timeout is imposed in M1.
async fn run_git(cwd: &Path, args: &[&str], op: &str) -> Result<Vec<u8>, String> {
    let output = tokio::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "git executable not found; install Git and make sure it is on PATH".to_string()
            } else {
                format!("{op}: failed to launch git: {e}")
            }
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{op} failed: {}", stderr.trim()));
    }
    Ok(output.stdout)
}

fn stdout_text(bytes: Vec<u8>, op: &str) -> Result<String, String> {
    String::from_utf8(bytes).map_err(|_| format!("{op}: git output is not valid UTF-8"))
}

#[tauri::command]
pub async fn git_validate_repo(path: String) -> Result<GitRepoInfo, String> {
    let out = run_git(
        Path::new(&path),
        &["rev-parse", "--show-toplevel"],
        "git rev-parse --show-toplevel",
    )
    .await?;
    let toplevel = stdout_text(out, "git rev-parse --show-toplevel")?;
    let toplevel = toplevel.trim_end_matches(['\n', '\r']);
    let root = tokio::fs::canonicalize(toplevel)
        .await
        .map_err(|e| format!("cannot canonicalize repository root '{toplevel}': {e}"))?;
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.to_string_lossy().into_owned());
    Ok(GitRepoInfo {
        path: root.to_string_lossy().into_owned(),
        name,
    })
}

#[tauri::command]
pub async fn git_list_worktrees(repo_path: String) -> Result<Vec<GitWorktreeInfo>, String> {
    let op = "git worktree list --porcelain";
    let out = run_git(Path::new(&repo_path), &["worktree", "list", "--porcelain"], op).await?;
    parse_worktree_list(&stdout_text(out, op)?)
}

#[tauri::command]
pub async fn git_list_branches(repo_path: String) -> Result<Vec<GitBranchInfo>, String> {
    let op = "git for-each-ref";
    let out = run_git(
        Path::new(&repo_path),
        &[
            "for-each-ref",
            "--format=%(refname)%00%(objectname)%00%(HEAD)%00%(symref)",
            "refs/heads",
            "refs/remotes",
        ],
        op,
    )
    .await?;
    parse_branch_refs(&stdout_text(out, op)?)
}

/// Picks the worktree `target` refers to, refusing the main worktree (always listed first
/// by Git) and any path Git does not list. Paths are compared canonicalised so symlinked
/// aliases (for instance `/var` vs `/private/var` on macOS) cannot bypass the guard.
fn select_removable<'a>(
    worktrees: &'a [GitWorktreeInfo],
    target: &str,
) -> Result<&'a GitWorktreeInfo, String> {
    let wanted = canonical_or_raw(Path::new(target));
    let index = worktrees
        .iter()
        .position(|w| canonical_or_raw(Path::new(&w.path)) == wanted)
        .ok_or_else(|| format!("'{target}' is not a worktree of this repository"))?;
    if index == 0 {
        return Err(format!("refusing to remove the main worktree '{target}'"));
    }
    Ok(&worktrees[index])
}

fn canonical_or_raw(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Arguments for `git worktree remove`. `--force` is only present when explicitly confirmed.
fn removal_args(path: &str, force: bool) -> Vec<&str> {
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.extend(["--", path]);
    args
}

#[tauri::command]
pub async fn git_create_worktree(
    repo_path: String,
    worktree_path: String,
    branch: String,
    new_branch: bool,
) -> Result<GitWorktreeInfo, String> {
    if branch.is_empty() || branch.starts_with('-') {
        return Err(format!(
            "invalid branch name '{branch}': it cannot be empty or start with '-'"
        ));
    }
    if !Path::new(&worktree_path).is_absolute() {
        return Err(format!("worktree path must be absolute: '{worktree_path}'"));
    }
    let args: Vec<&str> = if new_branch {
        vec!["worktree", "add", "-b", &branch, "--", &worktree_path]
    } else {
        vec!["worktree", "add", "--", &worktree_path, &branch]
    };
    run_git(Path::new(&repo_path), &args, "git worktree add").await?;

    let wanted = canonical_or_raw(Path::new(&worktree_path));
    git_list_worktrees(repo_path)
        .await?
        .into_iter()
        .find(|w| canonical_or_raw(Path::new(&w.path)) == wanted)
        .ok_or_else(|| format!("worktree '{worktree_path}' was created but is not listed by git"))
}

#[tauri::command]
pub async fn git_remove_worktree(
    repo_path: String,
    worktree_path: String,
    force: bool,
) -> Result<(), String> {
    let listed = git_list_worktrees(repo_path.clone()).await?;
    // Remove using the path Git reports, never the caller's spelling of it.
    let target = select_removable(&listed, &worktree_path)?.path.clone();
    run_git(
        Path::new(&repo_path),
        &removal_args(&target, force),
        "git worktree remove",
    )
    .await?;
    Ok(())
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct GitFileInfo {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_symlink: bool,
}

/// Turns a `git status` failure into a clearer message when the installed Git is too old
/// for `--porcelain=v2`.
fn explain_status_error(err: String) -> String {
    if err.to_lowercase().contains("porcelain") {
        format!(
            "{err}\nThe installed Git does not support `git status --porcelain=v2` \
             (Git 2.11 or newer is required). Please upgrade Git."
        )
    } else {
        err
    }
}

#[tauri::command]
pub async fn git_get_status(worktree_path: String) -> Result<GitStatusResult, String> {
    // --no-optional-locks: the inspector polls, and must not contend with the user's own
    // git commands for the index lock.
    let out = run_git(
        Path::new(&worktree_path),
        &["--no-optional-locks", "status", "--porcelain=v2", "-z", "--untracked-files=all"],
        "git status --porcelain=v2 -z",
    )
    .await
    .map_err(explain_status_error)?;
    let mut result = parse_status_porcelain_v2_z(&out)?;
    result.head_oid = resolve_oid(Path::new(&worktree_path), "HEAD").await;
    Ok(result)
}

#[tauri::command]
pub async fn git_list_files(
    worktree_path: String,
    subpath: Option<String>,
) -> Result<Vec<GitFileInfo>, String> {
    let root = Path::new(&worktree_path);
    let dir = resolve_contained(root, subpath.as_deref().unwrap_or(""))?;
    // Build output paths from the resolved directory so `sub`, `sub/` and `./sub` agree.
    let rel_dir = dir
        .strip_prefix(root)
        .map_err(|_| format!("'{}' is outside the worktree", dir.display()))?
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/");

    let read_err = |e: std::io::Error| format!("cannot read directory '{}': {e}", dir.display());
    let mut entries = tokio::fs::read_dir(&dir).await.map_err(read_err)?;
    let mut files = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(read_err)? {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == ".git" {
            continue;
        }
        // DirEntry::file_type does not follow symlinks.
        let file_type = entry
            .file_type()
            .await
            .map_err(|e| format!("cannot inspect '{}': {e}", entry.path().display()))?;
        let path = if rel_dir.is_empty() {
            name.clone()
        } else {
            format!("{rel_dir}/{name}")
        };
        files.push(GitFileInfo {
            name,
            path,
            is_dir: file_type.is_dir(),
            is_symlink: file_type.is_symlink(),
        });
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}

/// Untracked files above this size are not turned into a synthetic diff.
const MAX_UNTRACKED_DIFF_BYTES: u64 = 1024 * 1024;
/// Like Git, a NUL byte within the first 8 KiB marks a file as binary.
const BINARY_SNIFF_BYTES: usize = 8192;

/// Purely lexical validation of a worktree-relative path: no absolute paths, drive prefixes
/// or `..`. Returns the path normalised to `/` separators. Symlinks are NOT inspected here
/// (a tracked symlink has a legitimate diff); callers that read the file use `resolve_contained`.
fn lexical_relative(rel: &str) -> Result<String, String> {
    let mut parts = Vec::new();
    for component in Path::new(rel).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            _ => {
                return Err(format!(
                    "path '{rel}' must be relative and stay inside the worktree"
                ));
            }
        }
    }
    if parts.is_empty() {
        return Err("file path must not be empty".to_string());
    }
    Ok(parts.join("/"))
}

async fn is_untracked(root: &Path, rel: &str) -> Result<bool, String> {
    let out = run_git(
        root,
        &[
            "--literal-pathspecs",
            "ls-files",
            "--others",
            "--exclude-standard",
            "-z",
            "--",
            rel,
        ],
        "git ls-files --others",
    )
    .await?;
    // Exact match only: an untracked directory must not be treated as an untracked file.
    Ok(out.split(|b| *b == 0).any(|entry| entry == rel.as_bytes()))
}

fn format_new_file_diff(path: &str, text: &str) -> String {
    let mut out = format!("diff --git a/{path} b/{path}\nnew file mode 100644\n");
    if text.is_empty() {
        return out;
    }
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let count = if lines.len() == 1 {
        "1".to_string()
    } else {
        format!("1,{}", lines.len())
    };
    out.push_str(&format!("--- /dev/null\n+++ b/{path}\n@@ -0,0 +{count} @@\n"));
    for line in &lines {
        out.push('+');
        out.push_str(line.strip_suffix('\n').unwrap_or(line));
        out.push('\n');
    }
    if !text.ends_with('\n') {
        out.push_str("\\ No newline at end of file\n");
    }
    out
}

async fn synthesize_new_file_diff(root: &Path, rel: &str) -> Result<String, String> {
    use tokio::io::AsyncReadExt;

    let too_large = || {
        format!(
            "'{rel}' is too large to display as a diff (limit {} MiB)",
            MAX_UNTRACKED_DIFF_BYTES / (1024 * 1024)
        )
    };
    let path = resolve_contained(root, rel)?;
    let meta = tokio::fs::symlink_metadata(&path)
        .await
        .map_err(|e| format!("cannot inspect '{rel}': {e}"))?;
    if !meta.is_file() {
        return Err(format!("'{rel}' is not a regular file; no text diff available"));
    }
    if meta.len() > MAX_UNTRACKED_DIFF_BYTES {
        return Err(too_large());
    }
    // Read at most limit + 1 bytes so a file that grows after the check is still bounded.
    let mut bytes = Vec::new();
    tokio::fs::File::open(&path)
        .await
        .map_err(|e| format!("cannot open '{rel}': {e}"))?
        .take(MAX_UNTRACKED_DIFF_BYTES + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|e| format!("cannot read '{rel}': {e}"))?;
    if bytes.len() as u64 > MAX_UNTRACKED_DIFF_BYTES {
        return Err(too_large());
    }
    if bytes[..bytes.len().min(BINARY_SNIFF_BYTES)].contains(&0) {
        return Err(format!("'{rel}' is a binary file; no text diff available"));
    }
    Ok(format_new_file_diff(rel, &String::from_utf8_lossy(&bytes)))
}

/// Reads a text file inside a worktree for the read-only viewer.
#[tauri::command]
pub async fn git_read_file(worktree_path: String, file_path: String) -> Result<String, String> {
    use tokio::io::AsyncReadExt;

    let root = Path::new(&worktree_path);
    let rel = lexical_relative(&file_path)?;
    let too_large = || {
        format!(
            "'{rel}' is too large to display (limit {} MiB)",
            MAX_UNTRACKED_DIFF_BYTES / (1024 * 1024)
        )
    };
    let path = resolve_contained(root, &rel)?;
    let meta = tokio::fs::symlink_metadata(&path)
        .await
        .map_err(|e| format!("cannot inspect '{rel}': {e}"))?;
    if !meta.is_file() {
        return Err(format!("'{rel}' is not a regular file"));
    }
    if meta.len() > MAX_UNTRACKED_DIFF_BYTES {
        return Err(too_large());
    }
    let mut bytes = Vec::new();
    tokio::fs::File::open(&path)
        .await
        .map_err(|e| format!("cannot open '{rel}': {e}"))?
        .take(MAX_UNTRACKED_DIFF_BYTES + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|e| format!("cannot read '{rel}': {e}"))?;
    if bytes.len() as u64 > MAX_UNTRACKED_DIFF_BYTES {
        return Err(too_large());
    }
    if bytes[..bytes.len().min(BINARY_SNIFF_BYTES)].contains(&0) {
        return Err(format!("'{rel}' is a binary file; no preview available"));
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[tauri::command]
pub async fn git_get_diff(
    worktree_path: String,
    file_path: Option<String>,
    cached: bool,
) -> Result<String, String> {
    let root = Path::new(&worktree_path);
    let rel = file_path.as_deref().map(lexical_relative).transpose()?;

    let mut args = vec![
        "--no-optional-locks",
        "--literal-pathspecs",
        "diff",
        "--no-ext-diff",
        "--no-color",
    ];
    if cached {
        args.push("--cached");
    }
    if let Some(rel) = &rel {
        args.extend(["--", rel.as_str()]);
    }
    let out = run_git(root, &args, "git diff").await?;
    let diff = String::from_utf8_lossy(&out).into_owned();

    if diff.is_empty() && !cached {
        if let Some(rel) = &rel {
            if is_untracked(root, rel).await? {
                return synthesize_new_file_diff(root, rel).await;
            }
        }
    }
    Ok(diff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_worktree_porcelain_records() {
        let input = "\
worktree /home/u/repo
HEAD 1111111111111111111111111111111111111111
branch refs/heads/main

worktree /home/u/my work/ñandú árbol
HEAD 2222222222222222222222222222222222222222
branch refs/heads/feature/x
locked reason: on a usb stick

worktree /home/u/detached
HEAD 3333333333333333333333333333333333333333
detached
prunable gitdir file points to non-existent location

worktree /home/u/bare.git
bare

worktree /home/u/locked-no-reason
HEAD 4444444444444444444444444444444444444444
branch refs/heads/y
locked
";
        let got = parse_worktree_list(input).expect("should parse");
        assert_eq!(got.len(), 5);

        assert_eq!(got[0].path, "/home/u/repo");
        assert_eq!(
            got[0].head.as_deref(),
            Some("1111111111111111111111111111111111111111")
        );
        assert_eq!(got[0].branch.as_deref(), Some("main"));
        assert!(!got[0].bare);
        assert_eq!(got[0].locked, None);
        assert_eq!(got[0].prunable, None);

        assert_eq!(got[1].path, "/home/u/my work/ñandú árbol");
        assert_eq!(got[1].branch.as_deref(), Some("feature/x"));
        assert_eq!(got[1].locked.as_deref(), Some("reason: on a usb stick"));

        assert_eq!(got[2].branch, None);
        assert!(got[2].head.is_some());
        assert_eq!(
            got[2].prunable.as_deref(),
            Some("gitdir file points to non-existent location")
        );

        assert_eq!(got[3].path, "/home/u/bare.git");
        assert!(got[3].bare);
        assert_eq!(got[3].head, None);
        assert_eq!(got[3].branch, None);

        assert_eq!(got[4].locked.as_deref(), Some(""));
    }

    #[test]
    fn parses_status_porcelain_v2_z_entries() {
        let h = "0000000000000000000000000000000000000000";
        let records: Vec<String> = vec![
            format!("1 M. N... 100644 100644 100644 {h} {h} src/staged.rs"),
            format!("1 .M N... 100644 100644 100644 {h} {h} dir with space/ñandú.txt"),
            format!("1 MM N... 100644 100644 100644 {h} {h} both.rs"),
            format!("2 R. N... 100644 100644 100644 {h} {h} R100 new name.rs"),
            "old name.rs".to_string(),
            format!("2 C. N... 100644 100644 100644 {h} {h} C75 copy.rs"),
            "source.rs".to_string(),
            format!("u UU N... 100644 100644 100644 100644 {h} {h} {h} conflict.rs"),
            "? untracked file.txt".to_string(),
            "! ignored.log".to_string(),
        ];
        let mut bytes = Vec::new();
        for r in &records {
            bytes.extend_from_slice(r.as_bytes());
            bytes.push(0);
        }

        let got = parse_status_porcelain_v2_z(&bytes).expect("should parse").entries;
        assert_eq!(got.len(), 7, "ignored entries are omitted");

        assert_eq!(got[0].path, "src/staged.rs");
        assert_eq!((got[0].index_status, got[0].worktree_status), ('M', '.'));
        assert_eq!(got[0].kind, GitStatusKind::Tracked);
        assert_eq!(got[0].original_path, None);

        assert_eq!(got[1].path, "dir with space/ñandú.txt");
        assert_eq!((got[1].index_status, got[1].worktree_status), ('.', 'M'));

        assert_eq!((got[2].index_status, got[2].worktree_status), ('M', 'M'));

        assert_eq!(got[3].path, "new name.rs");
        assert_eq!(got[3].original_path.as_deref(), Some("old name.rs"));
        assert_eq!(got[3].index_status, 'R');

        assert_eq!(got[4].path, "copy.rs");
        assert_eq!(got[4].original_path.as_deref(), Some("source.rs"));
        assert_eq!(got[4].index_status, 'C');

        assert_eq!(got[5].path, "conflict.rs");
        assert_eq!(got[5].kind, GitStatusKind::Conflicted);
        assert_eq!((got[5].index_status, got[5].worktree_status), ('U', 'U'));

        assert_eq!(got[6].path, "untracked file.txt");
        assert_eq!(got[6].kind, GitStatusKind::Untracked);
        assert_eq!((got[6].index_status, got[6].worktree_status), ('?', '?'));

        // The rename's original path must not leak out as its own entry.
        assert!(got.iter().all(|e| e.path != "old name.rs" && e.path != "source.rs"));
    }

    #[test]
    fn parses_branch_ref_records() {
        let a = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let b = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let input = format!(
            "refs/heads/main\0{a}\0*\0\n\
             refs/heads/feature/ñandú\0{b}\0 \0\n\
             refs/remotes/origin/main\0{a}\0 \0\n\
             refs/remotes/origin/HEAD\0{a}\0 \0refs/remotes/origin/main\n"
        );
        let got = parse_branch_refs(&input).expect("should parse");
        assert_eq!(got.len(), 3, "symbolic remote HEAD must be omitted");

        assert_eq!(got[0].name, "main");
        assert!(!got[0].is_remote);
        assert!(got[0].is_current);
        assert_eq!(got[0].commit, a);

        assert_eq!(got[1].name, "feature/ñandú");
        assert!(!got[1].is_remote);
        assert!(!got[1].is_current);
        assert_eq!(got[1].commit, b);

        assert_eq!(got[2].name, "origin/main");
        assert!(got[2].is_remote);
        assert!(!got[2].is_current);
    }

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let p = std::env::temp_dir().join(format!(
                "trunks-git-{tag}-{}-{nanos}",
                std::process::id()
            ));
            std::fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn path_guard_accepts_contained_relative_directory() {
        let tmp = TempDir::new("accept");
        std::fs::create_dir_all(tmp.0.join("src/mi carpeta ñ")).unwrap();

        let got = resolve_contained(&tmp.0, "src/mi carpeta ñ").expect("contained path");
        assert_eq!(got, tmp.0.join("src").join("mi carpeta ñ"));
        assert_eq!(resolve_contained(&tmp.0, "").unwrap(), tmp.0);
        assert_eq!(resolve_contained(&tmp.0, "does/not/exist").unwrap(), tmp.0.join("does/not/exist"));
    }

    #[test]
    fn path_guard_rejects_absolute_and_parent_paths() {
        let tmp = TempDir::new("reject");
        assert!(resolve_contained(&tmp.0, "/etc").is_err());
        assert!(resolve_contained(&tmp.0, "..").is_err());
        assert!(resolve_contained(&tmp.0, "../outside").is_err());
        assert!(resolve_contained(&tmp.0, "src/../../outside").is_err());
        // Even a `..` that stays inside is rejected: no lexical tricks.
        assert!(resolve_contained(&tmp.0, "src/../src").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn path_guard_refuses_to_traverse_symlinks() {
        let tmp = TempDir::new("symlink");
        let outside = TempDir::new("symlink-target");
        std::fs::create_dir_all(outside.0.join("secret")).unwrap();
        std::os::unix::fs::symlink(&outside.0, tmp.0.join("link")).unwrap();

        assert!(resolve_contained(&tmp.0, "link").is_err(), "symlink itself");
        assert!(resolve_contained(&tmp.0, "link/secret").is_err(), "through symlink");
    }

    // ---- commit history parser ----

    fn rec(hash: &str, short: &str, author: &[u8], time: &str, subject: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        for (i, f) in [hash.as_bytes(), short.as_bytes(), author, time.as_bytes(), subject]
            .iter()
            .enumerate()
        {
            if i > 0 {
                v.push(0);
            }
            v.extend_from_slice(f);
        }
        v.push(0x1e);
        v
    }

    #[test]
    fn parses_commit_history_empty_input() {
        assert!(parse_commit_history(b"", GitCommitGroup::Flat).unwrap().is_empty());
        assert!(parse_commit_history(b"\n", GitCommitGroup::Flat).unwrap().is_empty());
    }

    #[test]
    fn parses_commit_history_multiple_records_with_unicode() {
        let mut input = rec(&"a".repeat(40), "aaaaaaa", "Ñandú Pérez".as_bytes(), "1700000000", "fix: ¡acentos! | pipe".as_bytes());
        input.push(b'\n'); // git log adds a newline between records
        input.extend(rec(&"b".repeat(40), "bbbbbbb", b"Bob", "1600000000", "日本語 subject".as_bytes()));

        let got = parse_commit_history(&input, GitCommitGroup::BranchOnly).expect("parse");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].hash, "a".repeat(40));
        assert_eq!(got[0].short_hash, "aaaaaaa");
        assert_eq!(got[0].author, "Ñandú Pérez");
        assert_eq!(got[0].commit_time, 1_700_000_000);
        assert_eq!(got[0].subject, "fix: ¡acentos! | pipe");
        assert_eq!(got[0].group, GitCommitGroup::BranchOnly);
        assert_eq!(got[1].hash, "b".repeat(40));
        assert_eq!(got[1].author, "Bob");
        assert_eq!(got[1].subject, "日本語 subject");
        assert_eq!(got[1].commit_time, 1_600_000_000);
    }

    #[test]
    fn parses_commit_history_decodes_non_utf8_lossily() {
        let input = rec(&"c".repeat(40), "ccccccc", b"Jos\xe9", "1", b"caf\xe9 ok");
        let got = parse_commit_history(&input, GitCommitGroup::Flat).expect("never fails the page");
        assert_eq!(got.len(), 1);
        assert!(got[0].author.starts_with("Jos") && got[0].author.contains('\u{fffd}'));
        assert!(got[0].subject.contains('\u{fffd}') && got[0].subject.ends_with(" ok"));
    }

    #[test]
    fn parses_commit_history_rejects_malformed_records() {
        // too few fields
        assert!(parse_commit_history(b"abc\0def\x1e", GitCommitGroup::Flat).is_err());
        // non-numeric timestamp
        let bad_time = rec(&"d".repeat(40), "ddddddd", b"A", "not-a-number", b"s");
        assert!(parse_commit_history(&bad_time, GitCommitGroup::Flat).is_err());
        // truncated: the last record has no terminator
        let mut truncated = rec(&"e".repeat(40), "eeeeeee", b"A", "1", b"s");
        truncated.pop();
        assert!(parse_commit_history(&truncated, GitCommitGroup::Flat).is_err());
    }

    // ---- Integration tests: they drive the real `git` binary against temp repositories. ----

    fn git_sync(dir: &Path, args: &[&str]) {
        let out = std::process::Command::new("git")
            .args(["-c", "user.name=Trunks Test", "-c", "user.email=test@trunks.local"])
            .args(["-c", "commit.gpgsign=false", "-c", "init.defaultBranch=main"])
            .args(args)
            .current_dir(dir)
            .output()
            .expect("git must be installed to run integration tests");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// Creates `<tmp>/<name>` as a git repository with one commit on `main`.
    fn init_repo(tmp: &TempDir, name: &str) -> PathBuf {
        let repo = tmp.0.join(name);
        std::fs::create_dir_all(&repo).unwrap();
        git_sync(&repo, &["init", "-b", "main"]);
        git_sync(&repo, &["commit", "--allow-empty", "-m", "initial"]);
        repo.canonicalize().unwrap()
    }

    #[tokio::test]
    async fn validate_repo_returns_root_from_nested_directory() {
        let tmp = TempDir::new("validate");
        let repo = init_repo(&tmp, "repo con espacio ñ");
        let nested = repo.join("a/b");
        std::fs::create_dir_all(&nested).unwrap();

        let info = git_validate_repo(nested.to_string_lossy().into_owned())
            .await
            .expect("nested dir resolves to the root");
        assert_eq!(PathBuf::from(&info.path), repo);
        assert_eq!(info.name, "repo con espacio ñ");
    }

    #[tokio::test]
    async fn validate_repo_rejects_non_git_directory() {
        let tmp = TempDir::new("notgit");
        let err = git_validate_repo(tmp.0.to_string_lossy().into_owned())
            .await
            .expect_err("a plain directory is not a repository");
        assert!(err.contains("rev-parse"), "error should carry operation context: {err}");
    }

    #[tokio::test]
    async fn list_worktrees_reports_main_and_linked_worktree() {
        let tmp = TempDir::new("worktrees");
        let repo = init_repo(&tmp, "repo");
        let linked = tmp.0.join("árbol enlazado");
        git_sync(&repo, &["worktree", "add", "-b", "feat/x", linked.to_str().unwrap()]);

        let got = git_list_worktrees(repo.to_string_lossy().into_owned())
            .await
            .expect("worktree list");
        assert_eq!(got.len(), 2);
        assert_eq!(PathBuf::from(&got[0].path).canonicalize().unwrap(), repo);
        assert_eq!(got[0].branch.as_deref(), Some("main"));
        assert_eq!(
            PathBuf::from(&got[1].path).canonicalize().unwrap(),
            linked.canonicalize().unwrap()
        );
        assert_eq!(got[1].branch.as_deref(), Some("feat/x"));
    }

    #[tokio::test]
    async fn list_branches_marks_current_branch() {
        let tmp = TempDir::new("branches");
        let repo = init_repo(&tmp, "repo");
        git_sync(&repo, &["branch", "feat/ñandú"]);

        let got = git_list_branches(repo.to_string_lossy().into_owned())
            .await
            .expect("branch list");
        let main = got.iter().find(|b| b.name == "main").expect("main exists");
        let feat = got.iter().find(|b| b.name == "feat/ñandú").expect("feature exists");
        assert!(main.is_current);
        assert!(!feat.is_current);
        assert!(!main.is_remote && !feat.is_remote);
        assert_eq!(main.commit.len(), 40);
    }

    // ---- Worktree removal guard (pure) ----

    fn wt(path: &Path) -> GitWorktreeInfo {
        GitWorktreeInfo {
            path: path.to_string_lossy().into_owned(),
            head: None,
            branch: None,
            bare: false,
            locked: None,
            prunable: None,
        }
    }

    #[test]
    fn rejects_main_and_unlisted_worktree_removal() {
        let tmp = TempDir::new("removal-guard");
        let main = tmp.0.join("main ñ");
        let linked = tmp.0.join("linked");
        let other = tmp.0.join("other");
        for d in [&main, &linked, &other] {
            std::fs::create_dir_all(d).unwrap();
        }
        let list = vec![wt(&main), wt(&linked)];

        assert!(select_removable(&list, main.to_str().unwrap()).is_err(), "main worktree");
        assert!(select_removable(&list, other.to_str().unwrap()).is_err(), "unlisted path");
        let ok = select_removable(&list, linked.to_str().unwrap()).expect("linked is removable");
        assert_eq!(ok.path, linked.to_string_lossy());
    }

    #[cfg(unix)]
    #[test]
    fn removal_guard_matches_symlink_aliases() {
        let tmp = TempDir::new("removal-alias");
        let main = tmp.0.join("main");
        let linked = tmp.0.join("linked");
        std::fs::create_dir_all(&main).unwrap();
        std::fs::create_dir_all(&linked).unwrap();
        let main_alias = tmp.0.join("main-alias");
        let linked_alias = tmp.0.join("linked-alias");
        std::os::unix::fs::symlink(&main, &main_alias).unwrap();
        std::os::unix::fs::symlink(&linked, &linked_alias).unwrap();
        let list = vec![wt(&main), wt(&linked)];

        assert!(select_removable(&list, main_alias.to_str().unwrap()).is_err(), "alias of main");
        assert!(select_removable(&list, linked_alias.to_str().unwrap()).is_ok(), "alias of linked");
    }

    #[test]
    fn removal_args_respect_force_confirmation() {
        assert_eq!(removal_args("/x/y z", false), vec!["worktree", "remove", "--", "/x/y z"]);
        assert_eq!(
            removal_args("/x/y z", true),
            vec!["worktree", "remove", "--force", "--", "/x/y z"]
        );
    }

    // ---- create / remove integration ----

    fn s(p: &Path) -> String {
        p.to_string_lossy().into_owned()
    }

    #[tokio::test]
    async fn create_worktree_with_new_branch_returns_its_record() {
        let tmp = TempDir::new("create-new");
        let repo = init_repo(&tmp, "repo");
        let dest = tmp.0.join("nuevo árbol");

        let info = git_create_worktree(s(&repo), s(&dest), "feat/new".into(), true)
            .await
            .expect("create with new branch");
        assert_eq!(info.branch.as_deref(), Some("feat/new"));
        assert_eq!(
            PathBuf::from(&info.path).canonicalize().unwrap(),
            dest.canonicalize().unwrap()
        );
        assert!(dest.join(".git").exists());
    }

    #[tokio::test]
    async fn create_worktree_checks_out_existing_branch() {
        let tmp = TempDir::new("create-existing");
        let repo = init_repo(&tmp, "repo");
        git_sync(&repo, &["branch", "existing"]);
        let dest = tmp.0.join("wt");

        let info = git_create_worktree(s(&repo), s(&dest), "existing".into(), false)
            .await
            .expect("create from existing branch");
        assert_eq!(info.branch.as_deref(), Some("existing"));
    }

    #[tokio::test]
    async fn create_worktree_rejects_relative_destination() {
        let tmp = TempDir::new("create-relative");
        let repo = init_repo(&tmp, "repo");
        let err = git_create_worktree(s(&repo), "relative/dir".into(), "x".into(), true)
            .await
            .expect_err("relative destination must be refused");
        assert!(err.contains("absolute"), "{err}");
        assert!(!repo.join("relative").exists());
    }

    #[tokio::test]
    async fn create_worktree_rejects_option_like_branch() {
        let tmp = TempDir::new("create-option");
        let repo = init_repo(&tmp, "repo");
        let dest = tmp.0.join("wt");
        let err = git_create_worktree(s(&repo), s(&dest), "--detach".into(), false)
            .await
            .expect_err("option-like branch must be refused");
        assert!(err.contains("branch"), "{err}");
        assert!(!dest.exists());
    }

    #[tokio::test]
    async fn create_worktree_surfaces_git_error_for_branch_in_use() {
        let tmp = TempDir::new("create-inuse");
        let repo = init_repo(&tmp, "repo");
        let dest = tmp.0.join("wt");
        let err = git_create_worktree(s(&repo), s(&dest), "main".into(), true)
            .await
            .expect_err("main already exists");
        assert!(err.contains("git worktree add"), "{err}");
    }

    #[tokio::test]
    async fn remove_worktree_removes_a_clean_linked_worktree() {
        let tmp = TempDir::new("remove-clean");
        let repo = init_repo(&tmp, "repo");
        let dest = tmp.0.join("wt ñ");
        git_create_worktree(s(&repo), s(&dest), "feat/a".into(), true).await.unwrap();

        git_remove_worktree(s(&repo), s(&dest), false).await.expect("clean removal");
        assert!(!dest.exists());
    }

    #[tokio::test]
    async fn remove_worktree_requires_force_for_dirty_tree() {
        let tmp = TempDir::new("remove-dirty");
        let repo = init_repo(&tmp, "repo");
        let dest = tmp.0.join("wt");
        git_create_worktree(s(&repo), s(&dest), "feat/b".into(), true).await.unwrap();
        std::fs::write(dest.join("untracked.txt"), "work in progress").unwrap();

        let err = git_remove_worktree(s(&repo), s(&dest), false)
            .await
            .expect_err("git must refuse a dirty worktree");
        assert!(err.contains("git worktree remove"), "{err}");
        assert!(dest.join("untracked.txt").exists(), "data must survive the refusal");

        git_remove_worktree(s(&repo), s(&dest), true).await.expect("confirmed force removal");
        assert!(!dest.exists());
    }

    #[tokio::test]
    async fn remove_worktree_rejects_main_and_unlisted_paths() {
        let tmp = TempDir::new("remove-guard-int");
        let repo = init_repo(&tmp, "repo");
        let stray = tmp.0.join("stray");
        std::fs::create_dir_all(&stray).unwrap();

        let main_err = git_remove_worktree(s(&repo), s(&repo), true)
            .await
            .expect_err("main, even forced");
        assert!(main_err.contains("main worktree"), "{main_err}");
        let stray_err = git_remove_worktree(s(&repo), s(&stray), true)
            .await
            .expect_err("unlisted");
        assert!(stray_err.contains("not a worktree"), "{stray_err}");
        assert!(repo.join(".git").exists());
        assert!(stray.exists());
    }

    // ---- status ----

    #[test]
    fn status_error_suggests_upgrade_for_porcelain_rejection() {
        let msg = explain_status_error(
            "git status failed: fatal: unsupported porcelain version".to_string(),
        );
        assert!(msg.contains("2.11"), "should name the minimum Git version: {msg}");
        assert!(msg.to_lowercase().contains("upgrade"), "{msg}");

        let unrelated = explain_status_error("git status failed: fatal: not a git repository".into());
        assert!(!unrelated.to_lowercase().contains("upgrade"), "{unrelated}");
    }

    #[tokio::test]
    async fn get_status_reports_staged_unstaged_renamed_and_untracked() {
        let tmp = TempDir::new("status");
        let repo = init_repo(&tmp, "repo ñ");
        std::fs::write(repo.join("a.txt"), "one\n").unwrap();
        std::fs::write(repo.join("ñandú b.txt"), "one\n").unwrap();
        std::fs::write(repo.join("old.txt"), "stable content\nline 2\nline 3\n").unwrap();
        git_sync(&repo, &["add", "-A"]);
        git_sync(&repo, &["commit", "-m", "files"]);

        std::fs::write(repo.join("a.txt"), "one\ntwo\n").unwrap(); // unstaged
        std::fs::write(repo.join("ñandú b.txt"), "one\ntwo\n").unwrap(); // staged
        git_sync(&repo, &["add", "ñandú b.txt"]);
        git_sync(&repo, &["mv", "old.txt", "new name.txt"]); // staged rename
        std::fs::write(repo.join("sin seguimiento ü.txt"), "x").unwrap();

        let status = git_get_status(s(&repo)).await.expect("status");
        let find = |p: &str| {
            status
                .entries
                .iter()
                .find(|e| e.path == p)
                .unwrap_or_else(|| panic!("missing '{p}' in {:?}", status.entries))
        };
        let a = find("a.txt");
        assert_eq!((a.index_status, a.worktree_status), ('.', 'M'));
        let n = find("ñandú b.txt");
        assert_eq!((n.index_status, n.worktree_status), ('M', '.'));
        let r = find("new name.txt");
        assert_eq!((r.index_status, r.worktree_status), ('R', '.'));
        assert_eq!(r.original_path.as_deref(), Some("old.txt"));
        let u = find("sin seguimiento ü.txt");
        assert_eq!(u.kind, GitStatusKind::Untracked);
        assert_eq!(status.entries.len(), 4, "{:?}", status.entries);
    }

    #[tokio::test]
    async fn get_status_is_empty_for_a_clean_worktree() {
        let tmp = TempDir::new("status-clean");
        let repo = init_repo(&tmp, "repo");
        let status = git_get_status(s(&repo)).await.expect("status");
        assert!(status.entries.is_empty());
    }

    #[tokio::test]
    async fn get_status_fails_outside_a_repository() {
        let tmp = TempDir::new("status-nogit");
        let err = git_get_status(s(&tmp.0)).await.expect_err("not a repository");
        assert!(err.contains("git status"), "{err}");
    }

    // ---- file listing ----

    #[tokio::test]
    async fn lists_one_level_and_marks_symlinks_without_following() {
        let tmp = TempDir::new("files");
        let root = &tmp.0;
        std::fs::write(root.join("b.txt"), "b").unwrap();
        std::fs::write(root.join(".hidden"), "h").unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::create_dir_all(root.join("sub/deep")).unwrap();
        std::fs::write(root.join("sub/inner.txt"), "i").unwrap();
        std::fs::create_dir_all(root.join("ñ dir")).unwrap();

        let mut expected = vec![".hidden", "b.txt", "sub", "ñ dir"];
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join("sub"), root.join("link")).unwrap();
            expected.insert(2, "link");
        }

        let top = git_list_files(s(root), None).await.expect("top level");
        let names: Vec<&str> = top.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, expected, ".git hidden, hidden files kept, sorted by name");
        let sub = top.iter().find(|f| f.name == "sub").unwrap();
        assert!(sub.is_dir && !sub.is_symlink);
        assert_eq!(sub.path, "sub");
        #[cfg(unix)]
        {
            let link = top.iter().find(|f| f.name == "link").unwrap();
            assert!(link.is_symlink && !link.is_dir, "symlink to a dir is not followed");
        }

        let inner = git_list_files(s(root), Some("sub".into())).await.expect("one level down");
        let inner_paths: Vec<&str> = inner.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(inner_paths, vec!["sub/deep", "sub/inner.txt"]);
    }

    #[tokio::test]
    async fn lists_one_level_rejects_escapes_and_missing_directories() {
        let tmp = TempDir::new("files-errors");
        let missing = git_list_files(s(&tmp.0), Some("nope".into())).await.expect_err("missing dir");
        assert!(missing.contains("nope"), "error should name the directory: {missing}");
        assert!(git_list_files(s(&tmp.0), Some("/etc".into())).await.is_err());
        assert!(git_list_files(s(&tmp.0), Some("../x".into())).await.is_err());

        #[cfg(unix)]
        {
            let outside = TempDir::new("files-outside");
            std::os::unix::fs::symlink(&outside.0, tmp.0.join("link")).unwrap();
            assert!(git_list_files(s(&tmp.0), Some("link".into())).await.is_err());
        }
    }

    // ---- diff ----

    #[tokio::test]
    async fn get_diff_returns_staged_and_unstaged_changes_without_color() {
        let tmp = TempDir::new("diff-tracked");
        let repo = init_repo(&tmp, "repo");
        std::fs::write(repo.join("a.txt"), "one\n").unwrap();
        git_sync(&repo, &["add", "-A"]);
        git_sync(&repo, &["commit", "-m", "a"]);
        std::fs::write(repo.join("a.txt"), "two\n").unwrap();

        let unstaged = git_get_diff(s(&repo), Some("a.txt".into()), false).await.unwrap();
        assert!(unstaged.contains("-one") && unstaged.contains("+two"), "{unstaged}");
        assert!(!unstaged.contains('\x1b'), "no ANSI colour codes");
        assert_eq!(git_get_diff(s(&repo), None, false).await.unwrap(), unstaged);
        assert!(git_get_diff(s(&repo), Some("a.txt".into()), true).await.unwrap().is_empty());

        git_sync(&repo, &["add", "a.txt"]);
        let staged = git_get_diff(s(&repo), Some("a.txt".into()), true).await.unwrap();
        assert!(staged.contains("+two"), "{staged}");
        assert!(git_get_diff(s(&repo), Some("a.txt".into()), false).await.unwrap().is_empty());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn get_diff_treats_the_path_as_a_literal_pathspec() {
        let tmp = TempDir::new("diff-literal");
        let repo = init_repo(&tmp, "repo");
        std::fs::write(repo.join("a.txt"), "a1\n").unwrap();
        std::fs::write(repo.join("*.txt"), "star1\n").unwrap();
        git_sync(&repo, &["add", "-A"]);
        git_sync(&repo, &["commit", "-m", "files"]);
        std::fs::write(repo.join("a.txt"), "a2\n").unwrap();
        std::fs::write(repo.join("*.txt"), "star2\n").unwrap();

        let diff = git_get_diff(s(&repo), Some("*.txt".into()), false).await.unwrap();
        assert!(diff.contains("star2"), "{diff}");
        assert!(!diff.contains("a2"), "glob must not expand to a.txt: {diff}");
    }

    #[tokio::test]
    async fn get_diff_rejects_absolute_escaping_and_empty_paths() {
        let tmp = TempDir::new("diff-paths");
        let repo = init_repo(&tmp, "repo");
        let abs = git_get_diff(s(&repo), Some("/etc/passwd".into()), false).await.unwrap_err();
        assert!(abs.contains("relative"), "{abs}");
        let up = git_get_diff(s(&repo), Some("../x".into()), false).await.unwrap_err();
        assert!(up.contains("relative"), "{up}");
        let empty = git_get_diff(s(&repo), Some("".into()), false).await.unwrap_err();
        assert!(empty.contains("empty"), "{empty}");
    }

    #[tokio::test]
    async fn untracked_diff_adds_every_line_of_a_text_file() {
        let tmp = TempDir::new("untracked-text");
        let repo = init_repo(&tmp, "repo");
        std::fs::create_dir_all(repo.join("carpeta")).unwrap();
        std::fs::write(repo.join("carpeta/sin seguimiento ñ.txt"), "uno\ndos\ntres").unwrap();

        let diff = git_get_diff(s(&repo), Some("carpeta/sin seguimiento ñ.txt".into()), false)
            .await
            .expect("synthetic diff");
        let p = "carpeta/sin seguimiento ñ.txt";
        let expected = format!(
            "diff --git a/{p} b/{p}\nnew file mode 100644\n--- /dev/null\n+++ b/{p}\n\
             @@ -0,0 +1,3 @@\n+uno\n+dos\n+tres\n\\ No newline at end of file\n"
        );
        assert_eq!(diff, expected);
    }

    #[tokio::test]
    async fn untracked_diff_single_line_header_omits_the_count() {
        let tmp = TempDir::new("untracked-single");
        let repo = init_repo(&tmp, "repo");
        std::fs::write(repo.join("one.txt"), "solo\n").unwrap();

        let diff = git_get_diff(s(&repo), Some("one.txt".into()), false).await.unwrap();
        assert!(diff.contains("@@ -0,0 +1 @@\n+solo\n"), "{diff}");
        assert!(!diff.contains("No newline"), "{diff}");
    }

    #[tokio::test]
    async fn untracked_diff_of_an_empty_file_has_no_hunk() {
        let tmp = TempDir::new("untracked-empty");
        let repo = init_repo(&tmp, "repo");
        std::fs::write(repo.join("empty.txt"), "").unwrap();

        let diff = git_get_diff(s(&repo), Some("empty.txt".into()), false).await.unwrap();
        assert_eq!(
            diff,
            "diff --git a/empty.txt b/empty.txt\nnew file mode 100644\n"
        );
    }

    #[tokio::test]
    async fn untracked_diff_refuses_binary_files() {
        let tmp = TempDir::new("untracked-binary");
        let repo = init_repo(&tmp, "repo");
        std::fs::write(repo.join("img.bin"), [0x89, b'P', b'N', b'G', 0, 1, 2, 3]).unwrap();

        let err = git_get_diff(s(&repo), Some("img.bin".into()), false).await.unwrap_err();
        assert!(err.contains("binary"), "{err}");
    }

    #[tokio::test]
    async fn untracked_diff_refuses_files_over_the_size_limit() {
        let tmp = TempDir::new("untracked-large");
        let repo = init_repo(&tmp, "repo");
        let limit = MAX_UNTRACKED_DIFF_BYTES as usize;
        std::fs::write(repo.join("exact.txt"), vec![b'a'; limit]).unwrap();
        std::fs::write(repo.join("over.txt"), vec![b'a'; limit + 1]).unwrap();

        assert!(git_get_diff(s(&repo), Some("exact.txt".into()), false).await.is_ok(), "exactly 1 MiB is allowed");
        let err = git_get_diff(s(&repo), Some("over.txt".into()), false).await.unwrap_err();
        assert!(err.contains("too large"), "{err}");
    }

    #[tokio::test]
    async fn untracked_diff_is_not_synthesized_for_the_cached_view_or_ignored_files() {
        let tmp = TempDir::new("untracked-scope");
        let repo = init_repo(&tmp, "repo");
        std::fs::write(repo.join("new.txt"), "x\n").unwrap();
        std::fs::write(repo.join(".gitignore"), "ignored.txt\n").unwrap();
        std::fs::write(repo.join("ignored.txt"), "y\n").unwrap();

        assert_eq!(git_get_diff(s(&repo), Some("new.txt".into()), true).await.unwrap(), "");
        assert_eq!(git_get_diff(s(&repo), Some("ignored.txt".into()), false).await.unwrap(), "");
    }

    // ---- Commit history integration ----

    fn commit(repo: &Path, msg: &str) {
        git_sync(repo, &["commit", "--allow-empty", "-m", msg]);
    }

    async fn history(repo: &Path, base: Option<&str>) -> GitCommitHistoryPage {
        commit_history(repo.to_str().unwrap(), base, None, COMMIT_HISTORY_PAGE_SIZE)
            .await
            .expect("history")
    }

    fn subjects(page: &GitCommitHistoryPage, group: GitCommitGroup) -> Vec<&str> {
        page.commits
            .iter()
            .filter(|c| c.group == group)
            .map(|c| c.subject.as_str())
            .collect()
    }

    #[tokio::test]
    async fn commit_history_linear_branch_only_and_shared() {
        let tmp = TempDir::new("hist-linear");
        let repo = init_repo(&tmp, "repo");
        git_sync(&repo, &["checkout", "-b", "feat"]);
        commit(&repo, "one");
        commit(&repo, "two");

        let page = history(&repo, None).await;
        assert_eq!(page.branch.as_deref(), Some("feat"));
        assert_eq!(page.head_state, GitHeadState::Branch);
        assert_eq!(page.base.as_deref(), Some("main"));
        assert_eq!(page.base_source, GitBaseSource::Auto);
        assert!(!page.partial_history && page.next_cursor.is_none());
        assert_eq!(subjects(&page, GitCommitGroup::BranchOnly), ["two", "one"]);
        assert_eq!(subjects(&page, GitCommitGroup::Shared), ["initial"]);
        // Branch-only commits come before shared ones.
        assert_eq!(page.commits[0].group, GitCommitGroup::BranchOnly);
        assert_eq!(page.commits[2].group, GitCommitGroup::Shared);
    }

    #[tokio::test]
    async fn commit_history_merged_side_branch_classified_by_reachability() {
        let tmp = TempDir::new("hist-merge");
        let repo = init_repo(&tmp, "repo");
        git_sync(&repo, &["checkout", "-b", "side"]);
        commit(&repo, "side work");
        git_sync(&repo, &["checkout", "-b", "feat", "main"]);
        commit(&repo, "feat work");
        git_sync(&repo, &["merge", "--no-ff", "-m", "merge side", "side"]);

        let page = history(&repo, None).await;
        let only = subjects(&page, GitCommitGroup::BranchOnly);
        assert_eq!(only.len(), 3, "{only:?}");
        assert!(only.contains(&"side work") && only.contains(&"merge side"));
        assert_eq!(subjects(&page, GitCommitGroup::Shared), ["initial"]);
    }

    #[tokio::test]
    async fn commit_history_branch_equal_to_base_has_empty_branch_only() {
        let tmp = TempDir::new("hist-equal");
        let repo = init_repo(&tmp, "repo");
        commit(&repo, "second");

        let page = history(&repo, None).await;
        assert_eq!(page.base.as_deref(), Some("main"));
        assert!(subjects(&page, GitCommitGroup::BranchOnly).is_empty());
        assert_eq!(subjects(&page, GitCommitGroup::Shared), ["second", "initial"]);
    }

    #[tokio::test]
    async fn commit_history_without_detectable_base_is_flat() {
        let tmp = TempDir::new("hist-nobase");
        let repo = init_repo(&tmp, "repo");
        git_sync(&repo, &["branch", "-m", "main", "trunk"]);
        commit(&repo, "second");

        let page = history(&repo, None).await;
        assert_eq!(page.base, None);
        assert_eq!(page.base_source, GitBaseSource::Auto);
        assert_eq!(subjects(&page, GitCommitGroup::Flat), ["second", "initial"]);
        assert!(page.commits.iter().all(|c| c.group == GitCommitGroup::Flat));
    }

    #[tokio::test]
    async fn commit_history_stale_remote_head_does_not_become_base() {
        let tmp = TempDir::new("hist-stale");
        let repo = init_repo(&tmp, "repo");
        git_sync(&repo, &["branch", "-m", "main", "trunk"]);
        git_sync(&repo, &["remote", "add", "origin", "https://example.invalid/x.git"]);
        git_sync(&repo, &["config", "branch.trunk.remote", "origin"]);
        git_sync(&repo, &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/gone"]);

        let page = history(&repo, None).await;
        assert_eq!(page.base, None);
        assert_eq!(subjects(&page, GitCommitGroup::Flat), ["initial"]);
    }

    #[tokio::test]
    async fn commit_history_detached_head_is_flat() {
        let tmp = TempDir::new("hist-detached");
        let repo = init_repo(&tmp, "repo");
        commit(&repo, "second");
        git_sync(&repo, &["checkout", "--detach"]);

        let page = history(&repo, None).await;
        assert_eq!(page.head_state, GitHeadState::Detached);
        assert_eq!(page.branch, None);
        assert_eq!(page.base, None);
        assert_eq!(subjects(&page, GitCommitGroup::Flat), ["second", "initial"]);
    }

    #[tokio::test]
    async fn commit_history_unborn_branch_is_empty() {
        let tmp = TempDir::new("hist-unborn");
        let repo = tmp.0.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        git_sync(&repo, &["init", "-b", "main"]);

        let page = history(&repo.canonicalize().unwrap(), None).await;
        assert_eq!(page.head_state, GitHeadState::Unborn);
        assert_eq!(page.head_oid, None);
        assert!(page.commits.is_empty() && page.next_cursor.is_none());
    }

    #[tokio::test]
    async fn commit_history_shallow_clone_is_flat_and_partial() {
        let tmp = TempDir::new("hist-shallow");
        let src = init_repo(&tmp, "src");
        commit(&src, "second");
        commit(&src, "third");
        let url = format!("file://{}", src.display());
        git_sync(&tmp.0, &["clone", "--depth", "1", &url, "clone"]);

        let page = history(&tmp.0.join("clone").canonicalize().unwrap(), None).await;
        assert!(page.partial_history);
        assert_eq!(page.base, None, "no separator in a shallow repository");
        assert_eq!(subjects(&page, GitCommitGroup::Flat), ["third"]);
    }

    #[tokio::test]
    async fn commit_history_status_reports_head_oid() {
        let tmp = TempDir::new("hist-status");
        let repo = init_repo(&tmp, "repo");
        let status = git_get_status(repo.to_string_lossy().into_owned()).await.unwrap();
        let page = history(&repo, None).await;
        assert_eq!(status.head_oid.as_deref().map(str::len), Some(40));
        assert_eq!(status.head_oid, page.head_oid);

        let empty = tmp.0.join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        git_sync(&empty, &["init", "-b", "main"]);
        let status = git_get_status(empty.to_string_lossy().into_owned()).await.unwrap();
        assert_eq!(status.head_oid, None);
    }

    // ---- Commit history: pagination, reset, explicit base, cancellation ----

    async fn page_at(
        repo: &Path,
        base: Option<&str>,
        cursor: Option<&GitCommitCursor>,
        size: usize,
    ) -> GitCommitHistoryPage {
        commit_history(repo.to_str().unwrap(), base, cursor, size)
            .await
            .expect("history page")
    }

    fn write_commit(repo: &Path, file: &str, msg: &str) {
        std::fs::write(repo.join(file), msg).unwrap();
        git_sync(repo, &["add", file]);
        git_sync(repo, &["commit", "-m", msg]);
    }

    #[tokio::test]
    async fn commit_history_pagination_has_no_gaps_or_duplicates() {
        let tmp = TempDir::new("hist-pages");
        let repo = init_repo(&tmp, "repo");
        commit(&repo, "m2");
        commit(&repo, "m3");
        git_sync(&repo, &["checkout", "-b", "feat"]);
        for i in 1..=4 {
            commit(&repo, &format!("f{i}"));
        }
        let full = page_at(&repo, None, None, 100).await;
        assert_eq!(full.commits.len(), 7);
        for size in [1usize, 2, 3, 4, 5, 7] {
            let mut hashes = Vec::new();
            let mut groups = Vec::new();
            let mut cursor = None;
            loop {
                let p = page_at(&repo, None, cursor.as_ref(), size).await;
                assert!(!p.reset);
                assert!(p.commits.len() <= size);
                hashes.extend(p.commits.iter().map(|c| c.hash.clone()));
                groups.extend(p.commits.iter().map(|c| c.group));
                cursor = p.next_cursor;
                if cursor.is_none() {
                    break;
                }
            }
            let expected: Vec<_> = full.commits.iter().map(|c| c.hash.clone()).collect();
            assert_eq!(hashes, expected, "page size {size}");
            let expected_groups: Vec<_> = full.commits.iter().map(|c| c.group).collect();
            assert_eq!(groups, expected_groups, "page size {size}");
        }
        // An exact multiple must not leave a dangling empty page.
        let last = page_at(&repo, None, None, 7).await;
        assert!(last.next_cursor.is_none());
    }

    #[tokio::test]
    async fn commit_history_base_ahead_and_diverged() {
        let tmp = TempDir::new("hist-ahead");
        let repo = init_repo(&tmp, "repo");
        git_sync(&repo, &["checkout", "-b", "feat"]);
        git_sync(&repo, &["checkout", "main"]);
        commit(&repo, "main2");
        // Base ahead of HEAD: nothing branch-only; shared is HEAD's own ancestry.
        git_sync(&repo, &["checkout", "feat"]);
        let page = history(&repo, None).await;
        assert!(subjects(&page, GitCommitGroup::BranchOnly).is_empty());
        assert_eq!(subjects(&page, GitCommitGroup::Shared), ["initial"]);
        // Diverged: feat has its own commit and main moved on; main2 must not appear.
        commit(&repo, "feat1");
        let page = history(&repo, None).await;
        assert_eq!(subjects(&page, GitCommitGroup::BranchOnly), ["feat1"]);
        assert_eq!(subjects(&page, GitCommitGroup::Shared), ["initial"]);
    }

    #[tokio::test]
    async fn commit_history_unrelated_histories_have_empty_shared_group() {
        let tmp = TempDir::new("hist-unrelated");
        let repo = init_repo(&tmp, "repo");
        git_sync(&repo, &["checkout", "--orphan", "other"]);
        commit(&repo, "orphan root");
        let page = history(&repo, None).await;
        assert_eq!(page.base.as_deref(), Some("main"));
        assert_eq!(subjects(&page, GitCommitGroup::BranchOnly), ["orphan root"]);
        assert!(subjects(&page, GitCommitGroup::Shared).is_empty());
        assert!(page.next_cursor.is_none());
    }

    #[tokio::test]
    async fn commit_history_criss_cross_uses_all_merge_bases() {
        let tmp = TempDir::new("hist-crisscross");
        let repo = init_repo(&tmp, "repo");
        git_sync(&repo, &["checkout", "-b", "b1"]);
        commit(&repo, "B1");
        git_sync(&repo, &["checkout", "-b", "b2", "main"]);
        commit(&repo, "B2");
        git_sync(&repo, &["merge", "--no-ff", "-m", "M2", "b1"]);
        git_sync(&repo, &["checkout", "b1"]);
        git_sync(&repo, &["merge", "--no-ff", "-m", "M1", "b2^1"]);

        let page = history(&repo, Some("b2")).await;
        assert_eq!(subjects(&page, GitCommitGroup::BranchOnly), ["M1"]);
        let mut shared = subjects(&page, GitCommitGroup::Shared);
        shared.sort();
        assert_eq!(shared, ["B1", "B2", "initial"], "both merge bases contribute");
    }

    #[tokio::test]
    async fn commit_history_squash_merged_branch_stays_branch_only() {
        let tmp = TempDir::new("hist-squash");
        let repo = init_repo(&tmp, "repo");
        git_sync(&repo, &["checkout", "-b", "feat"]);
        write_commit(&repo, "a.txt", "feat a");
        write_commit(&repo, "b.txt", "feat b");
        git_sync(&repo, &["checkout", "main"]);
        git_sync(&repo, &["merge", "--squash", "feat"]);
        git_sync(&repo, &["commit", "-m", "squashed"]);
        git_sync(&repo, &["checkout", "feat"]);

        let page = history(&repo, None).await;
        assert_eq!(subjects(&page, GitCommitGroup::BranchOnly), ["feat b", "feat a"]);
        assert_eq!(subjects(&page, GitCommitGroup::Shared), ["initial"]);
    }

    #[tokio::test]
    async fn commit_history_cursor_mismatch_resets_to_first_page() {
        let tmp = TempDir::new("hist-reset");
        let repo = init_repo(&tmp, "repo");
        git_sync(&repo, &["checkout", "-b", "feat"]);
        commit(&repo, "f1");
        commit(&repo, "f2");
        commit(&repo, "f3");

        let first = page_at(&repo, None, None, 2).await;
        let cursor = first.next_cursor.clone().expect("more pages");
        assert!(!first.reset);

        // Same state: the cursor continues.
        let second = page_at(&repo, None, Some(&cursor), 2).await;
        assert!(!second.reset);
        assert_eq!(subjects(&second, GitCommitGroup::BranchOnly), ["f1"]);

        // HEAD advances: fresh first page plus reset.
        commit(&repo, "f4");
        let after_head = page_at(&repo, None, Some(&cursor), 2).await;
        assert!(after_head.reset);
        assert_eq!(after_head.commits[0].subject, "f4");
        assert_eq!(after_head.commits.len(), 2);

        // Base advances: also a reset.
        let cursor = after_head.next_cursor.clone().expect("more pages");
        git_sync(&repo, &["checkout", "main"]);
        commit(&repo, "main2");
        git_sync(&repo, &["checkout", "feat"]);
        let after_base = page_at(&repo, None, Some(&cursor), 2).await;
        assert!(after_base.reset);
        assert_eq!(after_base.commits[0].subject, "f4");
    }

    #[tokio::test]
    async fn commit_history_explicit_base_overrides_detection() {
        let tmp = TempDir::new("hist-explicit");
        let repo = init_repo(&tmp, "repo");
        git_sync(&repo, &["checkout", "-b", "develop"]);
        commit(&repo, "dev1");
        git_sync(&repo, &["checkout", "-b", "feat"]);
        commit(&repo, "f1");
        // origin/HEAD points at main, as in a gitflow repository.
        git_sync(&repo, &["remote", "add", "origin", "https://example.invalid/x.git"]);
        git_sync(&repo, &["update-ref", "refs/remotes/origin/main", "main"]);
        git_sync(&repo, &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"]);
        git_sync(&repo, &["config", "branch.feat.remote", "origin"]);

        let auto = history(&repo, None).await;
        assert_eq!(auto.base.as_deref(), Some("origin/main"));
        assert_eq!(auto.base_source, GitBaseSource::Auto);
        assert_eq!(subjects(&auto, GitCommitGroup::BranchOnly), ["f1", "dev1"]);

        let explicit = history(&repo, Some("develop")).await;
        assert_eq!(explicit.base.as_deref(), Some("develop"));
        assert_eq!(explicit.base_source, GitBaseSource::Explicit);
        assert_eq!(subjects(&explicit, GitCommitGroup::BranchOnly), ["f1"]);
        assert_eq!(subjects(&explicit, GitCommitGroup::Shared), ["dev1", "initial"]);

        // A remote branch is also a valid explicit base.
        let remote = history(&repo, Some("origin/main")).await;
        assert_eq!(remote.base.as_deref(), Some("origin/main"));
        assert_eq!(remote.base_source, GitBaseSource::Explicit);
    }

    #[tokio::test]
    async fn commit_history_rejects_invalid_explicit_base() {
        let tmp = TempDir::new("hist-badbase");
        let repo = init_repo(&tmp, "repo");
        git_sync(&repo, &["tag", "v1"]);
        let path = repo.to_str().unwrap();
        for bad in ["nope", "v1", "HEAD", "--all", "refs/heads/main", ""] {
            let r = commit_history(path, Some(bad), None, 10).await;
            assert!(r.is_err(), "base {bad:?} must be rejected, got {r:?}");
        }
        // The valid name still works and no state leaked from the failures.
        assert_eq!(history(&repo, Some("main")).await.base.as_deref(), Some("main"));
    }

    #[tokio::test]
    async fn commit_history_dash_prefixed_ref_is_not_an_option() {
        let tmp = TempDir::new("hist-dash");
        let repo = init_repo(&tmp, "repo");
        git_sync(&repo, &["checkout", "-b", "feat"]);
        commit(&repo, "f1");
        // A remote named `-r` yields the branch name `-r/main`.
        git_sync(&repo, &["update-ref", "refs/remotes/-r/main", "main"]);

        let page = history(&repo, Some("-r/main")).await;
        assert_eq!(page.base.as_deref(), Some("-r/main"));
        assert_eq!(subjects(&page, GitCommitGroup::BranchOnly), ["f1"]);
        assert_eq!(subjects(&page, GitCommitGroup::Shared), ["initial"]);
    }

    #[tokio::test]
    async fn commit_history_cancel_and_supersede_abort_the_pending_work() {
        use std::time::Duration;
        let slow = || async {
            tokio::time::sleep(Duration::from_secs(30)).await;
            Ok::<u8, String>(1)
        };

        // Explicit cancellation by request id.
        let task = tokio::spawn(run_cancellable("/wt/cancel", "req-a", slow()));
        tokio::time::sleep(Duration::from_millis(50)).await;
        git_cancel_commit_history("req-a".to_string()).await.unwrap();
        let r = tokio::time::timeout(Duration::from_secs(5), task).await.expect("aborted").unwrap();
        assert_eq!(r, Err(CANCELLED.to_string()));

        // A newer request for the same worktree supersedes the older one.
        let old = tokio::spawn(run_cancellable("/wt/supersede", "req-old", slow()));
        tokio::time::sleep(Duration::from_millis(50)).await;
        let newer = run_cancellable("/wt/supersede", "req-new", async { Ok::<u8, String>(2) }).await;
        assert_eq!(newer, Ok(2));
        let r = tokio::time::timeout(Duration::from_secs(5), old).await.expect("aborted").unwrap();
        assert_eq!(r, Err(CANCELLED.to_string()));

        // Other worktrees are untouched.
        let other = run_cancellable("/wt/other", "req-o", async { Ok::<u8, String>(3) }).await;
        assert_eq!(other, Ok(3));
        assert!(in_flight().lock().unwrap().is_empty() || !in_flight().lock().unwrap().contains_key("req-o"));
    }
}
