pub mod session;

use crate::error::{AppError, AppResult};
use crate::pty::session::PtySession;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc::Receiver,
    Arc, Mutex,
};

static NEXT_PTY_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PtySpawnDto {
    pub session_id: String,
    pub program: String,
    pub cwd: String,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PtyOutputEvent {
    pub session_id: String,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PtyExitEvent {
    pub session_id: String,
    pub code: Option<i32>,
}

/// Resolves the user's preferred shell and login/interactive arguments (`-l -i`).
pub fn resolve_default_shell(custom_shell: Option<&str>) -> (String, Vec<String>) {
    let shell_prog = custom_shell
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            env::var("SHELL")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| {
            if Path::new("/bin/bash").exists() {
                "/bin/bash".to_string()
            } else {
                "/bin/sh".to_string()
            }
        });

    let args = vec!["-l".to_string(), "-i".to_string()];
    (shell_prog, args)
}

pub fn generate_session_id() -> String {
    let seq = NEXT_PTY_ID.fetch_add(1, Ordering::Relaxed);
    format!("pty-{}", seq)
}

pub fn spawn_session_entry(
    sessions: &Mutex<HashMap<String, Arc<PtySession>>>,
    requested_id: Option<String>,
    program: &str,
    args: &[&str],
    cwd: &Path,
    cols: u16,
    rows: u16,
) -> AppResult<(PtySpawnDto, Arc<PtySession>, Receiver<String>)> {
    let safe_cols = cols.max(2);
    let safe_rows = rows.max(2);
    let session_id = requested_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(generate_session_id);

    let previous = {
        let mut guard = sessions.lock().map_err(|_| AppError::LockPoisoned)?;
        guard.remove(&session_id)
    };
    if let Some(old_session) = previous {
        let _ = old_session.kill();
    }

    let (session, rx) = PtySession::spawn(program, args, cwd, safe_cols, safe_rows)?;
    let arc_session = Arc::new(session);

    {
        let mut guard = sessions.lock().map_err(|_| AppError::LockPoisoned)?;
        guard.insert(session_id.clone(), Arc::clone(&arc_session));
    }

    let dto = PtySpawnDto {
        session_id,
        program: program.to_string(),
        cwd: cwd.to_string_lossy().into_owned(),
        cols: safe_cols,
        rows: safe_rows,
    };

    Ok((dto, arc_session, rx))
}

pub fn write_session_entry(
    sessions: &Mutex<HashMap<String, Arc<PtySession>>>,
    session_id: &str,
    data: &[u8],
) -> AppResult<()> {
    let session = {
        let guard = sessions.lock().map_err(|_| AppError::LockPoisoned)?;
        guard
            .get(session_id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("pty session not found: {}", session_id)))?
    };
    session.write(data)?;
    Ok(())
}

pub fn resize_session_entry(
    sessions: &Mutex<HashMap<String, Arc<PtySession>>>,
    session_id: &str,
    cols: u16,
    rows: u16,
) -> AppResult<()> {
    let session = {
        let guard = sessions.lock().map_err(|_| AppError::LockPoisoned)?;
        guard
            .get(session_id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("pty session not found: {}", session_id)))?
    };
    session.resize(cols.max(2), rows.max(2))?;
    Ok(())
}

pub fn kill_session_entry(
    sessions: &Mutex<HashMap<String, Arc<PtySession>>>,
    session_id: &str,
) -> AppResult<()> {
    let removed = {
        let mut guard = sessions.lock().map_err(|_| AppError::LockPoisoned)?;
        guard.remove(session_id)
    };
    if let Some(session) = removed {
        session.kill()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn resolves_default_shell_with_login_interactive_flags() {
        let (prog, args) = resolve_default_shell(Some("/bin/bash"));
        assert_eq!(prog, "/bin/bash");
        assert_eq!(args, vec!["-l".to_string(), "-i".to_string()]);
    }

    #[test]
    fn manages_pty_session_lifecycle_in_map() -> AppResult<()> {
        let tmp = TempDir::new()?;
        let map: Mutex<HashMap<String, Arc<PtySession>>> = Mutex::new(HashMap::new());

        let (dto, _arc, rx) =
            spawn_session_entry(&map, Some("test-1".into()), "sh", &[], tmp.path(), 80, 24)?;
        assert_eq!(dto.session_id, "test-1");

        resize_session_entry(&map, "test-1", 100, 30)?;
        write_session_entry(&map, "test-1", b"echo pty_ready\nexit\n")?;

        let mut collected = String::new();
        while let Ok(chunk) = rx.recv() {
            collected.push_str(&chunk);
            if collected.contains("pty_ready") {
                break;
            }
        }
        assert!(collected.contains("pty_ready"));

        kill_session_entry(&map, "test-1")?;
        assert!(map.lock().map_err(|_| AppError::LockPoisoned)?.is_empty());
        Ok(())
    }
}
