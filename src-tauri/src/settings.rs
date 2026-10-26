use crate::error::AppResult;
use crate::state::atomic_write_bytes;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceSettings {
    pub theme: String,
    pub font_family: String,
    pub font_size: u16,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            font_family: "system-mono".to_string(),
            font_size: 14,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EditorSettings {
    pub tab_size: u8,
    pub insert_spaces: bool,
    pub word_wrap: bool,
    pub line_numbers: bool,
    pub large_file_limit_mb: u32,
    pub long_line_limit: u32,
    pub draft_autosave_seconds: u32,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            tab_size: 4,
            insert_spaces: true,
            word_wrap: false,
            line_numbers: true,
            large_file_limit_mb: 5,
            long_line_limit: 10_000,
            draft_autosave_seconds: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSettings {
    pub shell: String,
    pub shell_args: Vec<String>,
    pub extra_env: HashMap<String, String>,
    pub scrollback: u32,
    pub cursor_style: String,
    pub confirm_paste_over_kb: u32,
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            shell: String::new(),
            shell_args: vec!["-l".to_string(), "-i".to_string()],
            extra_env: HashMap::new(),
            scrollback: 5000,
            cursor_style: "block".to_string(),
            confirm_paste_over_kb: 64,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FilesSettings {
    pub ignore: Vec<String>,
}

impl Default for FilesSettings {
    fn default() -> Self {
        Self {
            ignore: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitSettings {
    pub command_timeout_seconds: u32,
    pub diff_size_limit_mb: u32,
}

impl Default for GitSettings {
    fn default() -> Self {
        Self {
            command_timeout_seconds: 30,
            diff_size_limit_mb: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchSettings {
    pub max_results: usize,
    pub snippet_max_chars: usize,
    pub debounce_ms: u32,
}

impl Default for SearchSettings {
    fn default() -> Self {
        Self {
            max_results: 2000,
            snippet_max_chars: 300,
            debounce_ms: 200,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewSettings {
    pub allowed_hosts: Vec<String>,
    pub auto_detect_port: bool,
}

impl Default for PreviewSettings {
    fn default() -> Self {
        Self {
            allowed_hosts: vec!["localhost".to_string(), "127.0.0.1".to_string()],
            auto_detect_port: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionSettings {
    pub restore: bool,
    pub recent_folders_limit: usize,
}

impl Default for SessionSettings {
    fn default() -> Self {
        Self {
            restore: true,
            recent_folders_limit: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LoggingSettings {
    pub level: String,
}

impl Default for LoggingSettings {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KeybindingsSettings {
    pub command_palette: String,
    pub quick_open: String,
    pub sidebar_files: String,
    pub sidebar_git: String,
    pub sidebar_search: String,
    pub new_terminal: String,
    pub close_tab: String,
    pub split_right: String,
    pub split_down: String,
    pub close_pane: String,
    pub maximize_pane: String,
    pub terminal_find: String,
}

impl Default for KeybindingsSettings {
    fn default() -> Self {
        Self {
            command_palette: "Alt+P".to_string(),
            quick_open: "Alt+O".to_string(),
            sidebar_files: "Alt+E".to_string(),
            sidebar_git: "Alt+V".to_string(),
            sidebar_search: "Alt+G".to_string(),
            new_terminal: "Alt+T".to_string(),
            close_tab: "Alt+W".to_string(),
            split_right: "Alt+R".to_string(),
            split_down: "Alt+J".to_string(),
            close_pane: "Alt+Q".to_string(),
            maximize_pane: "Alt+M".to_string(),
            terminal_find: "Alt+S".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub appearance: AppearanceSettings,
    pub editor: EditorSettings,
    pub terminal: TerminalSettings,
    pub files: FilesSettings,
    pub git: GitSettings,
    pub search: SearchSettings,
    pub preview: PreviewSettings,
    pub session: SessionSettings,
    pub logging: LoggingSettings,
    pub keybindings: KeybindingsSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsLoadDto {
    pub settings: AppSettings,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TomlFileRepr {
    appearance: AppearanceToml,
    editor: EditorToml,
    terminal: TerminalToml,
    files: FilesToml,
    git: GitToml,
    search: SearchToml,
    preview: PreviewToml,
    session: SessionToml,
    logging: LoggingToml,
    keybindings: KeybindingsToml,
}

#[derive(Debug, Serialize, Deserialize)]
struct AppearanceToml {
    theme: String,
    font_family: String,
    font_size: u16,
}

#[derive(Debug, Serialize, Deserialize)]
struct EditorToml {
    tab_size: u8,
    insert_spaces: bool,
    word_wrap: bool,
    line_numbers: bool,
    large_file_limit_mb: u32,
    long_line_limit: u32,
    draft_autosave_seconds: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct TerminalToml {
    shell: String,
    shell_args: Vec<String>,
    extra_env: HashMap<String, String>,
    scrollback: u32,
    cursor_style: String,
    confirm_paste_over_kb: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct FilesToml {
    ignore: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GitToml {
    command_timeout_seconds: u32,
    diff_size_limit_mb: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchToml {
    max_results: usize,
    snippet_max_chars: usize,
    debounce_ms: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct PreviewToml {
    allowed_hosts: Vec<String>,
    auto_detect_port: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct SessionToml {
    restore: bool,
    recent_folders_limit: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoggingToml {
    level: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct KeybindingsToml {
    command_palette: String,
    quick_open: String,
    sidebar_files: String,
    sidebar_git: String,
    sidebar_search: String,
    new_terminal: String,
    close_tab: String,
    split_right: String,
    split_down: String,
    close_pane: String,
    maximize_pane: String,
    terminal_find: String,
}

pub fn load_settings_from_path(path: &Path) -> AppResult<SettingsLoadDto> {
    if !path.exists() {
        return Ok(SettingsLoadDto {
            settings: AppSettings::default(),
            warnings: Vec::new(),
        });
    }

    let raw = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => {
            return Ok(SettingsLoadDto {
                settings: AppSettings::default(),
                warnings: vec![
                    "Gagal membaca settings.toml; menggunakan pengaturan bawaan.".to_string(),
                ],
            });
        }
    };

    Ok(parse_and_validate_settings_toml(&raw))
}

pub fn save_settings_to_path(path: &Path, settings: &AppSettings) -> AppResult<()> {
    let repr = TomlFileRepr {
        appearance: AppearanceToml {
            theme: settings.appearance.theme.clone(),
            font_family: settings.appearance.font_family.clone(),
            font_size: settings.appearance.font_size,
        },
        editor: EditorToml {
            tab_size: settings.editor.tab_size,
            insert_spaces: settings.editor.insert_spaces,
            word_wrap: settings.editor.word_wrap,
            line_numbers: settings.editor.line_numbers,
            large_file_limit_mb: settings.editor.large_file_limit_mb,
            long_line_limit: settings.editor.long_line_limit,
            draft_autosave_seconds: settings.editor.draft_autosave_seconds,
        },
        terminal: TerminalToml {
            shell: settings.terminal.shell.clone(),
            shell_args: settings.terminal.shell_args.clone(),
            extra_env: settings.terminal.extra_env.clone(),
            scrollback: settings.terminal.scrollback,
            cursor_style: settings.terminal.cursor_style.clone(),
            confirm_paste_over_kb: settings.terminal.confirm_paste_over_kb,
        },
        files: FilesToml {
            ignore: settings.files.ignore.clone(),
        },
        git: GitToml {
            command_timeout_seconds: settings.git.command_timeout_seconds,
            diff_size_limit_mb: settings.git.diff_size_limit_mb,
        },
        search: SearchToml {
            max_results: settings.search.max_results,
            snippet_max_chars: settings.search.snippet_max_chars,
            debounce_ms: settings.search.debounce_ms,
        },
        preview: PreviewToml {
            allowed_hosts: settings.preview.allowed_hosts.clone(),
            auto_detect_port: settings.preview.auto_detect_port,
        },
        session: SessionToml {
            restore: settings.session.restore,
            recent_folders_limit: settings.session.recent_folders_limit,
        },
        logging: LoggingToml {
            level: settings.logging.level.clone(),
        },
        keybindings: KeybindingsToml {
            command_palette: settings.keybindings.command_palette.clone(),
            quick_open: settings.keybindings.quick_open.clone(),
            sidebar_files: settings.keybindings.sidebar_files.clone(),
            sidebar_git: settings.keybindings.sidebar_git.clone(),
            sidebar_search: settings.keybindings.sidebar_search.clone(),
            new_terminal: settings.keybindings.new_terminal.clone(),
            close_tab: settings.keybindings.close_tab.clone(),
            split_right: settings.keybindings.split_right.clone(),
            split_down: settings.keybindings.split_down.clone(),
            close_pane: settings.keybindings.close_pane.clone(),
            maximize_pane: settings.keybindings.maximize_pane.clone(),
            terminal_find: settings.keybindings.terminal_find.clone(),
        },
    };

    let serialized = toml::to_string_pretty(&repr)
        .map_err(|e| crate::error::AppError::Validation(e.to_string()))?;
    atomic_write_bytes(path, serialized.as_bytes())
}

pub fn parse_and_validate_settings_toml(raw: &str) -> SettingsLoadDto {
    let mut settings = AppSettings::default();
    let mut warnings = Vec::new();

    let root: toml::Value = match raw.parse::<toml::Value>() {
        Ok(v) => v,
        Err(_) => {
            warnings.push(
                "File settings.toml rusak dan tidak dapat diurai; menggunakan pengaturan bawaan."
                    .to_string(),
            );
            return SettingsLoadDto { settings, warnings };
        }
    };

    let Some(table) = root.as_table() else {
        warnings.push("Format settings.toml tidak valid.".to_string());
        return SettingsLoadDto { settings, warnings };
    };

    if let Some(app_tbl) = table.get("appearance").and_then(|v| v.as_table()) {
        if let Some(theme_val) = app_tbl.get("theme").and_then(|v| v.as_str()) {
            if theme_val == "dark" || theme_val == "light" {
                settings.appearance.theme = theme_val.to_string();
            } else {
                warnings.push(format!(
                    "appearance.theme '{}' tidak valid; memakai default 'dark'.",
                    theme_val
                ));
            }
        }
        if let Some(font_val) = app_tbl.get("font_family").and_then(|v| v.as_str()) {
            if font_val == "system-mono" || font_val == "jetbrains-mono-nerd" {
                settings.appearance.font_family = font_val.to_string();
            } else {
                warnings.push(format!(
                    "appearance.font_family '{}' tidak valid; memakai default 'system-mono'.",
                    font_val
                ));
            }
        }
        if let Some(size_val) = app_tbl.get("font_size").and_then(|v| v.as_integer()) {
            if (10..=28).contains(&size_val) {
                settings.appearance.font_size = size_val as u16;
            } else {
                warnings.push(format!(
                    "appearance.font_size '{}' di luar rentang 10..28; memakai default 14.",
                    size_val
                ));
            }
        }
    }

    if let Some(ed_tbl) = table.get("editor").and_then(|v| v.as_table()) {
        if let Some(ts) = ed_tbl.get("tab_size").and_then(|v| v.as_integer()) {
            if (1..=16).contains(&ts) {
                settings.editor.tab_size = ts as u8;
            } else {
                warnings.push("editor.tab_size di luar rentang 1..16; memakai default 4.".into());
            }
        }
        if let Some(ins) = ed_tbl.get("insert_spaces").and_then(|v| v.as_bool()) {
            settings.editor.insert_spaces = ins;
        }
        if let Some(ww) = ed_tbl.get("word_wrap").and_then(|v| v.as_bool()) {
            settings.editor.word_wrap = ww;
        }
        if let Some(ln) = ed_tbl.get("line_numbers").and_then(|v| v.as_bool()) {
            settings.editor.line_numbers = ln;
        }
    }

    if let Some(kb_tbl) = table.get("keybindings").and_then(|v| v.as_table()) {
        macro_rules! read_kb {
            ($field:ident, $key:literal) => {
                if let Some(val) = kb_tbl.get($key).and_then(|v| v.as_str()) {
                    let trimmed = val.trim();
                    if !trimmed.is_empty() {
                        settings.keybindings.$field = trimmed.to_string();
                    }
                }
            };
        }
        read_kb!(command_palette, "command_palette");
        read_kb!(quick_open, "quick_open");
        read_kb!(sidebar_files, "sidebar_files");
        read_kb!(sidebar_git, "sidebar_git");
        read_kb!(sidebar_search, "sidebar_search");
        read_kb!(new_terminal, "new_terminal");
        read_kb!(close_tab, "close_tab");
        read_kb!(split_right, "split_right");
        read_kb!(split_down, "split_down");
        read_kb!(close_pane, "close_pane");
        read_kb!(maximize_pane, "maximize_pane");
        read_kb!(terminal_find, "terminal_find");
    }

    let kb_entries = [
        ("command_palette", &settings.keybindings.command_palette),
        ("quick_open", &settings.keybindings.quick_open),
        ("sidebar_files", &settings.keybindings.sidebar_files),
        ("sidebar_git", &settings.keybindings.sidebar_git),
        ("sidebar_search", &settings.keybindings.sidebar_search),
        ("new_terminal", &settings.keybindings.new_terminal),
        ("close_tab", &settings.keybindings.close_tab),
        ("split_right", &settings.keybindings.split_right),
        ("split_down", &settings.keybindings.split_down),
        ("close_pane", &settings.keybindings.close_pane),
        ("maximize_pane", &settings.keybindings.maximize_pane),
        ("terminal_find", &settings.keybindings.terminal_find),
    ];

    let mut seen = HashSet::new();
    for (name, combo) in kb_entries {
        let normalized = combo.to_ascii_lowercase();
        if !seen.insert(normalized.clone()) {
            warnings.push(format!(
                "Keybinding duplikat terdeteksi pada '{}': {}",
                name, combo
            ));
        }
        if normalized.starts_with("ctrl+") && !normalized.starts_with("ctrl+shift+") {
            warnings.push(format!(
                "Keybinding '{}' ({}) memakai Ctrl+huruf yang tidak aktif saat fokus di terminal.",
                name, combo
            ));
        }
    }

    SettingsLoadDto { settings, warnings }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn parses_valid_and_out_of_range_settings_with_warnings() {
        let raw = r#"
[appearance]
theme = "light"
font_family = "jetbrains-mono-nerd"
font_size = 99

[keybindings]
command_palette = "Alt+P"
quick_open = "Alt+P"
"#;

        let dto = parse_and_validate_settings_toml(raw);
        assert_eq!(dto.settings.appearance.theme, "light");
        assert_eq!(dto.settings.appearance.font_family, "jetbrains-mono-nerd");
        assert_eq!(dto.settings.appearance.font_size, 14);
        assert!(dto
            .warnings
            .iter()
            .any(|w| w.contains("appearance.font_size")));
        assert!(dto
            .warnings
            .iter()
            .any(|w| w.contains("Keybinding duplikat")));
    }

    #[test]
    fn corrupt_settings_toml_is_not_overwritten_and_returns_defaults() -> AppResult<()> {
        let tmp = TempDir::new()?;
        let path = tmp.path().join("settings.toml");
        fs::write(&path, "[[[broken toml")?;

        let dto = load_settings_from_path(&path)?;
        assert_eq!(dto.settings.appearance.theme, "dark");
        assert!(!dto.warnings.is_empty());
        assert_eq!(fs::read_to_string(&path)?, "[[[broken toml");
        Ok(())
    }
}
