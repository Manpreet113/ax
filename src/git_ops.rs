use git2::build::RepoBuilder;
use git2::{FetchOptions, RemoteCallbacks};
use std::path::Path;

/// Clones a repo from `url` to `path` with a progress reporter
pub fn clone_repo(url: &str, path: &Path) -> Result<(), git2::Error> {
    println!(":: Initializing clone for {}", url);

    // 1. Set up the Callbacks
    // These are hooks that trigger during the download process.
    let mut callbacks = RemoteCallbacks::new();

    // The 'transfer_progress' callback is triggered repeatedly during download.
    // 'stats' contains the numbers (total objects, received objects, etc.)
    callbacks.transfer_progress(|stats| {
        if stats.received_objects() == stats.total_objects() {
            // Avoid spamming stdout, only print when a step completes
            print!(
                "\r:: Resolving deltas {}/{}\r",
                stats.indexed_deltas(),
                stats.total_deltas()
            );
        } else if stats.total_objects() > 0 {
            // Calculate percentage
            let p = (stats.received_objects() as f64 / stats.total_objects() as f64) * 100.0;
            print!(
                "\r:: Downloading objects: {}/{} ({:.0}%)",
                stats.received_objects(),
                stats.total_objects(),
                p
            );
        }
        // Return true to continue the transfer. Returning false aborts it.
        true
    });

    // 2. Set up Fetch Options
    // We attach the callbacks to the fetch options.
    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    // 3. Build and Execute
    // We use the Builder pattern to configure the clone operation
    let mut builder = RepoBuilder::new();
    builder.fetch_options(fetch_options);

    // This line actually performs the network request and disk write
    match builder.clone(url, path) {
        Ok(_) => {
            println!("\n:: Clone complete!");
            Ok(())
        }
        Err(e) => Err(e),
    }
}

// Yup cheating here a little >.<
pub fn pull_repo(path: &Path) -> Result<(), git2::Error> {
    println!(":: Checking for updates in {:?}...", path);

    let status = std::process::Command::new("git")
        .current_dir(path)
        .arg("pull")
        .output();

    match status {
        Ok(s) if s.status.success() => Ok(()),
        _ => Err(git2::Error::from_str("Git pull failed")),
    }
}