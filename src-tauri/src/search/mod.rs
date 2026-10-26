use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

pub const DEFAULT_MAX_SEARCH_RESULTS: usize = 2000;
pub const DEFAULT_SNIPPET_MAX_CHARS: usize = 300;
const BATCH_EMIT_SIZE: usize = 60;
const BATCH_EMIT_INTERVAL: Duration = Duration::from_millis(35);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQueryOptions {
    pub query_id: String,
    pub pattern: String,
    pub case_sensitive: bool,
    pub regex: bool,
    pub glob: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatchDto {
    pub rel_path: String,
    pub line: u32,
    pub col: u32,
    pub snippet: String,
    pub match_start: usize,
    pub match_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultsEvent {
    pub query_id: String,
    pub items: Vec<SearchMatchDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchDoneEvent {
    pub query_id: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchErrorEvent {
    pub query_id: String,
    pub message: String,
}

pub struct ActiveSearchHandle {
    pub query_id: String,
    pub cancelled: Arc<AtomicBool>,
    pub pid: Option<u32>,
}

impl ActiveSearchHandle {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Some(pid) = self.pid {
            let pid_str = pid.to_string();
            let _ = Command::new("kill").args(["-TERM", &pid_str]).status();
        }
    }
}

/// Resolves the bundled ripgrep sidecar binary or falls back to system `rg`.
pub fn resolve_rg_binary() -> PathBuf {
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate_plain = dir.join("rg");
            if candidate_plain.is_file() {
                return candidate_plain;
            }
            let candidate_triple = dir.join("rg-x86_64-unknown-linux-gnu");
            if candidate_triple.is_file() {
                return candidate_triple;
            }
        }
    }

    let dev_sidecar = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join("rg-x86_64-unknown-linux-gnu");
    if dev_sidecar.is_file() {
        return dev_sidecar;
    }

    PathBuf::from("rg")
}

/// Builds the argument list for `rg --json` with `-e <pattern>` separated from flags.
pub fn build_rg_args(opts: &SearchQueryOptions) -> Vec<String> {
    let mut args = vec![
        "--json".to_string(),
        "--line-number".to_string(),
        "--column".to_string(),
        "--no-heading".to_string(),
        "--color=never".to_string(),
        "--glob=!.git".to_string(),
        "--glob=!node_modules".to_string(),
        "--glob=!target".to_string(),
    ];

    if opts.case_sensitive {
        args.push("--case-sensitive".to_string());
    } else {
        args.push("--ignore-case".to_string());
    }

    if !opts.regex {
        args.push("--fixed-strings".to_string());
    }

    if let Some(raw_glob) = opts.glob.as_deref() {
        for part in raw_glob.split(',') {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                args.push("--glob".to_string());
                args.push(trimmed.to_string());
            }
        }
    }

    args.push("-e".to_string());
    args.push(opts.pattern.clone());
    args.push(".".to_string());
    args
}

/// Parses a single `rg --json` line into a `SearchMatchDto` if it is a `"match"` event.
pub fn parse_rg_json_line(line: &str, max_snippet_chars: usize) -> Option<SearchMatchDto> {
    let parsed: serde_json::Value = serde_json::from_str(line).ok()?;
    if parsed.get("type")?.as_str()? != "match" {
        return None;
    }

    let data = parsed.get("data")?;
    let raw_path = data.get("path")?.get("text")?.as_str()?;
    let rel_path = raw_path
        .strip_prefix("./")
        .unwrap_or(raw_path)
        .replace('\\', "/");

    let line_num = data.get("line_number")?.as_u64()? as u32;
    let raw_line_text = data.get("lines")?.get("text")?.as_str().unwrap_or("");
    let trimmed_newline = raw_line_text.trim_end_matches(['\r', '\n']);

    let (byte_start, byte_end) = data
        .get("submatches")
        .and_then(|s| s.as_array())
        .and_then(|arr| arr.first())
        .map(|sub| {
            let st = sub.get("start").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let en = sub.get("end").and_then(|v| v.as_u64()).unwrap_or(st as u64) as usize;
            (st, en)
        })
        .unwrap_or((0, 0));

    let safe_start = byte_start.min(trimmed_newline.len());
    let safe_end = byte_end.min(trimmed_newline.len()).max(safe_start);

    let char_start = trimmed_newline
        .get(..safe_start)
        .map(|s| s.chars().count())
        .unwrap_or(0);
    let char_end = trimmed_newline
        .get(..safe_end)
        .map(|s| s.chars().count())
        .unwrap_or(char_start);

    let total_chars = trimmed_newline.chars().count();
    let (snippet, match_start, match_end) = if total_chars <= max_snippet_chars {
        (trimmed_newline.to_string(), char_start, char_end)
    } else {
        let truncated: String = trimmed_newline.chars().take(max_snippet_chars).collect();
        (
            format!("{}...", truncated),
            char_start.min(max_snippet_chars),
            char_end.min(max_snippet_chars),
        )
    };

    Some(SearchMatchDto {
        rel_path,
        line: line_num,
        col: (char_start as u32) + 1,
        snippet,
        match_start,
        match_end,
    })
}

pub fn spawn_rg_process(workspace_root: &Path, opts: &SearchQueryOptions) -> AppResult<Child> {
    if opts.pattern.is_empty() {
        return Err(AppError::Validation(
            "search pattern cannot be empty".into(),
        ));
    }

    let rg_bin = resolve_rg_binary();
    let args = build_rg_args(opts);

    let child = Command::new(&rg_bin)
        .current_dir(workspace_root)
        .env("LC_ALL", "C")
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| {
            AppError::ExternalCommand(format!(
                "gagal menjalankan ripgrep ({}): {}",
                rg_bin.display(),
                err
            ))
        })?;

    Ok(child)
}

/// Streams `rg --json` output in batches until completion, cancellation, or `max_results`.
pub fn consume_rg_stream<R, D, E>(
    mut child: Child,
    cancelled: Arc<AtomicBool>,
    max_results: usize,
    max_snippet_chars: usize,
    mut on_batch: R,
    on_done: D,
    on_error: E,
) where
    R: FnMut(Vec<SearchMatchDto>),
    D: FnOnce(bool),
    E: FnOnce(String),
{
    let stdout = match child.stdout.take() {
        Some(out) => out,
        None => {
            on_error("missing stdout from rg process".into());
            return;
        }
    };

    let reader = BufReader::new(stdout);
    let mut batch: Vec<SearchMatchDto> = Vec::with_capacity(BATCH_EMIT_SIZE);
    let mut total_count = 0usize;
    let mut truncated = false;
    let mut last_flush = Instant::now();

    for line_res in reader.lines() {
        if cancelled.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return;
        }

        let line = match line_res {
            Ok(l) => l,
            Err(_) => break,
        };

        if let Some(matched) = parse_rg_json_line(&line, max_snippet_chars) {
            batch.push(matched);
            total_count += 1;

            if batch.len() >= BATCH_EMIT_SIZE || last_flush.elapsed() >= BATCH_EMIT_INTERVAL {
                on_batch(std::mem::take(&mut batch));
                last_flush = Instant::now();
            }

            if total_count >= max_results {
                truncated = true;
                let _ = child.kill();
                break;
            }
        }
    }

    if cancelled.load(Ordering::Relaxed) {
        let _ = child.kill();
        let _ = child.wait();
        return;
    }

    if !batch.is_empty() {
        on_batch(batch);
    }

    match child.wait() {
        Ok(status) => {
            // Exit code 0 = matches found, 1 = no matches, 2 = error (unless killed due to truncation)
            if !truncated && status.code() == Some(2) {
                if let Some(mut stderr) = child.stderr.take() {
                    let mut err_buf = String::new();
                    let _ = std::io::Read::read_to_string(&mut stderr, &mut err_buf);
                    let msg = err_buf.trim();
                    if !msg.is_empty() {
                        on_error(msg.to_string());
                        return;
                    }
                }
            }
            on_done(truncated);
        }
        Err(err) => {
            on_error(err.to_string());
        }
    }
}

pub fn cancel_active_search(slot: &Mutex<Option<ActiveSearchHandle>>) {
    if let Ok(mut guard) = slot.lock() {
        if let Some(existing) = guard.take() {
            existing.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn parses_rg_json_match_and_truncates_long_snippets() {
        let sample = r#"{"type":"match","data":{"path":{"text":"./src/main.rs"},"lines":{"text":"fn main() { println!(\"hello\"); }\n"},"line_number":12,"absolute_offset":100,"submatches":[{"match":{"text":"println"},"start":12,"end":19}]}}"#;
        let dto = parse_rg_json_line(sample, 300).expect("valid match");
        assert_eq!(dto.rel_path, "src/main.rs");
        assert_eq!(dto.line, 12);
        assert_eq!(dto.col, 13);
        assert_eq!(dto.match_start, 12);
        assert_eq!(dto.match_end, 19);

        let short = parse_rg_json_line(sample, 10).expect("valid truncated match");
        assert_eq!(short.snippet, "fn main() ...");
    }

    #[test]
    fn executes_rg_search_with_glob_and_respects_result_cap() -> AppResult<()> {
        let tmp = TempDir::new()?;
        fs::write(
            tmp.path().join("alpha.rs"),
            "needle one\nneedle two\nneedle three\nneedle four\n",
        )?;
        fs::write(tmp.path().join("beta.txt"), "needle ignored by glob\n")?;

        let opts = SearchQueryOptions {
            query_id: "q-1".into(),
            pattern: "needle".into(),
            case_sensitive: true,
            regex: false,
            glob: Some("*.rs".into()),
        };

        let child = spawn_rg_process(tmp.path(), &opts)?;
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut collected: Vec<SearchMatchDto> = Vec::new();
        let mut was_truncated = false;

        consume_rg_stream(
            child,
            cancelled,
            3,
            300,
            |batch| collected.extend(batch),
            |trunc| was_truncated = trunc,
            |err| panic!("unexpected rg error: {}", err),
        );

        assert_eq!(collected.len(), 3);
        assert!(was_truncated);
        assert!(collected.iter().all(|m| m.rel_path == "alpha.rs"));
        Ok(())
    }
}
