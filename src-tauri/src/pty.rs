use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter, State};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PtyOutputPayload {
    pub id: String,
    pub data: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PtyExitPayload {
    pub id: String,
    pub exit_code: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SystemInfo {
    pub os: String,
    pub default_shell: String,
    pub home_dir: Option<String>,
}

struct PtySession {
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

pub struct PtyManager {
    sessions: Arc<Mutex<HashMap<String, PtySession>>>,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

fn get_default_shell_path() -> String {
    #[cfg(target_os = "windows")]
    {
        std::env::var("COMSPEC").unwrap_or_else(|_| "powershell.exe".to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())
    }
}

#[tauri::command]
pub fn get_system_info() -> Result<SystemInfo, String> {
    let os = std::env::consts::OS.to_string();
    let default_shell = get_default_shell_path();
    let home_dir = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).ok();

    Ok(SystemInfo {
        os,
        default_shell,
        home_dir,
    })
}

#[tauri::command]
pub fn pty_create(
    app: AppHandle,
    state: State<'_, PtyManager>,
    id: String,
    cwd: Option<String>,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let pty_system = native_pty_system();
    let size = PtySize {
        rows: if rows > 0 { rows } else { 24 },
        cols: if cols > 0 { cols } else { 80 },
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = pty_system
        .openpty(size)
        .map_err(|e| format!("Failed to open PTY: {}", e))?;

    let shell = get_default_shell_path();
    let mut cmd = CommandBuilder::new(&shell);

    // Common terminal environment variables
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");

    if let Some(ref dir) = cwd {
        let path = Path::new(dir);
        if path.exists() {
            cmd.cwd(path);
        }
    }

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("Failed to spawn command '{}': {}", shell, e))?;

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("Failed to clone PTY reader: {}", e))?;

    let writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("Failed to take PTY writer: {}", e))?;

    let master = pair.master;

    let session = PtySession {
        writer,
        master,
        child,
    };

    {
        let mut sessions = state.sessions.lock().unwrap();
        sessions.insert(id.clone(), session);
    }

    // Spawn reader thread
    let app_handle = app.clone();
    let session_id = id.clone();
    let sessions_map = Arc::clone(&state.sessions);

    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    // EOF reached
                    break;
                }
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buf[..n]).to_string();
                    let payload = PtyOutputPayload {
                        id: session_id.clone(),
                        data: text,
                    };
                    let _ = app_handle.emit("pty-output", payload);
                }
                Err(_) => {
                    // Read error / process exit
                    break;
                }
            }
        }

        // Clean up session if still present
        {
            let mut sessions = sessions_map.lock().unwrap();
            sessions.remove(&session_id);
        }

        let exit_payload = PtyExitPayload {
            id: session_id,
            exit_code: None,
        };
        let _ = app_handle.emit("pty-exit", exit_payload);
    });

    Ok(())
}

#[tauri::command]
pub fn pty_write(
    state: State<'_, PtyManager>,
    id: String,
    data: String,
) -> Result<(), String> {
    let mut sessions = state.sessions.lock().unwrap();
    if let Some(session) = sessions.get_mut(&id) {
        session
            .writer
            .write_all(data.as_bytes())
            .map_err(|e| format!("Failed to write to PTY: {}", e))?;
        session
            .writer
            .flush()
            .map_err(|e| format!("Failed to flush PTY: {}", e))?;
        Ok(())
    } else {
        Err(format!("PTY session '{}' not found", id))
    }
}

#[tauri::command]
pub fn pty_resize(
    state: State<'_, PtyManager>,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let sessions = state.sessions.lock().unwrap();
    if let Some(session) = sessions.get(&id) {
        let size = PtySize {
            rows: if rows > 0 { rows } else { 24 },
            cols: if cols > 0 { cols } else { 80 },
            pixel_width: 0,
            pixel_height: 0,
        };
        session
            .master
            .resize(size)
            .map_err(|e| format!("Failed to resize PTY: {}", e))?;
        Ok(())
    } else {
        Err(format!("PTY session '{}' not found", id))
    }
}

#[tauri::command]
pub fn pty_kill(
    state: State<'_, PtyManager>,
    id: String,
) -> Result<(), String> {
    let mut sessions = state.sessions.lock().unwrap();
    if let Some(mut session) = sessions.remove(&id) {
        let _ = session.child.kill();
        drop(session);
        Ok(())
    } else {
        Ok(())
    }
}

#[tauri::command]
pub fn pty_list(state: State<'_, PtyManager>) -> Result<Vec<String>, String> {
    let sessions = state.sessions.lock().unwrap();
    Ok(sessions.keys().cloned().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_system_info() {
        let info = get_system_info().expect("system info should succeed");
        assert!(!info.os.is_empty());
        assert!(!info.default_shell.is_empty());
    }

    #[test]
    fn test_pty_lifecycle_direct() {
        let pty_system = native_pty_system();
        let size = PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system.openpty(size).expect("openpty must succeed");
        let shell = get_default_shell_path();
        let cmd = CommandBuilder::new(&shell);

        let mut child = pair.slave.spawn_command(cmd).expect("spawn command must succeed");
        assert!(child.process_id().is_some());

        let mut writer = pair.master.take_writer().expect("take_writer must succeed");
        let mut reader = pair.master.try_clone_reader().expect("clone_reader must succeed");

        // Write command and newline
        writer.write_all(b"echo 'PTY_TEST_OK'\n").expect("write must succeed");
        writer.flush().expect("flush must succeed");

        // Resize
        let new_size = PtySize {
            rows: 40,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        };
        pair.master.resize(new_size).expect("resize must succeed");

        // Read output until expected marker or timeout
        let mut output = String::new();
        let mut buf = [0u8; 1024];
        for _ in 0..10 {
            if let Ok(n) = reader.read(&mut buf) {
                if n > 0 {
                    output.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if output.contains("PTY_TEST_OK") {
                        break;
                    }
                }
            }
        }

        assert!(
            output.contains("PTY_TEST_OK"),
            "Expected output to contain 'PTY_TEST_OK', got: {}",
            output
        );

        let _ = child.kill();
    }

    #[test]
    fn test_pty_cwd_with_spaces_and_unicode() {
        let temp_dir = std::env::temp_dir().join("senda_spike_test_espacio y acento ñ");
        let _ = fs::create_dir_all(&temp_dir);

        let pty_system = native_pty_system();
        let size = PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system.openpty(size).expect("openpty must succeed");
        let shell = get_default_shell_path();
        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(&temp_dir);

        let mut child = pair.slave.spawn_command(cmd).expect("spawn with unicode cwd must succeed");
        let mut writer = pair.master.take_writer().expect("take_writer must succeed");
        let mut reader = pair.master.try_clone_reader().expect("clone_reader must succeed");

        writer.write_all(b"pwd\n").expect("write must succeed");
        writer.flush().expect("flush must succeed");

        let mut output = String::new();
        let mut buf = [0u8; 1024];
        for _ in 0..10 {
            if let Ok(n) = reader.read(&mut buf) {
                if n > 0 {
                    output.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if output.contains("senda_spike_test_espacio y acento ñ") {
                        break;
                    }
                }
            }
        }

        let _ = child.kill();
        let _ = fs::remove_dir_all(&temp_dir);

        assert!(
            output.contains("senda_spike_test_espacio y acento ñ"),
            "Expected output to contain unicode cwd, got: {}",
            output
        );
    }
}
