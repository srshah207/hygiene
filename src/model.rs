//! Core data model: runtime context, deletable entries, targets, and the
//! [`Cleaner`] strategy trait that every cleanup category implements.

use std::path::PathBuf;

use crate::fsutil;

/// A detected WSL distribution; `home` is the default user's home directory
/// (`None` when that could not be determined).
pub struct WslDistro {
    pub name: String,
    pub home: Option<String>,
}

/// Per-run runtime context resolved once at startup.
pub struct Ctx {
    pub temp: PathBuf,
    pub localappdata: PathBuf,
    pub profile: PathBuf,
    pub elevated: bool,
    pub wsl: Vec<WslDistro>,
}

/// Browser profile roots used by the browser-cache cleaner.
pub struct BrowserRoots {
    pub chromium: Vec<PathBuf>,
    pub firefox_profiles: PathBuf,
}

/// One deletable thing: a directory to empty, a single file, or a
/// TrustedInstaller-owned directory (takeown/icacls first).
pub enum Entry {
    Dir(PathBuf),
    File(PathBuf),
    Owned(PathBuf),
}

impl Entry {
    /// Size on disk before cleaning.
    pub fn size(&self) -> u64 {
        match self {
            Entry::Dir(p) | Entry::Owned(p) => fsutil::dir_size(p),
            Entry::File(p) => fsutil::file_size(p),
        }
    }

    /// Delete this entry (best effort); returns bytes freed.
    pub fn clean(&self) -> u64 {
        match self {
            Entry::Dir(p) => fsutil::clear_dir(p),
            Entry::Owned(p) => fsutil::remove_owned_dir(p),
            Entry::File(p) => fsutil::remove_file_tracked(p),
        }
    }
}

/// Strategy for one category of junk: measure it, then remove it.
pub trait Cleaner {
    /// Estimated reclaimable bytes; never mutates anything.
    fn scan(&self, ctx: &Ctx) -> u64;

    /// Deletes junk and returns bytes actually freed. `Err` carries a
    /// user-facing message (shown verbatim).
    fn clean(&self, ctx: &Ctx) -> Result<u64, String>;
}

/// A user-visible row in the hygiene list, backed by a [`Cleaner`].
pub struct Target {
    pub label: String,
    pub desc: &'static str,
    pub admin: bool,
    pub cleaner: Box<dyn Cleaner>,
}

impl Target {
    pub fn new(
        label: impl Into<String>,
        desc: &'static str,
        admin: bool,
        cleaner: impl Cleaner + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            desc,
            admin,
            cleaner: Box::new(cleaner),
        }
    }

    /// Estimated reclaimable bytes.
    pub fn scan(&self, ctx: &Ctx) -> u64 {
        self.cleaner.scan(ctx)
    }

    /// Clean this target; enforces the elevation requirement in one place.
    pub fn clean(&self, ctx: &Ctx) -> Result<u64, String> {
        if self.admin && !ctx.elevated {
            return Err("needs admin - re-run hygiene from an elevated terminal".into());
        }
        self.cleaner.clean(ctx)
    }
}
