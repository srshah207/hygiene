//! Chromium- and Firefox-family browser cache discovery and cleanup.

use std::fs;
use std::path::{Path, PathBuf};

use crate::fsutil;
use crate::model::{BrowserRoots, Cleaner, Ctx};

/// Empties cache directories for the configured browser profiles.
pub struct BrowserCaches {
    roots: BrowserRoots,
}

impl BrowserCaches {
    pub fn new(roots: BrowserRoots) -> Self {
        Self { roots }
    }
}

impl Cleaner for BrowserCaches {
    fn scan(&self, _ctx: &Ctx) -> u64 {
        let mut total = 0u64;
        for user_data in &self.roots.chromium {
            total += chromium_cache_paths(user_data)
                .iter()
                .map(|p| fsutil::dir_size(p))
                .sum::<u64>();
        }
        total
            + firefox_cache_paths(&self.roots.firefox_profiles)
                .iter()
                .map(|p| fsutil::dir_size(p))
                .sum::<u64>()
    }

    fn clean(&self, _ctx: &Ctx) -> Result<u64, String> {
        let mut freed = 0u64;
        for user_data in &self.roots.chromium {
            for p in chromium_cache_paths(user_data) {
                freed += fsutil::clear_dir(&p);
            }
        }
        for p in firefox_cache_paths(&self.roots.firefox_profiles) {
            freed += fsutil::clear_dir(&p);
        }
        Ok(freed)
    }
}

/// Profile directories inside a Chromium `User Data` folder.
fn chromium_profile_dirs(user_data: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(user_data) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let is_profile = name == "Default"
                || name.starts_with("Profile ")
                || name == "Guest Profile"
                || name == "System Profile";
            if is_profile && e.path().is_dir() {
                out.push(e.path());
            }
        }
    }
    out
}

/// Cache directories inside a Firefox `Profiles` folder.
fn firefox_cache_paths(profiles_dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = fs::read_dir(profiles_dir) {
        for e in rd.flatten() {
            if e.path().is_dir() {
                out.push(e.path().join("cache2"));
            }
        }
    }
    out
}

/// All disposable cache paths inside a Chromium `User Data` folder.
fn chromium_cache_paths(user_data: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for profile in chromium_profile_dirs(user_data) {
        for rel in [
            "Cache",
            "Code Cache",
            "GPUCache",
            "ShaderCache",
            "GrShaderCache",
            "DawnGraphiteCache",
            "DawnWebGPUCache",
        ] {
            out.push(profile.join(rel));
        }
        for rel in ["CacheStorage", "ScriptCache"] {
            out.push(profile.join("Service Worker").join(rel));
        }
    }
    out
}
