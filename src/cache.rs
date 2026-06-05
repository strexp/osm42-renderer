use std::{
    path::PathBuf,
    time::{Duration, SystemTime},
};
use walkdir::WalkDir;

pub fn prune_expired_cache_blocking(dir: &PathBuf, ttl: Duration) {
    let now = SystemTime::now();
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age > ttl {
                            let _ = std::fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
    }
}
