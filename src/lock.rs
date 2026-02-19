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

        loop {
            // Try to create lock file exclusively (atomic operation)
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

                    return Ok(Lock {
                        file,
                        path: lock_path,
                    });
                }
                Err(_) => {
                    // Lock file exists, check if it's stale
                    // We open without truncate/create to read
                    let mut file = match File::open(&lock_path) {
                        Ok(f) => f,
                        Err(_) => continue, // File might have been removed by another process, retry loop
                    };

                    let mut pid_str = String::new();
                    // Read might fail if file is locked? No, read shared?
                    // But we want to just read PID.
                    // If read fails, maybe partial write?
                    if file.read_to_string(&mut pid_str).is_err() {
                        // Corrupt or locked?
                        // If locked by another ax, we shouldn't be able to remove it if we respect locks?
                        // But we don't hold lock to remove.
                        // Safe to assume if we cant read it's garbage or active.
                        // Let's check lock?
                        if file.try_lock_exclusive().is_ok() {
                             // We got lock, so it was garbage.
                             let _ = std::fs::remove_file(&lock_path);
                             continue;
                        } else {
                             // It is locked.
                             anyhow::bail!("ax is already running (Locked)");
                        }
                    }

                    if let Ok(pid) = pid_str.trim().parse::<u32>() {
                        // Use sysinfo to check if process exists and verify it's actually ax
                        let mut sys = System::new();
                        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

                        if let Some(process) = sys.process(sysinfo::Pid::from_u32(pid)) {
                            // Process exists
                            if pid == process::id() {
                                // Self? stale file from previous run of THIS process ID?
                                // Should be handled by create_new failure? 
                                // Removing it is safe.
                                let _ = std::fs::remove_file(&lock_path);
                            } else {
                                let proc_name = process.name().to_string_lossy().to_string();
                                if proc_name.contains("ax") {
                                    anyhow::bail!(
                                        "ax is already running (PID: {}, name: {})",
                                        pid,
                                        proc_name
                                    );
                                } else {
                                    // PID recycled
                                    let _ = std::fs::remove_file(&lock_path);
                                }
                            }
                        } else {
                            // Process dead
                            let _ = std::fs::remove_file(&lock_path);
                        }
                    } else {
                        // Corrupt PID
                         let _ = std::fs::remove_file(&lock_path);
                    }
                    
                    // Loop back to try creating again
                }
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
