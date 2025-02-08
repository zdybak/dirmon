use chrono::{FixedOffset, Local};
use notify::{Config, EventKind, PollWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::{collections::HashSet, path::Path, time::Duration};
use walkdir::WalkDir;

fn find_moved_directory(dir_name: &str, search_path: &Path) -> Option<PathBuf> {
    WalkDir::new(search_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .find(|e| e.file_type().is_dir() && e.file_name().to_string_lossy() == dir_name)
        .map(|e| e.path().to_path_buf())
}

fn main() {
    let est_offset = FixedOffset::west_opt(5 * 3600).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();

    // Initialize directory cache for top-level folders
    let mut known_directories: HashSet<PathBuf> = HashSet::new();

    // Scan initial top-level directories
    let watch_path = Path::new("./");
    for entry in std::fs::read_dir(watch_path).unwrap() {
        if let Ok(entry) = entry {
            if entry.path().is_dir() {
                let est_time = Local::now().with_timezone(&est_offset);
                println!(
                    "Initially found directory: {:?}, {}",
                    entry.path(),
                    est_time.format("%Y-%m-%d %H:%M:%S %z")
                );
                known_directories.insert(entry.path());
            }
        }
    }

    let config = Config::default().with_poll_interval(Duration::from_secs(1));
    let mut watcher = PollWatcher::new(tx, config).unwrap();

    watcher.watch(watch_path, RecursiveMode::Recursive).unwrap();

    let est_time = Local::now().with_timezone(&est_offset);
    println!(
        "Monitoring for changes, {}",
        est_time.format("%Y-%m-%d %H:%M:%S %z")
    );

    for e in rx {
        match e {
            Ok(event) => {
                match event.kind {
                    EventKind::Create(_) => {
                        for path in &event.paths {
                            // Check if it's a directory and is at top level
                            if path.is_dir() && path.parent() == Some(watch_path) {
                                //squelch log entries regarding New folder
                                if path != Path::new("./New folder") {
                                    let est_time = Local::now().with_timezone(&est_offset);
                                    println!(
                                        "New top-level directory created: {:?}, {}",
                                        path,
                                        est_time.format("%Y-%m-%d %H:%M:%S %z")
                                    );
                                }
                                known_directories.insert(path.to_path_buf());
                            }
                        }
                    }
                    EventKind::Remove(_) => {
                        for path in &event.paths {
                            if known_directories.contains(path) {
                                let dir_name = path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string();

                                // Search recursively for the moved directory
                                if let Some(new_path) =
                                    find_moved_directory(&dir_name, Path::new("./"))
                                {
                                    let est_time = Local::now().with_timezone(&est_offset);
                                    println!(
                                        "Directory '{}' moved to: {:?}, {}",
                                        dir_name,
                                        new_path,
                                        est_time.format("%Y-%m-%d %H:%M:%S %z")
                                    );
                                    known_directories.remove(path);
                                    // Only add to known directories if it's at top level
                                    if new_path.parent() == Some(watch_path) {
                                        known_directories.insert(new_path);
                                    }
                                } else {
                                    //squelch log entries regarding New folder
                                    if path != Path::new("./New folder") {
                                        let est_time = Local::now().with_timezone(&est_offset);
                                        println!(
                                            "Directory removed: {:?}, {}",
                                            path,
                                            est_time.format("%Y-%m-%d %H:%M:%S %z")
                                        );
                                    }
                                    known_directories.remove(path);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Err(error) => {
                let est_time = Local::now().with_timezone(&est_offset);
                println!(
                    "Error: {:?}, {}",
                    error,
                    est_time.format("%Y-%m-%d %H:%M:%S %z")
                );
            }
        }
    }
}
