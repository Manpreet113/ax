use anyhow::{Context, Result};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process;

pub struct Lock {
    path: PathBuf,
}

impl Lock {
    pub fn acquire() -> Result<Self> {
        let lock_path = std::env::temp_dir().join("ax.lock");

        if lock_path.exists() {
            let mut file = File::open(&lock_path)
                .context("Failed to open existing lock file")?;
            let mut pid_str = String::new();
            file.read_to_string(&mut pid_str)?;

            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                // Check if process exists (Linux specific: /proc/<pid>)
                let proc_path = std::path::Path::new("/proc").join(pid.to_string());
                if proc_path.exists() {
                    // It might be us? (Use std::process::id())
                    if pid != process::id() {
                        anyhow::bail!("ax is already running (PID: {})", pid);
                    }
                } else {
                    // Stale lock
                    std::fs::remove_file(&lock_path).ok();
                }
            } else {
                 // Corrupt/Empty lock
                 std::fs::remove_file(&lock_path).ok();
            }
        }

        // Create new lock
        let mut file = File::create(&lock_path)
            .context("Failed to create lock file")?;
        write!(file, "{}", process::id())?;

        Ok(Lock { path: lock_path })
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        // remove_file returns Result, but we can't return it from Drop.
        // We ignore error (e.g., if file was already deleted).
        let _ = std::fs::remove_file(&self.path);
    }
}
