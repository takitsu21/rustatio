use crate::paths::is_within_depth;
use std::path::{Path, PathBuf};

pub fn is_torrent_file(path: &Path) -> bool {
    path.is_file() && path.extension().is_some_and(|ext| ext == "torrent")
}

pub fn scan_torrent_paths(
    watch_dir: &Path,
    max_depth: u32,
) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut results = Vec::new();
    if !watch_dir.exists() {
        return Ok(results);
    }

    let root = watch_dir.canonicalize().unwrap_or_else(|_| watch_dir.to_path_buf());
    let mut stack = vec![root.clone()];

    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) => {
                if dir == root {
                    return Err(e);
                }
                tracing::warn!("Skipping unreadable watch subdir {}: {}", dir.display(), e);
                continue;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().is_ok_and(|file_type| file_type.is_dir()) {
                if is_within_depth(&root, &path, max_depth, true) {
                    stack.push(path);
                }
                continue;
            }

            if is_torrent_file(&path) && is_within_depth(&root, &path, max_depth, false) {
                results.push(path);
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_torrent(path: &Path) -> Result<(), std::io::Error> {
        fs::write(path, b"test")
    }

    #[test]
    fn test_scan_torrent_paths_depth_limits() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let root = dir.path();

        let root_file = root.join("root.torrent");
        write_torrent(&root_file)?;

        let tracker_dir = root.join("tracker");
        fs::create_dir_all(&tracker_dir)?;
        let tracker_file = tracker_dir.join("one.torrent");
        write_torrent(&tracker_file)?;

        let nested_dir = tracker_dir.join("nested");
        fs::create_dir_all(&nested_dir)?;
        let nested_file = nested_dir.join("deep.torrent");
        write_torrent(&nested_file)?;

        let depth0 = scan_torrent_paths(root, 0)?;
        assert!(depth0.iter().any(|p| p.ends_with("root.torrent")));
        assert!(depth0.iter().any(|p| p.ends_with("one.torrent")));
        assert!(depth0.iter().any(|p| p.ends_with("deep.torrent")));

        let depth1 = scan_torrent_paths(root, 1)?;
        assert!(depth1.iter().any(|p| p.ends_with("root.torrent")));
        assert!(depth1.iter().any(|p| p.ends_with("one.torrent")));
        assert!(!depth1.iter().any(|p| p.ends_with("deep.torrent")));

        let depth2 = scan_torrent_paths(root, 2)?;
        assert!(depth2.iter().any(|p| p.ends_with("root.torrent")));
        assert!(depth2.iter().any(|p| p.ends_with("one.torrent")));
        assert!(depth2.iter().any(|p| p.ends_with("deep.torrent")));

        Ok(())
    }
}
