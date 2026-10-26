use crate::error::{AppError, AppResult};
use crate::fs::DEFAULT_IGNORE_NAMES;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Component, Path};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, RecvTimeoutError},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const DEBOUNCE_WINDOW: Duration = Duration::from_millis(150);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FsChangedEvent {
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FsWatchErrorEvent {
    pub message: String,
}

pub struct WorkspaceWatcher {
    _watcher: RecommendedWatcher,
    stop_flag: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl WorkspaceWatcher {
    pub fn start<F, E>(root_dir: &Path, on_changed: F, on_error: E) -> AppResult<Self>
    where
        F: Fn(Vec<String>) + Send + 'static,
        E: Fn(String) + Send + 'static,
    {
        let canonical_root = root_dir.canonicalize()?;
        let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop_flag);
        let root_for_worker = canonical_root.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default(),
        )
        .map_err(|e| AppError::ExternalCommand(format!("failed to create file watcher: {}", e)))?;

        watcher
            .watch(&canonical_root, RecursiveMode::Recursive)
            .map_err(|e| AppError::ExternalCommand(format!("failed to watch workspace: {}", e)))?;

        let worker = thread::spawn(move || {
            let mut pending: BTreeSet<String> = BTreeSet::new();
            let mut last_event_at: Option<Instant> = None;

            while !worker_stop.load(Ordering::Relaxed) {
                let timeout = if last_event_at.is_some() {
                    DEBOUNCE_WINDOW
                } else {
                    Duration::from_millis(250)
                };

                match rx.recv_timeout(timeout) {
                    Ok(Ok(event)) => {
                        for p in event.paths {
                            if !should_ignore_watch_path(&root_for_worker, &p, DEFAULT_IGNORE_NAMES)
                            {
                                pending.insert(p.to_string_lossy().replace('\\', "/"));
                            }
                        }
                        if !pending.is_empty() {
                            last_event_at = Some(Instant::now());
                        }
                    }
                    Ok(Err(err)) => {
                        on_error(err.to_string());
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        if let Some(ts) = last_event_at {
                            if ts.elapsed() >= DEBOUNCE_WINDOW && !pending.is_empty() {
                                let batch: Vec<String> = pending.iter().cloned().collect();
                                pending.clear();
                                last_event_at = None;
                                on_changed(batch);
                            }
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        Ok(Self {
            _watcher: watcher,
            stop_flag,
            worker: Some(worker),
        })
    }

    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for WorkspaceWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Checks whether a watched path belongs to an ignored folder or atomic temp file.
pub fn should_ignore_watch_path(root: &Path, path: &Path, ignore_names: &[&str]) -> bool {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if name.contains(".tmp.") {
            return true;
        }
    }

    let rel = path.strip_prefix(root).unwrap_or(path);
    for comp in rel.components() {
        if let Component::Normal(os) = comp {
            if let Some(seg) = os.to_str() {
                if ignore_names.contains(&seg) {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::TempDir;

    #[test]
    fn filters_ignored_folders_and_temp_files() {
        let root = Path::new("/workspace/proj");
        assert!(should_ignore_watch_path(
            root,
            Path::new("/workspace/proj/.git/index"),
            DEFAULT_IGNORE_NAMES
        ));
        assert!(should_ignore_watch_path(
            root,
            Path::new("/workspace/proj/node_modules/pkg/index.js"),
            DEFAULT_IGNORE_NAMES
        ));
        assert!(should_ignore_watch_path(
            root,
            Path::new("/workspace/proj/target/debug/glzr"),
            DEFAULT_IGNORE_NAMES
        ));
        assert!(should_ignore_watch_path(
            root,
            Path::new("/workspace/proj/src/.main.rs.tmp.1234"),
            DEFAULT_IGNORE_NAMES
        ));
        assert!(!should_ignore_watch_path(
            root,
            Path::new("/workspace/proj/src/main.rs"),
            DEFAULT_IGNORE_NAMES
        ));
    }

    #[test]
    fn emits_debounced_batch_on_file_change() -> AppResult<()> {
        let tmp = TempDir::new()?;
        let received: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let received_clone = Arc::clone(&received);

        let mut watcher = WorkspaceWatcher::start(
            tmp.path(),
            move |paths| {
                if let Ok(mut guard) = received_clone.lock() {
                    guard.extend(paths);
                }
            },
            |_err| {},
        )?;

        let tracked_file = tmp.path().join("tracked.txt");
        fs::write(&tracked_file, "hello")?;

        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if let Ok(guard) = received.lock() {
                if !guard.is_empty() {
                    break;
                }
            }
            thread::sleep(Duration::from_millis(30));
        }

        watcher.stop();
        let guard = received.lock().map_err(|_| AppError::LockPoisoned)?;
        assert!(guard.iter().any(|p| p.ends_with("tracked.txt")));
        Ok(())
    }
}
