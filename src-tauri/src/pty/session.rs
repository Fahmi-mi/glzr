use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const READ_CHUNK_BYTES: usize = 32 * 1024;
const BATCH_MAX_BYTES: usize = 64 * 1024;
const CHANNEL_BOUND: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum PtyError {
    #[error("pty error: {0}")]
    Pty(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("lock poisoned")]
    LockPoisoned,
}

pub struct PtySession {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    child: Mutex<Box<dyn portable_pty::Child + Send + Sync>>,
    stop_flag: Arc<AtomicBool>,
    reader_handle: Mutex<Option<JoinHandle<()>>>,
}

impl PtySession {
    pub fn spawn(
        program: &str,
        args: &[&str],
        cwd: &Path,
        cols: u16,
        rows: u16,
    ) -> Result<(Self, Receiver<String>), PtyError> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::Pty(e.to_string()))?;

        let mut cmd = CommandBuilder::new(program);
        cmd.args(args);
        cmd.cwd(cwd);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("TERM_PROGRAM", "glzr");
        cmd.env_remove("WEBKIT_DISABLE_DMABUF_RENDERER");
        cmd.env_remove("WEBKIT_DISABLE_COMPOSITING_MODE");
        cmd.env_remove("APPIMAGE");
        cmd.env_remove("APPDIR");
        cmd.env_remove("OWD");

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::Pty(e.to_string()))?;
        drop(pair.slave);

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::Pty(e.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::Pty(e.to_string()))?;

        let (tx, rx) = mpsc::sync_channel::<String>(CHANNEL_BOUND);
        let stop_flag = Arc::new(AtomicBool::new(false));
        let reader_stop = Arc::clone(&stop_flag);

        let reader_handle = thread::spawn(move || {
            run_batched_reader(reader, tx, reader_stop);
        });

        Ok((
            Self {
                master: Mutex::new(pair.master),
                writer: Mutex::new(writer),
                child: Mutex::new(child),
                stop_flag,
                reader_handle: Mutex::new(Some(reader_handle)),
            },
            rx,
        ))
    }

    pub fn write(&self, input: &[u8]) -> Result<(), PtyError> {
        let mut writer = self.writer.lock().map_err(|_| PtyError::LockPoisoned)?;
        writer.write_all(input)?;
        writer.flush()?;
        Ok(())
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), PtyError> {
        let master = self.master.lock().map_err(|_| PtyError::LockPoisoned)?;
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::Pty(e.to_string()))?;
        Ok(())
    }

    pub fn wait_exit_code(&self) -> Option<i32> {
        let mut child = self.child.lock().ok()?;
        let status = child.wait().ok()?;
        Some(status.exit_code() as i32)
    }

    pub fn kill(&self) -> Result<(), PtyError> {
        self.stop_flag.store(true, Ordering::SeqCst);
        let mut child = self.child.lock().map_err(|_| PtyError::LockPoisoned)?;
        if let Some(pid) = child.process_id() {
            let neg_pgid = format!("-{}", pid);
            let _ = Command::new("kill").args(["-TERM", &neg_pgid]).status();
            let _ = Command::new("kill").args(["-KILL", &neg_pgid]).status();
        }
        let _ = child.kill();
        let _ = child.wait();
        if let Ok(mut guard) = self.reader_handle.lock() {
            if let Some(handle) = guard.take() {
                let _ = handle.join();
            }
        }
        Ok(())
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        let _ = self.kill();
    }
}

pub fn decode_utf8_stream(pending: &mut Vec<u8>, incoming: &[u8]) -> String {
    pending.extend_from_slice(incoming);
    let mut decoded = String::new();

    loop {
        match std::str::from_utf8(pending) {
            Ok(valid) => {
                decoded.push_str(valid);
                pending.clear();
                break;
            }
            Err(err) => {
                let valid_up_to = err.valid_up_to();
                if valid_up_to > 0 {
                    if let Ok(valid_prefix) = std::str::from_utf8(&pending[..valid_up_to]) {
                        decoded.push_str(valid_prefix);
                    }
                    pending.drain(..valid_up_to);
                }

                match err.error_len() {
                    None => break,
                    Some(invalid_len) => {
                        decoded.push(std::char::REPLACEMENT_CHARACTER);
                        pending.drain(..invalid_len);
                    }
                }
            }
        }
    }

    decoded
}

fn run_batched_reader(
    mut reader: Box<dyn Read + Send>,
    tx: SyncSender<String>,
    stop_flag: Arc<AtomicBool>,
) {
    let (raw_tx, raw_rx) = mpsc::sync_channel::<String>(CHANNEL_BOUND);
    let stop_reader = Arc::clone(&stop_flag);

    let reader_thread = thread::spawn(move || {
        let mut read_buf = vec![0u8; READ_CHUNK_BYTES];
        let mut utf8_pending = Vec::with_capacity(4);

        loop {
            if stop_reader.load(Ordering::Relaxed) {
                break;
            }

            match reader.read(&mut read_buf) {
                Ok(0) => {
                    if !utf8_pending.is_empty() {
                        let fallback = String::from_utf8_lossy(&utf8_pending).into_owned();
                        let _ = raw_tx.send(fallback);
                    }
                    break;
                }
                Ok(n) => {
                    let chunk_str = decode_utf8_stream(&mut utf8_pending, &read_buf[..n]);
                    if !chunk_str.is_empty() && raw_tx.send(chunk_str).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    if !utf8_pending.is_empty() {
                        let fallback = String::from_utf8_lossy(&utf8_pending).into_owned();
                        let _ = raw_tx.send(fallback);
                    }
                    break;
                }
            }
        }
    });

    let mut batch = String::with_capacity(BATCH_MAX_BYTES);
    let poll_timeout = Duration::from_millis(2);

    while let Ok(first) = raw_rx.recv() {
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }

        batch.push_str(&first);

        if batch.len() >= 1024 {
            while batch.len() < BATCH_MAX_BYTES {
                match raw_rx.recv_timeout(poll_timeout) {
                    Ok(more) => batch.push_str(&more),
                    Err(_) => break,
                }
            }
        } else {
            while batch.len() < BATCH_MAX_BYTES {
                match raw_rx.try_recv() {
                    Ok(more) => batch.push_str(&more),
                    Err(_) => break,
                }
            }
        }

        let outgoing = std::mem::take(&mut batch);
        match tx.try_send(outgoing) {
            Ok(()) => {}
            Err(TrySendError::Full(msg)) => {
                if tx.send(msg).is_err() {
                    break;
                }
            }
            Err(TrySendError::Disconnected(_)) => break,
        }
    }

    let _ = reader_thread.join();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn utf8_incomplete_boundary_is_buffered_cleanly() {
        let mut pending = Vec::new();
        let part1 = [b'a', 0xF0, 0x9F];
        let part2 = [0x9A, 0x80, b'z'];

        let out1 = decode_utf8_stream(&mut pending, &part1);
        assert_eq!(out1, "a");
        assert_eq!(pending, vec![0xF0, 0x9F]);

        let out2 = decode_utf8_stream(&mut pending, &part2);
        assert_eq!(out2, "\u{1F680}z");
        assert!(pending.is_empty());
    }

    #[test]
    fn spike_pty_handles_100mb_output_with_batching_and_backpressure() -> Result<(), PtyError> {
        let cwd = env::current_dir()?;
        let (session, rx) = PtySession::spawn(
            "sh",
            &[
                "-c",
                "dd if=/dev/zero bs=1048576 count=100 2>/dev/null | tr '\\0' 'y'",
            ],
            &cwd,
            80,
            24,
        )?;

        let mut total_bytes = 0usize;
        let mut batch_count = 0usize;
        while let Ok(chunk) = rx.recv() {
            total_bytes += chunk.len();
            batch_count += 1;
        }

        assert_eq!(total_bytes, 104_857_600);
        assert!(batch_count < 10_000);
        drop(session);
        Ok(())
    }

    #[test]
    fn spike_pty_resize_and_kill_terminates_process_group() -> Result<(), PtyError> {
        let cwd = env::current_dir()?;
        let (session, _rx) = PtySession::spawn("sh", &["-c", "sleep 60 & wait"], &cwd, 80, 24)?;
        session.resize(120, 40)?;
        session.kill()?;
        Ok(())
    }
}
