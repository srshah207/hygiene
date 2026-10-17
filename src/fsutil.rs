//! Best-effort filesystem measurement and deletion helpers.
//!
//! Everything here is silently forgiving: locked or missing files are
//! skipped instead of failing the whole operation.

use std::fs;
use std::path::Path;
use std::process::Command;

/// Recursively sum file sizes (symlinks are not followed).
pub fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(p) = stack.pop() {
        let rd = match fs::read_dir(&p) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for e in rd.flatten() {
            let meta = match fs::symlink_metadata(e.path()) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.file_type().is_symlink() {
                continue;
            }
            if meta.is_dir() {
                stack.push(e.path());
            } else {
                total += meta.len();
            }
        }
    }
    total
}

/// Delete everything inside `path` (best effort). Returns bytes freed.
pub fn clear_dir(path: &Path) -> u64 {
    if !path.is_dir() {
        return 0;
    }
    let before = dir_size(path);
    if let Ok(rd) = fs::read_dir(path) {
        for e in rd.flatten() {
            let p = e.path();
            let meta = match fs::symlink_metadata(&p) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.is_dir() && !meta.file_type().is_symlink() {
                let _ = fs::remove_dir_all(&p);
            } else {
                let _ = fs::remove_file(&p);
            }
        }
    }
    let after = dir_size(path);
    before.saturating_sub(after)
}

/// Size of a single file, or 0 when missing.
pub fn file_size(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// Delete a single file; returns the size that was freed (0 if not removed).
pub fn remove_file_tracked(path: &Path) -> u64 {
    if !path.is_file() {
        return 0;
    }
    let size = file_size(path);
    if fs::remove_file(path).is_ok() {
        size
    } else {
        0
    }
}

/// TrustedInstaller-owned directory: take ownership, grant admins, remove.
pub fn remove_owned_dir(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    let before = dir_size(path);
    let p = path.to_string_lossy().into_owned();
    let _ = Command::new("takeown")
        .args(["/f", p.as_str(), "/r", "/d", "y"])
        .output();
    let _ = Command::new("icacls")
        .args([p.as_str(), "/grant", "*S-1-5-32-544:(F)", "/t", "/c", "/q"])
        .output();
    if fs::remove_dir_all(path).is_err() {
        let _ = clear_dir(path);
    }
    let after = dir_size(path);
    before.saturating_sub(after)
}
