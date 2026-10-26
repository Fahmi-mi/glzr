pub mod hunk;
pub mod parse;

use crate::error::{AppError, AppResult};
use crate::fs::detect::is_binary_header;
use crate::git::hunk::{
    apply_hunk_patch, build_hunk_patch, parse_file_diff, DiffHunk, FileDiff, HunkError,
};
use crate::git::parse::{parse_porcelain_v2, GitFileEntry};
use crate::state::atomic_write_bytes;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const DIFF_SIZE_LIMIT_BYTES: usize = 2 * 1024 * 1024;
const GIT_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatusDto {
    pub git_available: bool,
    pub is_repo: bool,
    pub branch: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub detached: bool,
    pub merging: bool,
    pub rebasing: bool,
    pub has_conflicts: bool,
    pub staged: Vec<GitFileEntry>,
    pub unstaged: Vec<GitFileEntry>,
    pub untracked: Vec<GitFileEntry>,
    pub conflicted: Vec<GitFileEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitDiffDto {
    pub path: String,
    pub staged: bool,
    pub is_binary: bool,
    pub too_large: bool,
    pub raw_diff: String,
    pub file_diff: Option<FileDiff>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBranchesDto {
    pub current: Option<String>,
    pub branches: Vec<String>,
}

fn validate_rel_path(rel_path: &str) -> AppResult<PathBuf> {
    let trimmed = rel_path.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidPath("empty relative path".into()));
    }
    let path = Path::new(trimmed);
    if path.is_absolute() {
        return Err(AppError::PathOutsideWorkspace(trimmed.to_string()));
    }
    let mut normalized = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::Normal(seg) => normalized.push(seg),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(AppError::PathOutsideWorkspace(trimmed.to_string()));
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        return Err(AppError::InvalidPath(trimmed.to_string()));
    }
    Ok(normalized)
}

fn run_git_raw(repo_dir: &Path, args: &[&str], timeout_secs: u64) -> AppResult<Output> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_dir)
        .env("LC_ALL", "C")
        .env("GIT_TERMINAL_PROMPT", "0")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            AppError::NotFound("git executable not found".into())
        } else {
            AppError::Io(err)
        }
    })?;

    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        match child.try_wait()? {
            Some(_) => return Ok(child.wait_with_output()?),
            None => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(AppError::ExternalCommand(format!(
                        "git command timed out after {} seconds",
                        timeout_secs
                    )));
                }
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

fn run_git_checked(repo_dir: &Path, args: &[&str]) -> AppResult<Output> {
    let output = run_git_raw(repo_dir, args, GIT_TIMEOUT_SECS)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(AppError::ExternalCommand(detail));
    }
    Ok(output)
}

fn resolve_git_dir(repo_dir: &Path) -> Option<PathBuf> {
    let output = run_git_raw(repo_dir, &["rev-parse", "--git-dir"], 5).ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let git_dir = PathBuf::from(raw);
    if git_dir.is_absolute() {
        Some(git_dir)
    } else {
        Some(repo_dir.join(git_dir))
    }
}

/// Returns repository status including branch, ahead/behind, and file entries.
pub fn status(repo_dir: &Path) -> AppResult<GitStatusDto> {
    let output = match run_git_raw(
        repo_dir,
        &["status", "--porcelain=v2", "-z", "--branch"],
        GIT_TIMEOUT_SECS,
    ) {
        Ok(out) => out,
        Err(AppError::NotFound(_)) => {
            return Ok(GitStatusDto {
                git_available: false,
                is_repo: false,
                branch: None,
                upstream: None,
                ahead: 0,
                behind: 0,
                detached: false,
                merging: false,
                rebasing: false,
                has_conflicts: false,
                staged: Vec::new(),
                unstaged: Vec::new(),
                untracked: Vec::new(),
                conflicted: Vec::new(),
            });
        }
        Err(err) => return Err(err),
    };

    if !output.status.success() {
        return Ok(GitStatusDto {
            git_available: true,
            is_repo: false,
            branch: None,
            upstream: None,
            ahead: 0,
            behind: 0,
            detached: false,
            merging: false,
            rebasing: false,
            has_conflicts: false,
            staged: Vec::new(),
            unstaged: Vec::new(),
            untracked: Vec::new(),
            conflicted: Vec::new(),
        });
    }

    let parsed = parse_porcelain_v2(&output.stdout);
    let git_dir = resolve_git_dir(repo_dir);
    let merging = git_dir
        .as_ref()
        .is_some_and(|dir| dir.join("MERGE_HEAD").exists());
    let rebasing = git_dir
        .as_ref()
        .is_some_and(|dir| dir.join("rebase-merge").exists() || dir.join("rebase-apply").exists());
    let has_conflicts = !parsed.conflicted.is_empty();

    Ok(GitStatusDto {
        git_available: true,
        is_repo: true,
        branch: parsed.branch,
        upstream: parsed.upstream,
        ahead: parsed.ahead,
        behind: parsed.behind,
        detached: parsed.detached,
        merging,
        rebasing,
        has_conflicts,
        staged: parsed.staged,
        unstaged: parsed.unstaged,
        untracked: parsed.untracked,
        conflicted: parsed.conflicted,
    })
}

/// Computes diff for a single file (staged, unstaged, or untracked).
pub fn diff(repo_dir: &Path, rel_path: &str, staged: bool) -> AppResult<GitDiffDto> {
    let valid_rel = validate_rel_path(rel_path)?;
    let rel_str = valid_rel.to_string_lossy().replace('\\', "/");

    let mut args = vec!["diff", "--no-color", "--no-ext-diff"];
    if staged {
        args.push("--cached");
    }
    args.push("--");
    args.push(&rel_str);

    let output = run_git_checked(repo_dir, &args)?;
    if output.stdout.len() > DIFF_SIZE_LIMIT_BYTES {
        return Ok(GitDiffDto {
            path: rel_str,
            staged,
            is_binary: false,
            too_large: true,
            raw_diff: String::new(),
            file_diff: None,
        });
    }

    let mut raw_diff = String::from_utf8_lossy(&output.stdout).into_owned();

    // Synthesize diff when inspecting an untracked file in unstaged mode
    if raw_diff.trim().is_empty() && !staged {
        let full_path = repo_dir.join(&valid_rel);
        if full_path.is_file() && !is_file_tracked(repo_dir, &rel_str) {
            let meta = fs::metadata(&full_path)?;
            if meta.len() as usize > DIFF_SIZE_LIMIT_BYTES {
                return Ok(GitDiffDto {
                    path: rel_str,
                    staged,
                    is_binary: false,
                    too_large: true,
                    raw_diff: String::new(),
                    file_diff: None,
                });
            }
            let bytes = fs::read(&full_path)?;
            if is_binary_header(&bytes) {
                return Ok(GitDiffDto {
                    path: rel_str,
                    staged,
                    is_binary: true,
                    too_large: false,
                    raw_diff: String::new(),
                    file_diff: None,
                });
            }
            let text = String::from_utf8_lossy(&bytes);
            raw_diff = build_untracked_raw_diff(&rel_str, &text);
        }
    }

    if is_binary_diff_output(&raw_diff) {
        return Ok(GitDiffDto {
            path: rel_str,
            staged,
            is_binary: true,
            too_large: false,
            raw_diff,
            file_diff: None,
        });
    }

    let file_diff = parse_file_diff(&raw_diff)?;
    Ok(GitDiffDto {
        path: rel_str,
        staged,
        is_binary: false,
        too_large: false,
        raw_diff,
        file_diff,
    })
}

/// Returns parsed hunks for a file.
pub fn hunks(repo_dir: &Path, rel_path: &str, staged: bool) -> AppResult<Vec<DiffHunk>> {
    let dto = diff(repo_dir, rel_path, staged)?;
    Ok(dto.file_diff.map(|fd| fd.hunks).unwrap_or_default())
}

fn is_file_tracked(repo_dir: &Path, rel_str: &str) -> bool {
    run_git_raw(repo_dir, &["ls-files", "--error-unmatch", "--", rel_str], 5)
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn is_binary_diff_output(raw_diff: &str) -> bool {
    for line in raw_diff.lines().take(12) {
        if (line.starts_with("Binary files ") && line.ends_with(" differ"))
            || line.starts_with("GIT binary patch")
        {
            return true;
        }
    }
    false
}

fn build_untracked_raw_diff(rel_str: &str, content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let count = lines.len();
    let mut out = format!(
        "diff --git a/{0} b/{0}\nnew file mode 100644\n--- /dev/null\n+++ b/{0}\n",
        rel_str
    );
    if count > 0 {
        out.push_str(&format!("@@ -0,0 +1,{} @@\n", count));
        for line in lines {
            out.push('+');
            out.push_str(line);
            out.push('\n');
        }
        if !content.ends_with('\n') {
            out.push_str("\\ No newline at end of file\n");
        }
    }
    out
}

/// Stages an entire file (`git add -- <path>`).
pub fn stage_file(repo_dir: &Path, rel_path: &str) -> AppResult<()> {
    let valid_rel = validate_rel_path(rel_path)?;
    let rel_str = valid_rel.to_string_lossy().replace('\\', "/");
    run_git_checked(repo_dir, &["add", "--", &rel_str])?;
    Ok(())
}

/// Unstages an entire file (`git reset HEAD -- <path>`, with fallback for initial unborn branch).
pub fn unstage_file(repo_dir: &Path, rel_path: &str) -> AppResult<()> {
    let valid_rel = validate_rel_path(rel_path)?;
    let rel_str = valid_rel.to_string_lossy().replace('\\', "/");

    let has_head = run_git_raw(repo_dir, &["rev-parse", "--verify", "HEAD"], 5)
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_head {
        run_git_checked(repo_dir, &["reset", "HEAD", "--", &rel_str])?;
    } else {
        run_git_checked(repo_dir, &["rm", "--cached", "-f", "--", &rel_str])?;
    }
    Ok(())
}

fn save_discard_backup(
    cache_dir: &Path,
    rel_str: &str,
    data: &[u8],
    suffix: &str,
) -> AppResult<()> {
    let discarded_dir = cache_dir.join("discarded");
    fs::create_dir_all(&discarded_dir)?;

    let mut hasher = DefaultHasher::new();
    rel_str.hash(&mut hasher);
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let file_name = format!("{:016x}-{}.{}", hasher.finish(), ts, suffix);
    atomic_write_bytes(&discarded_dir.join(file_name), data)
}

/// Discards unstaged changes to a file after saving a backup in the cache directory.
pub fn discard_file(repo_dir: &Path, cache_dir: &Path, rel_path: &str) -> AppResult<()> {
    let valid_rel = validate_rel_path(rel_path)?;
    let rel_str = valid_rel.to_string_lossy().replace('\\', "/");
    let full_path = repo_dir.join(&valid_rel);

    if is_file_tracked(repo_dir, &rel_str) {
        let diff_out = run_git_checked(repo_dir, &["diff", "--no-color", "--", &rel_str])?;
        if !diff_out.stdout.is_empty() {
            save_discard_backup(cache_dir, &rel_str, &diff_out.stdout, "patch")?;
        }
        run_git_checked(repo_dir, &["checkout", "--", &rel_str])?;
    } else if full_path.exists() {
        if full_path.is_file() {
            if let Ok(bytes) = fs::read(&full_path) {
                let _ = save_discard_backup(cache_dir, &rel_str, &bytes, "bak");
            }
        }
        if trash::delete(&full_path).is_err() {
            if full_path.is_dir() {
                fs::remove_dir_all(&full_path)?;
            } else {
                fs::remove_file(&full_path)?;
            }
        }
    }

    Ok(())
}

/// Stages a single hunk by index using `git apply --cached`.
pub fn stage_hunk(repo_dir: &Path, rel_path: &str, hunk_index: usize) -> AppResult<()> {
    let valid_rel = validate_rel_path(rel_path)?;
    let rel_str = valid_rel.to_string_lossy().replace('\\', "/");

    let raw = hunk::git_diff_file(repo_dir, &rel_str, false)?;
    let parsed = parse_file_diff(&raw)?
        .ok_or_else(|| HunkError::InvalidDiff("no unstaged diff found".into()))?;
    let patch = build_hunk_patch(&parsed, hunk_index)?;
    apply_hunk_patch(repo_dir, &patch, true, false)?;
    Ok(())
}

/// Unstages a single staged hunk by index using `git apply --cached --reverse`.
pub fn unstage_hunk(repo_dir: &Path, rel_path: &str, hunk_index: usize) -> AppResult<()> {
    let valid_rel = validate_rel_path(rel_path)?;
    let rel_str = valid_rel.to_string_lossy().replace('\\', "/");

    let raw = hunk::git_diff_file(repo_dir, &rel_str, true)?;
    let parsed = parse_file_diff(&raw)?
        .ok_or_else(|| HunkError::InvalidDiff("no staged diff found".into()))?;
    let patch = build_hunk_patch(&parsed, hunk_index)?;
    apply_hunk_patch(repo_dir, &patch, true, true)?;
    Ok(())
}

/// Discards a single unstaged hunk by index after archiving the patch in the cache directory.
pub fn discard_hunk(
    repo_dir: &Path,
    cache_dir: &Path,
    rel_path: &str,
    hunk_index: usize,
) -> AppResult<()> {
    let valid_rel = validate_rel_path(rel_path)?;
    let rel_str = valid_rel.to_string_lossy().replace('\\', "/");

    let raw = hunk::git_diff_file(repo_dir, &rel_str, false)?;
    let parsed = parse_file_diff(&raw)?
        .ok_or_else(|| HunkError::InvalidDiff("no unstaged diff found".into()))?;
    let patch = build_hunk_patch(&parsed, hunk_index)?;
    save_discard_backup(cache_dir, &rel_str, patch.as_bytes(), "patch")?;
    apply_hunk_patch(repo_dir, &patch, false, true)?;
    Ok(())
}

/// Commits staged changes with the given commit message.
pub fn commit(repo_dir: &Path, message: &str) -> AppResult<()> {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(
            "commit message cannot be empty".into(),
        ));
    }
    run_git_checked(repo_dir, &["commit", "-m", trimmed])?;
    Ok(())
}

/// Lists local branches and active branch name.
pub fn branches(repo_dir: &Path) -> AppResult<GitBranchesDto> {
    let output = run_git_checked(repo_dir, &["branch", "--format=%(HEAD)|%(refname:short)"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut current = None;
    let mut list = Vec::new();

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some((head_marker, name)) = trimmed.split_once('|') {
            let branch_name = name.trim().to_string();
            if !branch_name.is_empty() {
                if head_marker.trim() == "*" {
                    current = Some(branch_name.clone());
                }
                list.push(branch_name);
            }
        }
    }

    if current.is_none() {
        if let Ok(head_out) = run_git_raw(repo_dir, &["symbolic-ref", "--short", "HEAD"], 5) {
            if head_out.status.success() {
                let sym = String::from_utf8_lossy(&head_out.stdout).trim().to_string();
                if !sym.is_empty() {
                    current = Some(sym.clone());
                    if !list.contains(&sym) {
                        list.push(sym);
                    }
                }
            }
        }
    }

    Ok(GitBranchesDto {
        current,
        branches: list,
    })
}

/// Switches to the specified local branch.
pub fn checkout(repo_dir: &Path, branch: &str) -> AppResult<()> {
    let trimmed = branch.trim();
    if trimmed.is_empty() || trimmed.starts_with('-') {
        return Err(AppError::Validation("invalid branch name".into()));
    }
    run_git_checked(repo_dir, &["checkout", trimmed])?;
    Ok(())
}

/// Runs `git pull --ff-only`.
pub fn pull(repo_dir: &Path) -> AppResult<()> {
    run_git_checked(repo_dir, &["pull", "--ff-only"])?;
    Ok(())
}

/// Runs `git push`.
pub fn push(repo_dir: &Path) -> AppResult<()> {
    run_git_checked(repo_dir, &["push"])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::parse::GitBadge;
    use tempfile::TempDir;

    fn init_repo(dir: &Path) -> AppResult<()> {
        run_git_checked(dir, &["init", "-b", "main"])?;
        run_git_checked(dir, &["config", "user.name", "Test User"])?;
        run_git_checked(dir, &["config", "user.email", "test@example.com"])?;
        Ok(())
    }

    #[test]
    fn non_git_directory_returns_is_repo_false() -> AppResult<()> {
        let tmp = TempDir::new()?;
        let st = status(tmp.path())?;
        assert!(st.git_available);
        assert!(!st.is_repo);
        Ok(())
    }

    #[test]
    fn stage_unstage_discard_commit_and_branch_workflow() -> AppResult<()> {
        let tmp = TempDir::new()?;
        let repo = tmp.path().join("repo");
        let cache = tmp.path().join("cache");
        fs::create_dir_all(&repo)?;
        init_repo(&repo)?;

        fs::write(repo.join("hello.txt"), "line 1\nline 2\n")?;
        let st1 = status(&repo)?;
        assert!(st1.is_repo);
        assert_eq!(st1.branch.as_deref(), Some("main"));
        assert_eq!(st1.untracked.len(), 1);
        assert_eq!(st1.untracked[0].badge, GitBadge::Untracked);

        let untracked_diff = diff(&repo, "hello.txt", false)?;
        assert!(!untracked_diff.is_binary);
        assert_eq!(
            untracked_diff
                .file_diff
                .as_ref()
                .map(|fd| fd.hunks.len())
                .unwrap_or(0),
            1
        );

        stage_file(&repo, "hello.txt")?;
        let st2 = status(&repo)?;
        assert_eq!(st2.staged.len(), 1);
        assert_eq!(st2.staged[0].badge, GitBadge::Added);

        unstage_file(&repo, "hello.txt")?;
        let st3 = status(&repo)?;
        assert!(st3.staged.is_empty());
        assert_eq!(st3.untracked.len(), 1);

        stage_file(&repo, "hello.txt")?;
        commit(&repo, "initial commit")?;

        fs::write(repo.join("hello.txt"), "line 1\nline 2 updated\n")?;
        discard_file(&repo, &cache, "hello.txt")?;
        assert_eq!(
            fs::read_to_string(repo.join("hello.txt"))?,
            "line 1\nline 2\n"
        );
        assert!(cache.join("discarded").exists());

        run_git_checked(&repo, &["branch", "feature-a"])?;
        let br = branches(&repo)?;
        assert_eq!(br.current.as_deref(), Some("main"));
        assert!(br.branches.contains(&"feature-a".to_string()));

        checkout(&repo, "feature-a")?;
        assert_eq!(branches(&repo)?.current.as_deref(), Some("feature-a"));

        Ok(())
    }

    #[test]
    fn handles_binary_no_newline_crlf_delete_and_hunk_discard() -> AppResult<()> {
        let tmp = TempDir::new()?;
        let repo = tmp.path().join("repo");
        let cache = tmp.path().join("cache");
        fs::create_dir_all(&repo)?;
        init_repo(&repo)?;

        // File without trailing newline + CRLF file + binary file
        fs::write(repo.join("no_nl.txt"), "first\nsecond")?;
        fs::write(repo.join("crlf.txt"), "a\r\nb\r\nc\r\n")?;
        fs::write(repo.join("bin.dat"), [0x00, 0x01, 0x02, 0xff])?;
        stage_file(&repo, "no_nl.txt")?;
        stage_file(&repo, "crlf.txt")?;
        stage_file(&repo, "bin.dat")?;
        commit(&repo, "seed files")?;

        fs::write(repo.join("no_nl.txt"), "first\nsecond changed")?;
        fs::write(repo.join("crlf.txt"), "a\r\nb modified\r\nc\r\n")?;
        fs::write(repo.join("bin.dat"), [0x00, 0x09, 0x08, 0xff])?;

        let d_no_nl = diff(&repo, "no_nl.txt", false)?;
        assert!(d_no_nl.raw_diff.contains("No newline at end of file"));
        assert_eq!(
            d_no_nl
                .file_diff
                .as_ref()
                .map(|f| f.hunks.len())
                .unwrap_or(0),
            1
        );

        let d_crlf = diff(&repo, "crlf.txt", false)?;
        assert_eq!(
            d_crlf
                .file_diff
                .as_ref()
                .map(|f| f.hunks.len())
                .unwrap_or(0),
            1
        );

        let d_bin = diff(&repo, "bin.dat", false)?;
        assert!(d_bin.is_binary);
        assert!(d_bin.file_diff.is_none());

        discard_hunk(&repo, &cache, "no_nl.txt", 0)?;
        assert_eq!(fs::read_to_string(repo.join("no_nl.txt"))?, "first\nsecond");

        Ok(())
    }
}
