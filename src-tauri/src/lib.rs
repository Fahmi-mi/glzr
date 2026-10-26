pub mod error;
pub mod fs;
pub mod git;
pub mod pty;
pub mod search;
pub mod settings;
pub mod state;

use crate::error::{AppError, AppResult};
use crate::fs::detect::{EncodingKind, LineEnding};
use crate::fs::watcher::{FsChangedEvent, FsWatchErrorEvent, WorkspaceWatcher};
use crate::fs::{DirEntryDto, DraftDto, FileReadDto, WriteResultDto, DEFAULT_IGNORE_NAMES};
use crate::git::hunk::DiffHunk;
use crate::git::{GitBranchesDto, GitDiffDto, GitStatusDto};
use crate::pty::{PtyExitEvent, PtyOutputEvent, PtySpawnDto};
use crate::search::{
    cancel_active_search, consume_rg_stream, spawn_rg_process, ActiveSearchHandle, SearchDoneEvent,
    SearchErrorEvent, SearchQueryOptions, SearchResultsEvent, DEFAULT_MAX_SEARCH_RESULTS,
    DEFAULT_SNIPPET_MAX_CHARS,
};
use crate::settings::{
    load_settings_from_path, save_settings_to_path, AppSettings, SettingsLoadDto,
};
use crate::state::{AppState, ProjectStateDto, SessionLoadDto, XdgPaths};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{atomic::AtomicBool, Arc};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State, Window, WindowEvent};

#[tauri::command]
fn project_get_state(state: State<'_, AppState>) -> AppResult<ProjectStateDto> {
    state.get_project_state()
}

#[tauri::command]
fn project_open_folder(
    path: String,
    window: Window,
    state: State<'_, AppState>,
) -> AppResult<ProjectStateDto> {
    let dto = state.open_folder(Path::new(&path))?;
    update_window_title(&window, dto.folder_name.as_deref());
    Ok(dto)
}

#[tauri::command]
fn project_remove_recent(path: String, state: State<'_, AppState>) -> AppResult<ProjectStateDto> {
    state.remove_recent_folder(&path)
}

#[tauri::command]
fn fs_list_dir(
    dir_path: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<Vec<DirEntryDto>> {
    let root = state.active_folder_path()?;
    let target = dir_path.as_deref().map(Path::new);
    fs::list_dir(&root, target, DEFAULT_IGNORE_NAMES)
}

#[tauri::command]
fn fs_read_file(path: String, state: State<'_, AppState>) -> AppResult<FileReadDto> {
    let root = state.active_folder_path()?;
    fs::read_file(&root, Path::new(&path))
}

#[tauri::command]
fn fs_write_file(
    path: String,
    content: String,
    encoding: EncodingKind,
    line_ending: LineEnding,
    state: State<'_, AppState>,
) -> AppResult<WriteResultDto> {
    let root = state.active_folder_path()?;
    fs::write_file(&root, Path::new(&path), &content, encoding, line_ending)
}

#[tauri::command]
fn fs_create_file(path: String, state: State<'_, AppState>) -> AppResult<String> {
    let root = state.active_folder_path()?;
    fs::create_file(&root, Path::new(&path))
}

#[tauri::command]
fn fs_create_dir(path: String, state: State<'_, AppState>) -> AppResult<String> {
    let root = state.active_folder_path()?;
    fs::create_dir(&root, Path::new(&path))
}

#[tauri::command]
fn fs_rename(from_path: String, to_path: String, state: State<'_, AppState>) -> AppResult<String> {
    let root = state.active_folder_path()?;
    fs::rename_entry(&root, Path::new(&from_path), Path::new(&to_path))
}

#[tauri::command]
fn fs_trash(path: String, state: State<'_, AppState>) -> AppResult<()> {
    let root = state.active_folder_path()?;
    fs::trash_entry(&root, Path::new(&path))
}

#[tauri::command]
fn fs_list_workspace_files(state: State<'_, AppState>) -> AppResult<Vec<String>> {
    let root = state.active_folder_path()?;
    fs::list_workspace_files(&root, DEFAULT_IGNORE_NAMES)
}

#[tauri::command]
fn fs_read_image_data_url(path: String, state: State<'_, AppState>) -> AppResult<String> {
    let root = state.active_folder_path()?;
    fs::read_image_data_url(&root, Path::new(&path))
}

#[tauri::command]
fn fs_save_draft(path: String, content: String, state: State<'_, AppState>) -> AppResult<DraftDto> {
    let root = state.active_folder_path()?;
    fs::save_draft(&root, &state.xdg.cache_dir, Path::new(&path), &content)
}

#[tauri::command]
fn fs_load_draft(path: String, state: State<'_, AppState>) -> AppResult<Option<DraftDto>> {
    let root = state.active_folder_path()?;
    fs::load_draft(&root, &state.xdg.cache_dir, Path::new(&path))
}

#[tauri::command]
fn fs_clear_draft(path: String, state: State<'_, AppState>) -> AppResult<()> {
    let root = state.active_folder_path()?;
    fs::clear_draft(&root, &state.xdg.cache_dir, Path::new(&path))
}

#[tauri::command]
fn fs_watch(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    let root = state.active_folder_path()?;
    state.stop_watcher();

    let app_for_change = app.clone();
    let app_for_err = app;

    let watcher = WorkspaceWatcher::start(
        &root,
        move |paths| {
            let _ = app_for_change.emit("fs:changed", FsChangedEvent { paths });
        },
        move |message| {
            let _ = app_for_err.emit("fs:watch-error", FsWatchErrorEvent { message });
        },
    )?;

    let mut guard = state
        .fs_watcher
        .lock()
        .map_err(|_| AppError::LockPoisoned)?;
    *guard = Some(watcher);
    Ok(())
}

#[tauri::command]
fn fs_unwatch(state: State<'_, AppState>) -> AppResult<()> {
    state.stop_watcher();
    Ok(())
}

#[tauri::command]
fn search_start(
    query_id: String,
    pattern: String,
    case_sensitive: bool,
    regex: bool,
    glob: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let root = state.active_folder_path()?;
    state.cancel_search();

    let opts = SearchQueryOptions {
        query_id: query_id.clone(),
        pattern,
        case_sensitive,
        regex,
        glob,
    };

    let child = spawn_rg_process(&root, &opts)?;
    let cancelled = Arc::new(AtomicBool::new(false));
    let pid = Some(child.id());

    {
        let mut guard = state
            .active_search
            .lock()
            .map_err(|_| AppError::LockPoisoned)?;
        *guard = Some(ActiveSearchHandle {
            query_id: query_id.clone(),
            cancelled: Arc::clone(&cancelled),
            pid,
        });
    }

    thread::spawn(move || {
        let qid_batch = query_id.clone();
        let qid_done = query_id.clone();
        let qid_err = query_id;
        let app_batch = app.clone();
        let app_done = app.clone();
        let app_err = app;

        consume_rg_stream(
            child,
            cancelled,
            DEFAULT_MAX_SEARCH_RESULTS,
            DEFAULT_SNIPPET_MAX_CHARS,
            move |items| {
                let _ = app_batch.emit(
                    "search:results",
                    SearchResultsEvent {
                        query_id: qid_batch.clone(),
                        items,
                    },
                );
            },
            move |truncated| {
                let _ = app_done.emit(
                    "search:done",
                    SearchDoneEvent {
                        query_id: qid_done,
                        truncated,
                    },
                );
            },
            move |message| {
                let _ = app_err.emit(
                    "search:error",
                    SearchErrorEvent {
                        query_id: qid_err,
                        message,
                    },
                );
            },
        );
    });

    Ok(())
}

#[tauri::command]
fn search_cancel(state: State<'_, AppState>) -> AppResult<()> {
    cancel_active_search(&state.active_search);
    Ok(())
}

#[tauri::command]
fn git_status(state: State<'_, AppState>) -> AppResult<GitStatusDto> {
    let root = state.active_folder_path()?;
    git::status(&root)
}

#[tauri::command]
fn git_diff(path: String, staged: bool, state: State<'_, AppState>) -> AppResult<GitDiffDto> {
    let root = state.active_folder_path()?;
    git::diff(&root, &path, staged)
}

#[tauri::command]
fn git_hunks(path: String, staged: bool, state: State<'_, AppState>) -> AppResult<Vec<DiffHunk>> {
    let root = state.active_folder_path()?;
    git::hunks(&root, &path, staged)
}

#[tauri::command]
fn git_stage(path: String, state: State<'_, AppState>) -> AppResult<()> {
    let root = state.active_folder_path()?;
    git::stage_file(&root, &path)
}

#[tauri::command]
fn git_unstage(path: String, state: State<'_, AppState>) -> AppResult<()> {
    let root = state.active_folder_path()?;
    git::unstage_file(&root, &path)
}

#[tauri::command]
fn git_discard(path: String, state: State<'_, AppState>) -> AppResult<()> {
    let root = state.active_folder_path()?;
    git::discard_file(&root, &state.xdg.cache_dir, &path)
}

#[tauri::command]
fn git_stage_hunk(path: String, hunk_index: usize, state: State<'_, AppState>) -> AppResult<()> {
    let root = state.active_folder_path()?;
    git::stage_hunk(&root, &path, hunk_index)
}

#[tauri::command]
fn git_unstage_hunk(path: String, hunk_index: usize, state: State<'_, AppState>) -> AppResult<()> {
    let root = state.active_folder_path()?;
    git::unstage_hunk(&root, &path, hunk_index)
}

#[tauri::command]
fn git_discard_hunk(path: String, hunk_index: usize, state: State<'_, AppState>) -> AppResult<()> {
    let root = state.active_folder_path()?;
    git::discard_hunk(&root, &state.xdg.cache_dir, &path, hunk_index)
}

#[tauri::command]
fn git_commit(message: String, state: State<'_, AppState>) -> AppResult<()> {
    let root = state.active_folder_path()?;
    git::commit(&root, &message)
}

#[tauri::command]
fn git_branches(state: State<'_, AppState>) -> AppResult<GitBranchesDto> {
    let root = state.active_folder_path()?;
    git::branches(&root)
}

#[tauri::command]
fn git_checkout(branch: String, state: State<'_, AppState>) -> AppResult<()> {
    let root = state.active_folder_path()?;
    git::checkout(&root, &branch)
}

#[tauri::command]
fn git_pull(state: State<'_, AppState>) -> AppResult<()> {
    let root = state.active_folder_path()?;
    git::pull(&root)
}

#[tauri::command]
fn git_push(state: State<'_, AppState>) -> AppResult<()> {
    let root = state.active_folder_path()?;
    git::push(&root)
}

#[tauri::command]
fn pty_spawn(
    session_id: Option<String>,
    cols: Option<u16>,
    rows: Option<u16>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<PtySpawnDto> {
    let root = state.active_folder_path()?;
    let (prog, owned_args) = pty::resolve_default_shell(None);
    let arg_refs: Vec<&str> = owned_args.iter().map(String::as_str).collect();

    let (dto, arc_session, rx) = pty::spawn_session_entry(
        &state.pty_sessions,
        session_id,
        &prog,
        &arg_refs,
        &root,
        cols.unwrap_or(80),
        rows.unwrap_or(24),
    )?;

    let sid = dto.session_id.clone();
    thread::spawn(move || {
        while let Ok(chunk) = rx.recv() {
            let _ = app.emit(
                "pty:output",
                PtyOutputEvent {
                    session_id: sid.clone(),
                    data: chunk,
                },
            );
        }

        let exit_code = arc_session.wait_exit_code();
        if let Some(st) = app.try_state::<AppState>() {
            if let Ok(mut guard) = st.pty_sessions.lock() {
                if guard
                    .get(&sid)
                    .is_some_and(|existing| std::sync::Arc::ptr_eq(existing, &arc_session))
                {
                    guard.remove(&sid);
                }
            }
        }

        let _ = app.emit(
            "pty:exit",
            PtyExitEvent {
                session_id: sid,
                code: exit_code,
            },
        );
    });

    Ok(dto)
}

#[tauri::command]
fn pty_write(session_id: String, data: String, state: State<'_, AppState>) -> AppResult<()> {
    pty::write_session_entry(&state.pty_sessions, &session_id, data.as_bytes())
}

#[tauri::command]
fn pty_resize(
    session_id: String,
    cols: u16,
    rows: u16,
    state: State<'_, AppState>,
) -> AppResult<()> {
    pty::resize_session_entry(&state.pty_sessions, &session_id, cols, rows)
}

#[tauri::command]
fn pty_kill(session_id: String, state: State<'_, AppState>) -> AppResult<()> {
    pty::kill_session_entry(&state.pty_sessions, &session_id)
}

#[tauri::command]
fn session_load(state: State<'_, AppState>) -> AppResult<SessionLoadDto> {
    state.load_session()
}

#[tauri::command]
fn session_save(raw_json: String, state: State<'_, AppState>) -> AppResult<()> {
    state.save_session(&raw_json)
}

#[tauri::command]
fn settings_load(state: State<'_, AppState>) -> AppResult<SettingsLoadDto> {
    load_settings_from_path(&state.xdg.settings_file_path())
}

#[tauri::command]
fn settings_save(settings: AppSettings, state: State<'_, AppState>) -> AppResult<()> {
    save_settings_to_path(&state.xdg.settings_file_path(), &settings)
}

#[tauri::command]
fn reveal_in_file_manager(path: String) -> AppResult<()> {
    let target = Path::new(&path);
    if !target.exists() {
        return Err(AppError::NotFound(path));
    }

    let dir_to_open = if target.is_dir() {
        target
    } else {
        target
            .parent()
            .ok_or_else(|| AppError::InvalidPath(path.clone()))?
    };

    let status = Command::new("xdg-open").arg(dir_to_open).status()?;
    if !status.success() {
        return Err(AppError::ExternalCommand(
            "xdg-open failed to open file manager".into(),
        ));
    }
    Ok(())
}

#[tauri::command]
fn open_external(target: String) -> AppResult<()> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation("target cannot be empty".into()));
    }

    let is_http = trimmed.starts_with("http://") || trimmed.starts_with("https://");
    let is_existing_path = Path::new(trimmed).exists();
    if !is_http && !is_existing_path {
        return Err(AppError::Validation(format!(
            "unsupported external target: {}",
            trimmed
        )));
    }

    let status = Command::new("xdg-open").arg(trimmed).status()?;
    if !status.success() {
        return Err(AppError::ExternalCommand(
            "xdg-open failed to open external target".into(),
        ));
    }
    Ok(())
}

fn parse_localhost_target(raw_url: &str) -> AppResult<(String, u16)> {
    let trimmed = raw_url.trim();
    let (scheme, rest) = if let Some(after) = trimmed.strip_prefix("http://") {
        ("http", after)
    } else if let Some(after) = trimmed.strip_prefix("https://") {
        ("https", after)
    } else {
        return Err(AppError::Validation(
            "URL preview harus menggunakan http:// atau https://".into(),
        ));
    };

    let authority = rest.split(['/', '?', '#']).next().unwrap_or("").trim();

    if authority.is_empty() {
        return Err(AppError::Validation("Host preview kosong".into()));
    }

    let default_port = if scheme == "https" { 443 } else { 80 };
    let (host, port) = if let Some((h, p_str)) = authority.split_once(':') {
        let parsed_port = p_str
            .parse::<u16>()
            .map_err(|_| AppError::Validation(format!("Port preview tidak valid: {}", p_str)))?;
        if parsed_port == 0 {
            return Err(AppError::Validation("Port preview tidak boleh 0".into()));
        }
        (h.to_ascii_lowercase(), parsed_port)
    } else {
        (authority.to_ascii_lowercase(), default_port)
    };

    if host != "localhost" && host != "127.0.0.1" {
        return Err(AppError::Validation(format!(
            "Host preview tidak diizinkan: {}",
            host
        )));
    }

    Ok((host, port))
}

fn check_localhost_reachable(raw_url: &str) -> AppResult<bool> {
    let (host, port) = parse_localhost_target(raw_url)?;
    let timeout = Duration::from_millis(350);

    let v4_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port);
    if TcpStream::connect_timeout(&v4_addr, timeout).is_ok() {
        return Ok(true);
    }

    if host == "localhost" {
        let v6_addr = SocketAddr::new(Ipv6Addr::LOCALHOST.into(), port);
        if TcpStream::connect_timeout(&v6_addr, timeout).is_ok() {
            return Ok(true);
        }
    }

    Ok(false)
}

#[tauri::command]
fn preview_check_url(url: String) -> AppResult<bool> {
    check_localhost_reachable(&url)
}

fn update_window_title(window: &Window, folder_name: Option<&str>) {
    let title = match folder_name {
        Some(name) if !name.is_empty() => format!("{} - glzr", name),
        _ => "glzr".to_string(),
    };
    let _ = window.set_title(&title);
}

fn resolve_cli_folder_arg<I>(args: I) -> Option<PathBuf>
where
    I: IntoIterator<Item = String>,
{
    for arg in args.into_iter().skip(1) {
        if arg.starts_with('-') {
            continue;
        }
        let candidate = PathBuf::from(&arg);
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(target_os = "linux")]
pub fn configure_linux_webkit_environment() {
    let has_nvidia = Path::new("/dev/nvidia0").exists()
        || Path::new("/proc/driver/nvidia").exists()
        || Path::new("/sys/module/nvidia").exists();
    if has_nvidia {
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
        if std::env::var_os("WEBKIT_DISABLE_COMPOSITING_MODE").is_none() {
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    configure_linux_webkit_environment();

    let xdg = XdgPaths::resolve()
        .unwrap_or_else(|_| XdgPaths::from_base_env(Path::new("/tmp"), None, None, None, None));
    let initial_folder = resolve_cli_folder_arg(std::env::args());
    let app_state = AppState::new(xdg, initial_folder);

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
            if let Some(folder) = resolve_cli_folder_arg(argv) {
                if let Some(state) = app.try_state::<AppState>() {
                    let _ = state.open_folder(&folder);
                }
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_cli::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .max_file_size(2 * 1024 * 1024)
                .build(),
        )
        .manage(app_state)
        .setup(|app| {
            #[cfg(debug_assertions)]
            if let Some(window) = app.get_webview_window("main") {
                window.open_devtools();
            }

            if let (Some(window), Some(state)) =
                (app.get_webview_window("main"), app.try_state::<AppState>())
            {
                if let Ok(dto) = state.get_project_state() {
                    if let Some(name) = dto.folder_name.as_deref() {
                        let _ = window.set_title(&format!("{} - glzr", name));
                    }
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::Destroyed = event {
                if let Some(state) = window.try_state::<AppState>() {
                    state.stop_watcher();
                    state.cancel_search();
                    state.kill_all_pty_sessions();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            project_get_state,
            project_open_folder,
            project_remove_recent,
            fs_list_dir,
            fs_read_file,
            fs_write_file,
            fs_create_file,
            fs_create_dir,
            fs_rename,
            fs_trash,
            fs_list_workspace_files,
            fs_read_image_data_url,
            fs_save_draft,
            fs_load_draft,
            fs_clear_draft,
            fs_watch,
            fs_unwatch,
            search_start,
            search_cancel,
            git_status,
            git_diff,
            git_hunks,
            git_stage,
            git_unstage,
            git_discard,
            git_stage_hunk,
            git_unstage_hunk,
            git_discard_hunk,
            git_commit,
            git_branches,
            git_checkout,
            git_pull,
            git_push,
            pty_spawn,
            pty_write,
            pty_resize,
            pty_kill,
            session_load,
            session_save,
            settings_load,
            settings_save,
            reveal_in_file_manager,
            open_external,
            preview_check_url
        ]);

    if let Err(err) = builder.run(tauri::generate_context!()) {
        eprintln!("failed to run glzr: {}", err);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use tempfile::TempDir;

    #[test]
    fn cli_folder_arg_ignores_flags_and_picks_existing_dir() -> AppResult<()> {
        let tmp = TempDir::new()?;
        let dir_str = tmp.path().to_string_lossy().into_owned();
        let args = vec!["glzr".to_string(), "--verbose".to_string(), dir_str.clone()];

        let resolved = resolve_cli_folder_arg(args);
        assert_eq!(resolved, Some(PathBuf::from(dir_str)));
        Ok(())
    }

    #[test]
    fn validates_and_checks_localhost_reachability() -> AppResult<()> {
        assert_eq!(
            parse_localhost_target("http://localhost:5173/app?x=1")?,
            ("localhost".to_string(), 5173)
        );
        assert_eq!(
            parse_localhost_target("http://127.0.0.1:8080")?,
            ("127.0.0.1".to_string(), 8080)
        );
        assert!(parse_localhost_target("https://example.com:8080").is_err());
        assert!(parse_localhost_target("file:///etc/passwd").is_err());

        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        let url = format!("http://127.0.0.1:{}/", port);
        assert!(check_localhost_reachable(&url)?);
        drop(listener);

        // An intentionally unallocated high port is unreachable
        assert!(!check_localhost_reachable("http://127.0.0.1:59998")?);
        Ok(())
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn auto_detects_or_preserves_linux_webkit_environment() {
        configure_linux_webkit_environment();
        if Path::new("/dev/nvidia0").exists()
            || Path::new("/proc/driver/nvidia").exists()
            || Path::new("/sys/module/nvidia").exists()
        {
            assert_eq!(
                std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").as_deref(),
                Ok("1")
            );
            assert_eq!(
                std::env::var("WEBKIT_DISABLE_COMPOSITING_MODE").as_deref(),
                Ok("1")
            );

            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "0");
            configure_linux_webkit_environment();
            assert_eq!(
                std::env::var("WEBKIT_DISABLE_COMPOSITING_MODE").as_deref(),
                Ok("0")
            );
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
    }
}
