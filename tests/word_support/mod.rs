//! Helpers for the tests that need Word (`windows_word`). Every test starts its
//! own hidden Word and never touches a Word that the user has open.
#![allow(dead_code)] // each test file uses a different part of this module

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use adapdf::word::WordSession;

/// One Word test at a time.
pub static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

pub fn exclusive() -> MutexGuard<'static, ()> {
    // A test that panicked on purpose must not stop the others.
    ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner())
}

/// The ids of the `WINWORD.EXE` processes, read with `tasklist` and not with
/// the code under test.
pub fn word_pids() -> BTreeSet<u32> {
    let out = Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq WINWORD.EXE", "/FO", "CSV", "/NH"])
        .output()
        .expect("run tasklist");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|line| line.to_ascii_uppercase().starts_with("\"WINWORD.EXE\""))
        .filter_map(|line| line.split("\",\"").nth(1)?.parse().ok())
        .collect()
}

/// Waits (for at most 20 seconds) until the Word processes are the same as
/// `expected`, and returns the ones that are there at the end.
pub fn wait_for_word_pids(expected: &BTreeSet<u32>) -> BTreeSet<u32> {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let now = word_pids();
        if now == *expected || Instant::now() >= deadline {
            return now;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

pub fn kill(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .output();
}

/// A folder for files that a test makes, removed when the value is dropped.
/// `path` and `write` work in `folder`, which is the folder itself, or the
/// guide folder `Guide - Adapdf` inside it ([`TempDir::guide`]).
pub struct TempDir {
    root: PathBuf,
    folder: PathBuf,
}

impl TempDir {
    pub fn new(name: &str) -> TempDir {
        let dir = std::env::temp_dir().join(format!("adapdf-word-{name}-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("make the temp folder");
        TempDir {
            root: dir.clone(),
            folder: dir,
        }
    }

    /// A temp folder that holds the guide folder `Guide - Adapdf`, where the
    /// accessible copy `accessible.docx` is. `path` and `write` are the guide
    /// folder's.
    pub fn guide(name: &str) -> TempDir {
        let mut dir = TempDir::new(name);
        dir.folder = dir.root.join("Guide - Adapdf");
        fs::create_dir_all(&dir.folder).expect("make the guide folder");
        dir
    }

    pub fn path(&self) -> &Path {
        &self.folder
    }

    pub fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.folder.join(name);
        fs::write(&path, bytes).expect("write the test file");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Starts a session on a new thread, runs `work`, drops the session, and gives
/// the result. When Word does not answer within `limit`, the test fails and the
/// Word that the test started is ended, so a hang cannot go on.
pub fn within<T: Send + 'static>(
    limit: Duration,
    work: impl FnOnce(&WordSession) -> T + Send + 'static,
) -> T {
    let pid = Arc::new(AtomicU32::new(0));
    let pid_seen = Arc::clone(&pid);
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let session = WordSession::start().expect("start Word");
        pid_seen.store(session.pid(), Ordering::SeqCst);
        let result = work(&session);
        drop(session);
        let _ = sender.send(result);
    });
    match receiver.recv_timeout(limit) {
        Ok(result) => result,
        Err(error) => {
            let started = pid.load(Ordering::SeqCst);
            if started != 0 {
                kill(started);
            }
            panic!("Word did not finish within {limit:?} ({error})");
        }
    }
}
