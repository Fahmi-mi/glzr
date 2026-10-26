use crate::error::{AppError, AppResult};
use crate::fs::watcher::WorkspaceWatcher;
use crate::pty::session::PtySession;
use crate::search::{cancel_active_search, ActiveSearchHandle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const APP_DIR_NAME: &str = "glzr";
const DEFAULT_RECENT_LIMIT: usize = 5;
pub const CURRENT_SESSION_VERSION: u64 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStateDto {
    pub active_folder: Option<String>,
    pub folder_name: Option<String>,
    pub recent_folders: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionLoadDto {
    pub raw_json: Option<String>,
    pub backed_up_corrupt: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct RecentFile {
    folders: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct XdgPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub state_dir: PathBuf,
}

impl XdgPaths {
    pub fn resolve() -> AppResult<Self> {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| AppError::NotFound("HOME environment variable not set".into()))?;

        Ok(Self::from_base_env(
            &home,
            env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
            env::var_os("XDG_DATA_HOME").map(PathBuf::from),
            env::var_os("XDG_CACHE_HOME").map(PathBuf::from),
            env::var_os("XDG_STATE_HOME").map(PathBuf::from),
        ))
    }

    pub fn from_base_env(
        home: &Path,
        xdg_config: Option<PathBuf>,
        xdg_data: Option<PathBuf>,
        xdg_cache: Option<PathBuf>,
        xdg_state: Option<PathBuf>,
    ) -> Self {
        let config_base = xdg_config
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".config"));
        let data_base = xdg_data
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".local").join("share"));
        let cache_base = xdg_cache
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".cache"));
        let state_base = xdg_state
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".local").join("state"));

        Self {
            config_dir: config_base.join(APP_DIR_NAME),
            data_dir: data_base.join(APP_DIR_NAME),
            cache_dir: cache_base.join(APP_DIR_NAME),
            state_dir: state_base.join(APP_DIR_NAME),
        }
    }

    pub fn recent_file_path(&self) -> PathBuf {
        self.data_dir.join("recent.json")
    }

    pub fn session_file_path(&self) -> PathBuf {
        self.data_dir.join("session.json")
    }

    pub fn session_backup_path(&self) -> PathBuf {
        self.data_dir.join("session.json.bak")
    }
}

#[derive(Debug, Default)]
pub struct WorkspaceData {
    pub active_folder: Option<PathBuf>,
    pub recent_folders: Vec<String>,
}

pub struct AppState {
    pub xdg: XdgPaths,
    pub workspace: Mutex<WorkspaceData>,
    pub pty_sessions: Mutex<HashMap<String, Arc<PtySession>>>,
    pub fs_watcher: Mutex<Option<WorkspaceWatcher>>,
    pub active_search: Mutex<Option<ActiveSearchHandle>>,
}

impl AppState {
    pub fn new(xdg: XdgPaths, initial_folder: Option<PathBuf>) -> Self {
        let mut recent_folders = load_recent_folders(&xdg.recent_file_path()).unwrap_or_default();
        let mut active_folder = None;

        if let Some(folder) = initial_folder {
            if let Ok(canonical) = validate_project_dir(&folder) {
                let canonical_str = canonical.to_string_lossy().into_owned();
                push_recent_folder(&mut recent_folders, &canonical_str, DEFAULT_RECENT_LIMIT);
                let _ = save_recent_folders(&xdg.recent_file_path(), &recent_folders);
                active_folder = Some(canonical);
            }
        } else if let Some(saved_folder) = peek_session_folder(&xdg.session_file_path()) {
            if let Ok(canonical) = validate_project_dir(&saved_folder) {
                active_folder = Some(canonical);
            }
        }

        Self {
            xdg,
            workspace: Mutex::new(WorkspaceData {
                active_folder,
                recent_folders,
            }),
            pty_sessions: Mutex::new(HashMap::new()),
            fs_watcher: Mutex::new(None),
            active_search: Mutex::new(None),
        }
    }

    pub fn get_project_state(&self) -> AppResult<ProjectStateDto> {
        let guard = self.workspace.lock().map_err(|_| AppError::LockPoisoned)?;
        Ok(build_project_dto(&guard))
    }

    pub fn active_folder_path(&self) -> AppResult<PathBuf> {
        let guard = self.workspace.lock().map_err(|_| AppError::LockPoisoned)?;
        guard
            .active_folder
            .clone()
            .ok_or_else(|| AppError::Validation("no active project folder".into()))
    }

    pub fn open_folder(&self, raw_path: &Path) -> AppResult<ProjectStateDto> {
        let canonical = validate_project_dir(raw_path)?;
        let canonical_str = canonical.to_string_lossy().into_owned();

        let (dto, to_save) = {
            let mut guard = self.workspace.lock().map_err(|_| AppError::LockPoisoned)?;
            guard.active_folder = Some(canonical);
            push_recent_folder(
                &mut guard.recent_folders,
                &canonical_str,
                DEFAULT_RECENT_LIMIT,
            );
            (build_project_dto(&guard), guard.recent_folders.clone())
        };

        save_recent_folders(&self.xdg.recent_file_path(), &to_save)?;
        Ok(dto)
    }

    pub fn remove_recent_folder(&self, folder_path: &str) -> AppResult<ProjectStateDto> {
        let (dto, to_save) = {
            let mut guard = self.workspace.lock().map_err(|_| AppError::LockPoisoned)?;
            guard.recent_folders.retain(|item| item != folder_path);
            (build_project_dto(&guard), guard.recent_folders.clone())
        };

        save_recent_folders(&self.xdg.recent_file_path(), &to_save)?;
        Ok(dto)
    }

    pub fn kill_all_pty_sessions(&self) {
        if let Ok(mut map) = self.pty_sessions.lock() {
            for (_, session) in map.drain() {
                let _ = session.kill();
            }
        }
    }

    pub fn stop_watcher(&self) {
        if let Ok(mut guard) = self.fs_watcher.lock() {
            if let Some(mut watcher) = guard.take() {
                watcher.stop();
            }
        }
    }

    pub fn cancel_search(&self) {
        cancel_active_search(&self.active_search);
    }

    pub fn load_session(&self) -> AppResult<SessionLoadDto> {
        let session_path = self.xdg.session_file_path();
        if !session_path.exists() {
            return Ok(SessionLoadDto {
                raw_json: None,
                backed_up_corrupt: false,
            });
        }

        let raw = match fs::read_to_string(&session_path) {
            Ok(s) => s,
            Err(_) => {
                let _ = fs::copy(&session_path, self.xdg.session_backup_path());
                return Ok(SessionLoadDto {
                    raw_json: None,
                    backed_up_corrupt: true,
                });
            }
        };

        let parsed: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => {
                let _ = fs::copy(&session_path, self.xdg.session_backup_path());
                return Ok(SessionLoadDto {
                    raw_json: None,
                    backed_up_corrupt: true,
                });
            }
        };

        let version = parsed.get("version").and_then(|v| v.as_u64()).unwrap_or(0);
        if version == 0 || version > CURRENT_SESSION_VERSION {
            let _ = fs::copy(&session_path, self.xdg.session_backup_path());
            return Ok(SessionLoadDto {
                raw_json: None,
                backed_up_corrupt: true,
            });
        }

        Ok(SessionLoadDto {
            raw_json: Some(raw),
            backed_up_corrupt: false,
        })
    }

    pub fn save_session(&self, raw_json: &str) -> AppResult<()> {
        let parsed: serde_json::Value = serde_json::from_str(raw_json)?;
        let version = parsed.get("version").and_then(|v| v.as_u64()).unwrap_or(0);
        if version == 0 || version > CURRENT_SESSION_VERSION {
            return Err(AppError::Validation(format!(
                "unsupported session version: {}",
                version
            )));
        }
        let formatted = serde_json::to_vec_pretty(&parsed)?;
        atomic_write_bytes(&self.xdg.session_file_path(), &formatted)
    }
}

fn peek_session_folder(session_path: &Path) -> Option<PathBuf> {
    let raw = fs::read_to_string(session_path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let version = parsed.get("version")?.as_u64()?;
    if version == 0 || version > CURRENT_SESSION_VERSION {
        return None;
    }
    let folder = parsed.get("folder")?.as_str()?.trim();
    if folder.is_empty() {
        return None;
    }
    Some(PathBuf::from(folder))
}

fn build_project_dto(data: &WorkspaceData) -> ProjectStateDto {
    let active_folder = data
        .active_folder
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned());
    let folder_name = data
        .active_folder
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned());

    ProjectStateDto {
        active_folder,
        folder_name,
        recent_folders: data.recent_folders.clone(),
    }
}

pub fn validate_project_dir(path: &Path) -> AppResult<PathBuf> {
    if !path.exists() {
        return Err(AppError::NotFound(path.display().to_string()));
    }
    let canonical = path.canonicalize()?;
    if !canonical.is_dir() {
        return Err(AppError::InvalidPath(format!(
            "not a directory: {}",
            canonical.display()
        )));
    }
    Ok(canonical)
}

pub fn push_recent_folder(list: &mut Vec<String>, folder: &str, limit: usize) {
    list.retain(|existing| existing != folder);
    list.insert(0, folder.to_string());
    if list.len() > limit {
        list.truncate(limit);
    }
}

pub fn atomic_write_bytes(target_path: &Path, bytes: &[u8]) -> AppResult<()> {
    let parent = target_path
        .parent()
        .ok_or_else(|| AppError::InvalidPath(target_path.display().to_string()))?;
    fs::create_dir_all(parent)?;

    let file_name = target_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("tmp");
    let temp_path = parent.join(format!(".{}.tmp.{}", file_name, std::process::id()));

    {
        let mut file = fs::File::create(&temp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }

    fs::rename(&temp_path, target_path)?;
    Ok(())
}

fn load_recent_folders(path: &Path) -> AppResult<Vec<String>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(path)?;
    let parsed: RecentFile = serde_json::from_str(&raw)?;
    Ok(parsed.folders)
}

fn save_recent_folders(path: &Path, folders: &[String]) -> AppResult<()> {
    let payload = RecentFile {
        folders: folders.to_vec(),
    };
    let serialized = serde_json::to_vec_pretty(&payload)?;
    atomic_write_bytes(path, &serialized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn xdg_paths_use_defaults_and_respect_overrides() {
        let home = PathBuf::from("/home/tester");
        let defaults = XdgPaths::from_base_env(&home, None, None, None, None);
        assert_eq!(
            defaults.config_dir,
            PathBuf::from("/home/tester/.config/glzr")
        );
        assert_eq!(
            defaults.data_dir,
            PathBuf::from("/home/tester/.local/share/glzr")
        );
        assert_eq!(
            defaults.cache_dir,
            PathBuf::from("/home/tester/.cache/glzr")
        );
        assert_eq!(
            defaults.state_dir,
            PathBuf::from("/home/tester/.local/state/glzr")
        );

        let custom = XdgPaths::from_base_env(
            &home,
            Some(PathBuf::from("/tmp/cfg")),
            Some(PathBuf::from("/tmp/data")),
            None,
            None,
        );
        assert_eq!(custom.config_dir, PathBuf::from("/tmp/cfg/glzr"));
        assert_eq!(custom.data_dir, PathBuf::from("/tmp/data/glzr"));
    }

    #[test]
    fn recent_folders_deduplicate_and_cap_at_limit() -> AppResult<()> {
        let tmp = TempDir::new()?;
        let xdg = XdgPaths::from_base_env(
            tmp.path(),
            Some(tmp.path().join("config")),
            Some(tmp.path().join("data")),
            Some(tmp.path().join("cache")),
            Some(tmp.path().join("state")),
        );

        let state = AppState::new(xdg.clone(), None);
        for idx in 1..=6 {
            let dir = tmp.path().join(format!("proj-{}", idx));
            fs::create_dir_all(&dir)?;
            state.open_folder(&dir)?;
        }

        let current = state.get_project_state()?;
        assert_eq!(current.recent_folders.len(), 5);
        assert!(current.recent_folders[0].ends_with("proj-6"));

        let proj3 = tmp.path().join("proj-3");
        let updated = state.open_folder(&proj3)?;
        assert_eq!(updated.recent_folders.len(), 5);
        assert!(updated.recent_folders[0].ends_with("proj-3"));
        assert_eq!(updated.folder_name.as_deref(), Some("proj-3"));

        let reloaded = AppState::new(xdg, None);
        assert_eq!(reloaded.get_project_state()?.recent_folders.len(), 5);
        Ok(())
    }

    #[test]
    fn session_save_and_load_backs_up_incompatible_or_corrupt_files() -> AppResult<()> {
        let tmp = TempDir::new()?;
        let xdg = XdgPaths::from_base_env(
            tmp.path(),
            Some(tmp.path().join("config")),
            Some(tmp.path().join("data")),
            Some(tmp.path().join("cache")),
            Some(tmp.path().join("state")),
        );

        let proj = tmp.path().join("my-proj");
        fs::create_dir_all(&proj)?;
        let proj_str = proj.canonicalize()?.to_string_lossy().into_owned();

        let state = AppState::new(xdg.clone(), None);
        let initial = state.load_session()?;
        assert!(initial.raw_json.is_none());
        assert!(!initial.backed_up_corrupt);

        let valid_json = format!(
            r#"{{"version":1,"folder":"{}","activeWorkspaceId":"ws-1","workspaces":[]}}"#,
            proj_str
        );
        state.save_session(&valid_json)?;

        let loaded = state.load_session()?;
        assert!(loaded.raw_json.is_some());
        assert!(!loaded.backed_up_corrupt);

        let restored_state = AppState::new(xdg.clone(), None);
        assert_eq!(
            restored_state.get_project_state()?.active_folder.as_deref(),
            Some(proj_str.as_str())
        );

        fs::write(xdg.session_file_path(), r#"{"version":99}"#)?;
        let future_ver = state.load_session()?;
        assert!(future_ver.raw_json.is_none());
        assert!(future_ver.backed_up_corrupt);
        assert!(xdg.session_backup_path().exists());

        Ok(())
    }
}
