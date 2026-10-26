pub mod detect;
pub mod watcher;

use crate::error::{AppError, AppResult};
use crate::fs::detect::{
    classify_file_kind, encode_text_for_save, inspect_text_bytes, EncodingKind, FileKind,
    LineEnding, BINARY_SNIFF_BYTES, DEFAULT_LARGE_FILE_LIMIT_BYTES, DEFAULT_LONG_LINE_LIMIT_CHARS,
};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

pub const DEFAULT_IGNORE_NAMES: &[&str] = &[".git", "node_modules", "target"];
const MAX_WORKSPACE_FILES: usize = 20_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DraftDto {
    pub path: String,
    pub content: String,
    pub saved_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DirEntryDto {
    pub name: String,
    pub path: String,
    pub rel_path: String,
    pub is_dir: bool,
    pub is_symlink: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileReadDto {
    pub path: String,
    pub rel_path: String,
    pub name: String,
    pub kind: FileKind,
    pub content: Option<String>,
    pub size_bytes: u64,
    pub mtime_ms: u64,
    pub encoding: EncodingKind,
    pub line_ending: LineEnding,
    pub has_mixed_line_endings: bool,
    pub is_large_file: bool,
    pub has_long_lines: bool,
    pub disable_highlight: bool,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WriteResultDto {
    pub path: String,
    pub mtime_ms: u64,
    pub changed: bool,
}

pub fn resolve_existing_in_workspace(
    workspace_root: &Path,
    input_path: &Path,
) -> AppResult<PathBuf> {
    let canonical_root = workspace_root.canonicalize()?;
    let joined = if input_path.is_absolute() {
        input_path.to_path_buf()
    } else {
        canonical_root.join(input_path)
    };

    if !joined.exists() && fs::symlink_metadata(&joined).is_err() {
        return Err(AppError::NotFound(joined.display().to_string()));
    }

    let normalized_lex = normalize_lexical(&joined);
    if !normalized_lex.starts_with(&canonical_root) {
        return Err(AppError::PathOutsideWorkspace(joined.display().to_string()));
    }

    if let Ok(canonical_target) = joined.canonicalize() {
        if !canonical_target.starts_with(&canonical_root) {
            return Err(AppError::PathOutsideWorkspace(
                canonical_target.display().to_string(),
            ));
        }
        Ok(canonical_target)
    } else {
        Ok(normalized_lex)
    }
}

pub fn resolve_target_in_workspace(workspace_root: &Path, input_path: &Path) -> AppResult<PathBuf> {
    let canonical_root = workspace_root.canonicalize()?;
    let joined = if input_path.is_absolute() {
        input_path.to_path_buf()
    } else {
        canonical_root.join(input_path)
    };

    let normalized = normalize_lexical(&joined);
    if !normalized.starts_with(&canonical_root) {
        return Err(AppError::PathOutsideWorkspace(
            normalized.display().to_string(),
        ));
    }

    if let Some(parent) = normalized.parent() {
        if parent.exists() {
            let canonical_parent = parent.canonicalize()?;
            if !canonical_parent.starts_with(&canonical_root) {
                return Err(AppError::PathOutsideWorkspace(
                    canonical_parent.display().to_string(),
                ));
            }
            if let Some(file_name) = normalized.file_name() {
                return Ok(canonical_parent.join(file_name));
            }
        }
    }

    Ok(normalized)
}

fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn to_rel_path_string(canonical_root: &Path, target: &Path) -> String {
    target
        .strip_prefix(canonical_root)
        .unwrap_or(target)
        .to_string_lossy()
        .into_owned()
}

fn metadata_mtime_ms(meta: &fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn list_dir(
    workspace_root: &Path,
    dir_path: Option<&Path>,
    ignore_names: &[&str],
) -> AppResult<Vec<DirEntryDto>> {
    let canonical_root = workspace_root.canonicalize()?;
    let target_dir = match dir_path {
        Some(p) => resolve_existing_in_workspace(&canonical_root, p)?,
        None => canonical_root.clone(),
    };

    if !target_dir.is_dir() {
        return Err(AppError::InvalidPath(format!(
            "not a directory: {}",
            target_dir.display()
        )));
    }

    let mut entries = Vec::new();
    for entry_res in fs::read_dir(&target_dir)? {
        let entry = entry_res?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if ignore_names.contains(&name.as_str()) {
            continue;
        }

        let entry_path = entry.path();
        let sym_meta = fs::symlink_metadata(&entry_path)?;
        let is_symlink = sym_meta.file_type().is_symlink();
        let is_dir = entry_path.is_dir();

        if is_symlink {
            if let Ok(resolved) = entry_path.canonicalize() {
                if !resolved.starts_with(&canonical_root) {
                    continue;
                }
            }
        }

        let rel_path = to_rel_path_string(&canonical_root, &entry_path);
        entries.push(DirEntryDto {
            name,
            path: entry_path.to_string_lossy().into_owned(),
            rel_path,
            is_dir,
            is_symlink,
        });
    }

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a
            .name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.name.cmp(&b.name)),
    });

    Ok(entries)
}

pub fn read_file(workspace_root: &Path, file_path: &Path) -> AppResult<FileReadDto> {
    let canonical_root = workspace_root.canonicalize()?;
    let target = resolve_existing_in_workspace(&canonical_root, file_path)?;

    let meta = fs::metadata(&target)?;
    if meta.is_dir() {
        return Err(AppError::InvalidPath(format!(
            "cannot read directory as file: {}",
            target.display()
        )));
    }

    let size_bytes = meta.len();
    let mtime_ms = metadata_mtime_ms(&meta);
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let rel_path = to_rel_path_string(&canonical_root, &target);

    let mut file = File::open(&target)?;
    let sniff_len = (size_bytes as usize).min(BINARY_SNIFF_BYTES);
    let mut header = vec![0u8; sniff_len];
    file.read_exact(&mut header)?;

    let kind = classify_file_kind(&target, &header);
    if kind == FileKind::Binary || kind == FileKind::Image {
        return Ok(FileReadDto {
            path: target.to_string_lossy().into_owned(),
            rel_path,
            name,
            kind,
            content: None,
            size_bytes,
            mtime_ms,
            encoding: EncodingKind::Utf8,
            line_ending: LineEnding::Lf,
            has_mixed_line_endings: false,
            is_large_file: size_bytes > DEFAULT_LARGE_FILE_LIMIT_BYTES,
            has_long_lines: false,
            disable_highlight: true,
            read_only: true,
        });
    }

    let mut all_bytes = Vec::with_capacity(size_bytes as usize);
    all_bytes.extend_from_slice(&header);
    file.read_to_end(&mut all_bytes)?;

    let inspection = inspect_text_bytes(&all_bytes, DEFAULT_LONG_LINE_LIMIT_CHARS);
    let is_large_file = size_bytes > DEFAULT_LARGE_FILE_LIMIT_BYTES;
    let disable_highlight = is_large_file || inspection.has_long_lines;
    let read_only = meta.permissions().readonly() || inspection.encoding == EncodingKind::NonUtf8;

    Ok(FileReadDto {
        path: target.to_string_lossy().into_owned(),
        rel_path,
        name,
        kind,
        content: Some(inspection.text),
        size_bytes,
        mtime_ms,
        encoding: inspection.encoding,
        line_ending: inspection.line_ending,
        has_mixed_line_endings: inspection.has_mixed_line_endings,
        is_large_file,
        has_long_lines: inspection.has_long_lines,
        disable_highlight,
        read_only,
    })
}

pub fn write_file(
    workspace_root: &Path,
    file_path: &Path,
    content: &str,
    encoding: EncodingKind,
    line_ending: LineEnding,
) -> AppResult<WriteResultDto> {
    if encoding == EncodingKind::NonUtf8 {
        return Err(AppError::Validation(
            "cannot save file with non-UTF-8 encoding".into(),
        ));
    }

    let target = resolve_target_in_workspace(workspace_root, file_path)?;
    let encoded = encode_text_for_save(content, encoding, line_ending);

    if target.exists() {
        if let Ok(existing_bytes) = fs::read(&target) {
            if existing_bytes == encoded {
                let meta = fs::metadata(&target)?;
                return Ok(WriteResultDto {
                    path: target.to_string_lossy().into_owned(),
                    mtime_ms: metadata_mtime_ms(&meta),
                    changed: false,
                });
            }
        }
    }

    let parent = target
        .parent()
        .ok_or_else(|| AppError::InvalidPath(target.display().to_string()))?;
    fs::create_dir_all(parent)?;

    let existing_perms = fs::metadata(&target).ok().map(|m| m.permissions());
    let file_name = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    let temp_path = parent.join(format!(".{}.glzr-tmp.{}", file_name, std::process::id()));

    {
        let mut tmp_file = File::create(&temp_path)?;
        tmp_file.write_all(&encoded)?;
        tmp_file.sync_all()?;
    }

    if let Some(perms) = existing_perms {
        let _ = fs::set_permissions(&temp_path, perms);
    }

    fs::rename(&temp_path, &target)?;
    let meta = fs::metadata(&target)?;

    Ok(WriteResultDto {
        path: target.to_string_lossy().into_owned(),
        mtime_ms: metadata_mtime_ms(&meta),
        changed: true,
    })
}

pub fn create_file(workspace_root: &Path, file_path: &Path) -> AppResult<String> {
    let target = resolve_target_in_workspace(workspace_root, file_path)?;
    if target.exists() {
        return Err(AppError::Validation(format!(
            "file already exists: {}",
            target.display()
        )));
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    File::create(&target)?;
    Ok(target.to_string_lossy().into_owned())
}

pub fn create_dir(workspace_root: &Path, dir_path: &Path) -> AppResult<String> {
    let target = resolve_target_in_workspace(workspace_root, dir_path)?;
    if target.exists() {
        return Err(AppError::Validation(format!(
            "directory already exists: {}",
            target.display()
        )));
    }
    fs::create_dir_all(&target)?;
    Ok(target.to_string_lossy().into_owned())
}

pub fn rename_entry(workspace_root: &Path, from_path: &Path, to_path: &Path) -> AppResult<String> {
    let canonical_root = workspace_root.canonicalize()?;
    let src = resolve_existing_in_workspace(&canonical_root, from_path)?;
    if src == canonical_root {
        return Err(AppError::Validation(
            "cannot rename workspace root directory".into(),
        ));
    }

    let dst = resolve_target_in_workspace(&canonical_root, to_path)?;
    if dst.exists() {
        return Err(AppError::Validation(format!(
            "destination already exists: {}",
            dst.display()
        )));
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::rename(&src, &dst)?;
    Ok(dst.to_string_lossy().into_owned())
}

pub fn trash_entry(workspace_root: &Path, target_path: &Path) -> AppResult<()> {
    let canonical_root = workspace_root.canonicalize()?;
    let target = resolve_existing_in_workspace(&canonical_root, target_path)?;
    if target == canonical_root {
        return Err(AppError::Validation(
            "cannot trash workspace root directory".into(),
        ));
    }

    trash::delete(&target).map_err(|e| AppError::ExternalCommand(e.to_string()))
}

pub fn list_workspace_files(
    workspace_root: &Path,
    ignore_names: &[&str],
) -> AppResult<Vec<String>> {
    let canonical_root = workspace_root.canonicalize()?;
    let mut files = Vec::new();
    collect_files_recursive(
        &canonical_root,
        &canonical_root,
        ignore_names,
        &mut files,
        MAX_WORKSPACE_FILES,
    )?;
    files.sort_by(|a, b| {
        a.to_lowercase()
            .cmp(&b.to_lowercase())
            .then_with(|| a.cmp(b))
    });
    Ok(files)
}

fn collect_files_recursive(
    canonical_root: &Path,
    current_dir: &Path,
    ignore_names: &[&str],
    out: &mut Vec<String>,
    limit: usize,
) -> AppResult<()> {
    if out.len() >= limit {
        return Ok(());
    }

    let read_dir = match fs::read_dir(current_dir) {
        Ok(rd) => rd,
        Err(_) => return Ok(()),
    };

    for entry_res in read_dir {
        if out.len() >= limit {
            break;
        }
        let entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        if ignore_names.contains(&name.as_str()) {
            continue;
        }

        let path = entry.path();
        let sym_meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if sym_meta.file_type().is_symlink() {
            continue;
        }

        if sym_meta.is_dir() {
            collect_files_recursive(canonical_root, &path, ignore_names, out, limit)?;
        } else if sym_meta.is_file() {
            out.push(to_rel_path_string(canonical_root, &path));
        }
    }

    Ok(())
}

pub fn read_image_data_url(workspace_root: &Path, file_path: &Path) -> AppResult<String> {
    let canonical_root = workspace_root.canonicalize()?;
    let target = resolve_existing_in_workspace(&canonical_root, file_path)?;
    let ext = target
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "svg" => "image/svg+xml",
        _ => {
            return Err(AppError::Validation(format!(
                "unsupported image extension: {}",
                ext
            )))
        }
    };

    let bytes = fs::read(&target)?;
    let encoded = base64_encode(&bytes);
    Ok(format!("data:{};base64,{}", mime, encoded))
}

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);

    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        out.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }

    out
}

fn draft_file_for_target(cache_dir: &Path, canonical_target: &Path) -> PathBuf {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in canonical_target.to_string_lossy().as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    cache_dir.join("drafts").join(format!("{:016x}.json", hash))
}

pub fn save_draft(
    workspace_root: &Path,
    cache_dir: &Path,
    file_path: &Path,
    content: &str,
) -> AppResult<DraftDto> {
    let target = resolve_target_in_workspace(workspace_root, file_path)?;
    let draft_path = draft_file_for_target(cache_dir, &target);
    let now_ms = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let dto = DraftDto {
        path: target.to_string_lossy().into_owned(),
        content: content.to_string(),
        saved_at_ms: now_ms,
    };

    let bytes = serde_json::to_vec(&dto)?;
    crate::state::atomic_write_bytes(&draft_path, &bytes)?;
    Ok(dto)
}

pub fn load_draft(
    workspace_root: &Path,
    cache_dir: &Path,
    file_path: &Path,
) -> AppResult<Option<DraftDto>> {
    let target = resolve_target_in_workspace(workspace_root, file_path)?;
    let draft_path = draft_file_for_target(cache_dir, &target);
    if !draft_path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&draft_path)?;
    let dto: DraftDto = serde_json::from_str(&raw)?;
    Ok(Some(dto))
}

pub fn clear_draft(workspace_root: &Path, cache_dir: &Path, file_path: &Path) -> AppResult<()> {
    let target = resolve_target_in_workspace(workspace_root, file_path)?;
    let draft_path = draft_file_for_target(cache_dir, &target);
    if draft_path.exists() {
        fs::remove_file(&draft_path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn rejects_path_traversal_outside_workspace() -> AppResult<()> {
        let tmp = TempDir::new()?;
        let ws = tmp.path().join("workspace");
        fs::create_dir_all(&ws)?;

        let outside = tmp.path().join("secret.txt");
        fs::write(&outside, "secret")?;

        let traversal = Path::new("../secret.txt");
        let err = resolve_existing_in_workspace(&ws, traversal);
        assert!(matches!(err, Err(AppError::PathOutsideWorkspace(_))));

        let write_err = write_file(&ws, traversal, "hacked", EncodingKind::Utf8, LineEnding::Lf);
        assert!(matches!(write_err, Err(AppError::PathOutsideWorkspace(_))));
        Ok(())
    }

    #[test]
    fn lists_dir_with_ignore_filter_and_folders_first() -> AppResult<()> {
        let tmp = TempDir::new()?;
        let ws = tmp.path();

        fs::create_dir_all(ws.join(".git"))?;
        fs::create_dir_all(ws.join("node_modules"))?;
        fs::create_dir_all(ws.join("target"))?;
        fs::create_dir_all(ws.join("src"))?;
        fs::write(ws.join("README.md"), "# readme\n")?;
        fs::write(ws.join("a file with spaces.txt"), "hello\n")?;

        let items = list_dir(ws, None, DEFAULT_IGNORE_NAMES)?;
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["src", "a file with spaces.txt", "README.md"]);
        assert!(items[0].is_dir);
        assert!(!items[1].is_dir);
        Ok(())
    }

    #[test]
    fn atomic_write_preserves_format_and_noop_keeps_mtime() -> AppResult<()> {
        let tmp = TempDir::new()?;
        let ws = tmp.path();
        let file_path = ws.join(" unicode_nama_file.rs");

        let res1 = write_file(
            ws,
            &file_path,
            "fn main() {\n    println!(\"hi\");\n}\n",
            EncodingKind::Utf8Bom,
            LineEnding::Crlf,
        )?;
        assert!(res1.changed);

        let read_dto = read_file(ws, &file_path)?;
        assert_eq!(read_dto.encoding, EncodingKind::Utf8Bom);
        assert_eq!(read_dto.line_ending, LineEnding::Crlf);
        assert_eq!(
            read_dto.content.as_deref(),
            Some("fn main() {\n    println!(\"hi\");\n}\n")
        );

        thread::sleep(Duration::from_millis(15));
        let res2 = write_file(
            ws,
            &file_path,
            "fn main() {\n    println!(\"hi\");\n}\n",
            EncodingKind::Utf8Bom,
            LineEnding::Crlf,
        )?;
        assert!(!res2.changed);
        assert_eq!(res2.mtime_ms, res1.mtime_ms);
        Ok(())
    }

    #[test]
    fn handles_symlinks_and_create_rename_operations() -> AppResult<()> {
        let tmp = TempDir::new()?;
        let ws = tmp.path().join("ws");
        let outside = tmp.path().join("outside");
        fs::create_dir_all(&ws)?;
        fs::create_dir_all(&outside)?;
        fs::write(outside.join("leak.txt"), "outside")?;

        let created = create_file(&ws, Path::new("nested/notes.txt"))?;
        assert!(Path::new(&created).exists());

        let renamed = rename_entry(
            &ws,
            Path::new("nested/notes.txt"),
            Path::new("nested/renamed.txt"),
        )?;
        assert!(Path::new(&renamed).exists());

        symlink(outside.join("leak.txt"), ws.join("bad_link.txt"))?;
        let entries = list_dir(&ws, None, DEFAULT_IGNORE_NAMES)?;
        assert!(!entries.iter().any(|e| e.name == "bad_link.txt"));

        let read_link_err = read_file(&ws, Path::new("bad_link.txt"));
        assert!(matches!(
            read_link_err,
            Err(AppError::PathOutsideWorkspace(_))
        ));
        Ok(())
    }

    #[test]
    fn lists_workspace_files_and_manages_draft_lifecycle() -> AppResult<()> {
        let tmp = TempDir::new()?;
        let ws = tmp.path().join("ws");
        let cache = tmp.path().join("cache");
        fs::create_dir_all(ws.join("src"))?;
        fs::create_dir_all(ws.join("node_modules/pkg"))?;
        fs::write(ws.join("src/main.rs"), "fn main() {}\n")?;
        fs::write(ws.join("node_modules/pkg/index.js"), "export {};\n")?;
        fs::write(ws.join("icon.svg"), "<svg></svg>")?;

        let all_files = list_workspace_files(&ws, DEFAULT_IGNORE_NAMES)?;
        assert_eq!(all_files, vec!["icon.svg", "src/main.rs"]);

        let data_url = read_image_data_url(&ws, Path::new("icon.svg"))?;
        assert!(data_url.starts_with("data:image/svg+xml;base64,"));

        let saved = save_draft(&ws, &cache, Path::new("src/main.rs"), "fn main() { 42 }\n")?;
        assert_eq!(saved.content, "fn main() { 42 }\n");

        let loaded = load_draft(&ws, &cache, Path::new("src/main.rs"))?;
        assert_eq!(loaded, Some(saved));

        clear_draft(&ws, &cache, Path::new("src/main.rs"))?;
        assert_eq!(load_draft(&ws, &cache, Path::new("src/main.rs"))?, None);
        Ok(())
    }

    #[test]
    fn detects_large_file_and_disables_highlight() -> AppResult<()> {
        let tmp = TempDir::new()?;
        let ws = tmp.path().join("ws");
        fs::create_dir_all(&ws)?;
        let large_path = ws.join("big.log");
        let f = File::create(&large_path)?;
        f.set_len(6 * 1024 * 1024)?;
        drop(f);

        let read_dto = read_file(&ws, Path::new("big.log"))?;
        assert!(read_dto.is_large_file);
        assert!(read_dto.disable_highlight);
        assert_eq!(read_dto.size_bytes, 6 * 1024 * 1024);
        Ok(())
    }
}
