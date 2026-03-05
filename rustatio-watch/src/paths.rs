use std::path::{Component, Path, PathBuf};

fn depth_for_path(path: &Path, is_dir: bool) -> u32 {
    let count =
        path.components().filter(|component| matches!(component, Component::Normal(_))).count()
            as u32;

    if is_dir {
        count
    } else {
        count.saturating_sub(1)
    }
}

pub fn relative_watch_path(watch_dir: &Path, path: &Path) -> Result<PathBuf, String> {
    if path.is_relative() {
        return Ok(path.to_path_buf());
    }

    if let Ok(relative) = path.strip_prefix(watch_dir) {
        return Ok(relative.to_path_buf());
    }

    let canonical_watch = watch_dir.canonicalize().map_err(|_| "Invalid file path".to_string())?;
    let canonical_path = path.canonicalize().map_err(|_| "Invalid file path".to_string())?;

    canonical_path
        .strip_prefix(&canonical_watch)
        .map(Path::to_path_buf)
        .map_err(|_| "Invalid file path".to_string())
}

pub fn is_within_depth(watch_dir: &Path, path: &Path, max_depth: u32, is_dir: bool) -> bool {
    if max_depth == 0 {
        return true;
    }

    relative_watch_path(watch_dir, path)
        .is_ok_and(|relative| depth_for_path(&relative, is_dir) <= max_depth)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depth_for_path_file() {
        let path = PathBuf::from("tracker/file.torrent");
        assert_eq!(depth_for_path(&path, false), 1);
    }

    #[test]
    fn test_depth_for_path_dir() {
        let path = PathBuf::from("tracker");
        assert_eq!(depth_for_path(&path, true), 1);
    }
}
