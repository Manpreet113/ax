use anyhow::{Context, Result};
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process;
use sysinfo::System;

pub struct Lock {
    file: File,
    path: PathBuf,
}

impl Lock {
    pub fn acquire() -> Result<Self> {
        // Use XDG cache directory instead of /tmp to avoid symlink attacks
        let lock_path = crate::config::Config::get_default_cache_dir().join("ax.lock");

        // Ensure parent directory exists
        if let Some(parent) = lock_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Try to create lock file exclusively (atomic operation)
        // This fixes the TOCTOU race condition
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(mut file) => {
                // Successfully created new lock
                let pid = process::id();
                write!(file, "{}", pid)?;
                file.flush()?;

                // Lock the file to prevent other processes from reading/writing
                file.try_lock_exclusive()
                    .context("Failed to acquire exclusive lock")?;

                Ok(Lock {
                    file,
                    path: lock_path,
                })
            }
            Err(_) => {
                // Lock file exists, check if it's stale
                let file = File::open(&lock_path).context("Failed to open existing lock file")?;

                let mut pid_str = String::new();
                let mut reader = std::io::BufReader::new(file);
                reader.read_to_string(&mut pid_str)?;

                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    // Use sysinfo to check if process exists and verify it's actually ax
                    // This fixes the PID recycling issue
                    let mut sys = System::new();
                    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

                    if let Some(process) = sys.process(sysinfo::Pid::from_u32(pid)) {
                        // Process exists - check if it's the current process or another ax instance
                        if pid == process::id() {
                            // Same process (shouldn't happen, but handle gracefully)
                            std::fs::remove_file(&lock_path)?;
                        } else {
                            // Verify it's actually an ax process by checking the name
                            let proc_name = process.name().to_string_lossy().to_string();
                            if proc_name.contains("ax") {
                                anyhow::bail!(
                                    "ax is already running (PID: {}, name: {})",
                                    pid,
                                    proc_name
                                );
                            } else {
                                // Different process with recycled PID - safe to remove stale lock
                                std::fs::remove_file(&lock_path)?;
                            }
                        }
                    } else {
                        // Process doesn't exist - stale lock
                        std::fs::remove_file(&lock_path)?;
                    }
                } else {
                    // Corrupt lock file
                    std::fs::remove_file(&lock_path)?;
                }

                // Retry lock acquisition after cleanup
                let mut file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&lock_path)?;

                let pid = process::id();
                write!(file, "{}", pid)?;
                file.flush()?;

                file.try_lock_exclusive()
                    .context("Failed to acquire exclusive lock on retry")?;

                Ok(Lock {
                    file,
                    path: lock_path,
                })
            }
        }
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        // Unlock and remove the lock file
        let _ = self.file.unlock();
        let _ = std::fs::remove_file(&self.path);
    }
}
