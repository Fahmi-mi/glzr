pub mod error;
pub mod fs;
pub mod git;
pub mod pty;
pub mod state;

use crate::error::{AppError, AppResult};
use crate::fs::detect::{EncodingKind, LineEnding};
use crate::fs::{DirEntryDto, FileReadDto, WriteResultDto, DEFAULT_IGNORE_NAMES};
use crate::state::{AppState, ProjectStateDto, XdgPaths};
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{Manager, State, Window, WindowEvent};

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
            reveal_in_file_manager,
            open_external
        ]);

    if let Err(err) = builder.run(tauri::generate_context!()) {
        eprintln!("failed to run glzr: {}", err);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
