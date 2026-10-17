//! Cleanup strategies. Each cleaner implements [`Cleaner`] (scan + clean)
//! for one category of junk; OS-specific ones live in submodules.

pub mod browser;
pub mod docker;
pub mod windows;
pub mod wsl;

use std::path::PathBuf;

use crate::fsutil;
use crate::model::{Cleaner, Ctx, Entry};

/// Empties the contents of one directory.
pub struct DirCleaner {
    path: PathBuf,
}

impl DirCleaner {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Cleaner for DirCleaner {
    fn scan(&self, _ctx: &Ctx) -> u64 {
        fsutil::dir_size(&self.path)
    }

    fn clean(&self, _ctx: &Ctx) -> Result<u64, String> {
        Ok(fsutil::clear_dir(&self.path))
    }
}

/// A curated list of dirs/files/owned dirs treated as one item.
pub struct EntriesCleaner {
    entries: Vec<Entry>,
}

impl EntriesCleaner {
    pub fn new(entries: Vec<Entry>) -> Self {
        Self { entries }
    }
}

impl Cleaner for EntriesCleaner {
    fn scan(&self, _ctx: &Ctx) -> u64 {
        self.entries.iter().map(Entry::size).sum()
    }

    fn clean(&self, _ctx: &Ctx) -> Result<u64, String> {
        Ok(self.entries.iter().map(Entry::clean).sum())
    }
}
