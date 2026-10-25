use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffHunk {
    pub index: usize,
    pub header: String,
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub file_header: String,
    pub hunks: Vec<DiffHunk>,
}

#[derive(Debug, thiserror::Error)]
pub enum HunkError {
    #[error("invalid diff format: {0}")]
    InvalidDiff(String),
    #[error("hunk index {0} out of bounds")]
    HunkOutOfBounds(usize),
    #[error("git command failed: {0}")]
    GitCommand(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn parse_file_diff(raw_diff: &str) -> Result<Option<FileDiff>, HunkError> {
    if raw_diff.trim().is_empty() {
        return Ok(None);
    }

    let mut header_lines = Vec::new();
    let mut hunks = Vec::new();
    let mut current_header: Option<(String, u32, u32, u32, u32)> = None;
    let mut current_lines: Vec<&str> = Vec::new();

    for line in raw_diff.lines() {
        if line.starts_with("@@ ") {
            if let Some((hdr, os, ol, ns, nl)) = current_header.take() {
                let mut content = current_lines.join("\n");
                content.push('\n');
                hunks.push(DiffHunk {
                    index: hunks.len(),
                    header: hdr,
                    old_start: os,
                    old_lines: ol,
                    new_start: ns,
                    new_lines: nl,
                    content,
                });
                current_lines.clear();
            }

            let (os, ol, ns, nl) = parse_hunk_header(line)?;
            current_header = Some((line.to_string(), os, ol, ns, nl));
            current_lines.push(line);
        } else if current_header.is_some() {
            current_lines.push(line);
        } else {
            header_lines.push(line);
        }
    }

    if let Some((hdr, os, ol, ns, nl)) = current_header.take() {
        let mut content = current_lines.join("\n");
        content.push('\n');
        hunks.push(DiffHunk {
            index: hunks.len(),
            header: hdr,
            old_start: os,
            old_lines: ol,
            new_start: ns,
            new_lines: nl,
            content,
        });
    }

    if header_lines.is_empty() {
        return Err(HunkError::InvalidDiff("missing file diff header".into()));
    }

    let mut file_header = header_lines.join("\n");
    file_header.push('\n');

    Ok(Some(FileDiff { file_header, hunks }))
}

fn parse_hunk_header(line: &str) -> Result<(u32, u32, u32, u32), HunkError> {
    let after_prefix = line
        .strip_prefix("@@ ")
        .ok_or_else(|| HunkError::InvalidDiff(line.to_string()))?;
    let end_idx = after_prefix
        .find(" @@")
        .ok_or_else(|| HunkError::InvalidDiff(line.to_string()))?;
    let range_part = &after_prefix[..end_idx];

    let mut parts = range_part.split_whitespace();
    let old_part = parts
        .next()
        .and_then(|s| s.strip_prefix('-'))
        .ok_or_else(|| HunkError::InvalidDiff(line.to_string()))?;
    let new_part = parts
        .next()
        .and_then(|s| s.strip_prefix('+'))
        .ok_or_else(|| HunkError::InvalidDiff(line.to_string()))?;

    let (old_start, old_lines) = parse_range(old_part)?;
    let (new_start, new_lines) = parse_range(new_part)?;

    Ok((old_start, old_lines, new_start, new_lines))
}

fn parse_range(range_str: &str) -> Result<(u32, u32), HunkError> {
    if let Some((start, count)) = range_str.split_once(',') {
        let s = start
            .parse::<u32>()
            .map_err(|_| HunkError::InvalidDiff(range_str.to_string()))?;
        let c = count
            .parse::<u32>()
            .map_err(|_| HunkError::InvalidDiff(range_str.to_string()))?;
        Ok((s, c))
    } else {
        let s = range_str
            .parse::<u32>()
            .map_err(|_| HunkError::InvalidDiff(range_str.to_string()))?;
        Ok((s, 1))
    }
}

pub fn build_hunk_patch(diff: &FileDiff, hunk_index: usize) -> Result<String, HunkError> {
    let hunk = diff
        .hunks
        .get(hunk_index)
        .ok_or(HunkError::HunkOutOfBounds(hunk_index))?;

    let mut patch = String::with_capacity(diff.file_header.len() + hunk.content.len());
    patch.push_str(&diff.file_header);
    patch.push_str(&hunk.content);
    Ok(patch)
}

pub fn git_diff_file(repo_dir: &Path, rel_path: &str, staged: bool) -> Result<String, HunkError> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_dir)
        .env("LC_ALL", "C")
        .env("GIT_TERMINAL_PROMPT", "0")
        .arg("diff");

    if staged {
        cmd.arg("--cached");
    }

    cmd.arg("--").arg(rel_path);

    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(HunkError::GitCommand(stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn apply_hunk_patch(
    repo_dir: &Path,
    patch: &str,
    cached: bool,
    reverse: bool,
) -> Result<(), HunkError> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_dir)
        .env("LC_ALL", "C")
        .env("GIT_TERMINAL_PROMPT", "0")
        .arg("apply");

    if cached {
        cmd.arg("--cached");
    }
    if reverse {
        cmd.arg("--reverse");
    }

    cmd.arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(patch.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(HunkError::GitCommand(stderr));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn run_git(dir: &Path, args: &[&str]) -> Result<(), HunkError> {
        let status = Command::new("git")
            .current_dir(dir)
            .env("LC_ALL", "C")
            .args(args)
            .status()?;
        if !status.success() {
            return Err(HunkError::GitCommand(format!("git {:?} failed", args)));
        }
        Ok(())
    }

    #[test]
    fn spike_stage_and_unstage_single_hunk_from_three_hunks() -> Result<(), HunkError> {
        let tmp = TempDir::new()?;
        let repo = tmp.path();

        run_git(repo, &["init", "-b", "main"])?;
        run_git(repo, &["config", "user.name", "Test User"])?;
        run_git(repo, &["config", "user.email", "test@example.com"])?;

        let initial_lines: Vec<String> = (1..=30).map(|i| format!("line {}", i)).collect();
        let file_path = repo.join("sample.txt");
        fs::write(&file_path, format!("{}\n", initial_lines.join("\n")))?;

        run_git(repo, &["add", "sample.txt"])?;
        run_git(repo, &["commit", "-m", "initial commit"])?;

        let mut modified_lines = initial_lines;
        modified_lines[1] = "line 2 modified hunk 1".to_string();
        modified_lines[14] = "line 15 modified hunk 2".to_string();
        modified_lines[28] = "line 29 modified hunk 3".to_string();
        fs::write(&file_path, format!("{}\n", modified_lines.join("\n")))?;

        let raw_unstaged = git_diff_file(repo, "sample.txt", false)?;
        let parsed = parse_file_diff(&raw_unstaged)?
            .ok_or_else(|| HunkError::InvalidDiff("expected non-empty diff".into()))?;
        assert_eq!(parsed.hunks.len(), 3);

        let patch_hunk_2 = build_hunk_patch(&parsed, 1)?;
        apply_hunk_patch(repo, &patch_hunk_2, true, false)?;

        let raw_staged = git_diff_file(repo, "sample.txt", true)?;
        let staged_parsed = parse_file_diff(&raw_staged)?
            .ok_or_else(|| HunkError::InvalidDiff("expected staged diff".into()))?;
        assert_eq!(staged_parsed.hunks.len(), 1);
        assert!(staged_parsed.hunks[0]
            .content
            .contains("+line 15 modified hunk 2"));
        assert!(!staged_parsed.hunks[0].content.contains("modified hunk 1"));
        assert!(!staged_parsed.hunks[0].content.contains("modified hunk 3"));

        let remaining_unstaged = git_diff_file(repo, "sample.txt", false)?;
        let remaining_parsed = parse_file_diff(&remaining_unstaged)?
            .ok_or_else(|| HunkError::InvalidDiff("expected remaining diff".into()))?;
        assert_eq!(remaining_parsed.hunks.len(), 2);

        let unstage_patch = build_hunk_patch(&staged_parsed, 0)?;
        apply_hunk_patch(repo, &unstage_patch, true, true)?;

        let after_unstage = git_diff_file(repo, "sample.txt", true)?;
        assert!(parse_file_diff(&after_unstage)?.is_none());

        Ok(())
    }
}
