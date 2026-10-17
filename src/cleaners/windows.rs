//! Windows-specific cleaners: explorer caches, recycle bin, update cache,
//! and the DISM component-store cleanup.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::fsutil;
use crate::model::{Cleaner, Ctx};
use crate::platform;

/// Thumbnail/icon caches (Explorer DB files plus cache dirs).
pub struct ExplorerCaches {
    db_dir: PathBuf,
    extra: Vec<PathBuf>,
}

impl ExplorerCaches {
    pub fn new(db_dir: PathBuf, extra: Vec<PathBuf>) -> Self {
        Self { db_dir, extra }
    }
}

impl Cleaner for ExplorerCaches {
    fn scan(&self, _ctx: &Ctx) -> u64 {
        explorer_scan(&self.db_dir, &self.extra)
    }

    fn clean(&self, _ctx: &Ctx) -> Result<u64, String> {
        Ok(explorer_clean(&self.db_dir, &self.extra))
    }
}

/// Empties the user's recycle bin via the Shell COM object.
pub struct RecycleBin;

impl Cleaner for RecycleBin {
    fn scan(&self, _ctx: &Ctx) -> u64 {
        recycle_scan()
    }

    fn clean(&self, _ctx: &Ctx) -> Result<u64, String> {
        recycle_clean()
    }
}

/// Windows Update downloads + Delivery Optimization cache.
pub struct WindowsUpdateCache {
    paths: Vec<PathBuf>,
}

impl WindowsUpdateCache {
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self { paths }
    }
}

impl Cleaner for WindowsUpdateCache {
    fn scan(&self, _ctx: &Ctx) -> u64 {
        self.paths.iter().map(|p| fsutil::dir_size(p)).sum()
    }

    fn clean(&self, _ctx: &Ctx) -> Result<u64, String> {
        winupdate_clean(&self.paths)
    }
}

/// DISM component-store cleanup; the freed size is only known afterwards.
pub struct DismComponentStore;

impl Cleaner for DismComponentStore {
    fn scan(&self, _ctx: &Ctx) -> u64 {
        0
    }

    fn clean(&self, _ctx: &Ctx) -> Result<u64, String> {
        dism_clean()
    }
}

fn explorer_scan(db_dir: &Path, extra: &[PathBuf]) -> u64 {
    let mut total = 0u64;
    if let Ok(rd) = fs::read_dir(db_dir) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_lowercase();
            if n.starts_with("thumbcache") || n.starts_with("iconcache") {
                if let Ok(m) = fs::metadata(e.path()) {
                    if m.is_file() {
                        total += m.len();
                    }
                }
            }
        }
    }
    for dir in extra {
        total += fsutil::dir_size(dir);
    }
    total
}

fn explorer_clean(db_dir: &Path, extra: &[PathBuf]) -> u64 {
    let mut freed = 0u64;
    if let Ok(rd) = fs::read_dir(db_dir) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_lowercase();
            if n.starts_with("thumbcache") || n.starts_with("iconcache") {
                if let Ok(m) = fs::metadata(e.path()) {
                    if m.is_file() && fs::remove_file(e.path()).is_ok() {
                        freed += m.len();
                    }
                }
            }
        }
    }
    for dir in extra {
        freed += fsutil::clear_dir(dir);
    }
    freed
}

fn recycle_scan() -> u64 {
    let script = "$s=(New-Object -ComObject Shell.Application).NameSpace(10).Items() | ForEach-Object {$_.Size} | Measure-Object -Sum; [int64]$s.Sum";
    platform::run_capture("powershell", &["-NoProfile", "-Command", script])
        .map(|b| {
            platform::decode_output(&b)
                .trim()
                .parse::<u64>()
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

fn recycle_clean() -> Result<u64, String> {
    let before = recycle_scan();
    Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Clear-RecycleBin -Force -ErrorAction SilentlyContinue",
        ])
        .output()
        .map_err(|e| e.to_string())?;
    let after = recycle_scan();
    Ok(before.saturating_sub(after))
}

fn winupdate_clean(paths: &[PathBuf]) -> Result<u64, String> {
    let before: u64 = paths.iter().map(|p| fsutil::dir_size(p)).sum();
    let list: Vec<String> = paths.iter().map(|p| format!("'{}'", p.display())).collect();
    let script = format!(
        "foreach ($s in 'wuauserv','bits','dosvc') {{ Stop-Service $s -Force -ErrorAction SilentlyContinue }}; \
         foreach ($p in @({})) {{ Remove-Item ($p + '\\*') -Recurse -Force -ErrorAction SilentlyContinue }}; \
         foreach ($s in 'wuauserv','bits','dosvc') {{ Start-Service $s -ErrorAction SilentlyContinue }}",
        list.join(",")
    );
    let _ = Command::new("powershell")
        .args(["-NoProfile", "-Command", script.as_str()])
        .output();
    let after: u64 = paths.iter().map(|p| fsutil::dir_size(p)).sum();
    Ok(before.saturating_sub(after))
}

fn dism_clean() -> Result<u64, String> {
    print!("running DISM /StartComponentCleanup (5-15 min, no output until done)... ");
    let _ = io::stdout().flush();
    let out = Command::new("Dism.exe")
        .args(["/Online", "/Cleanup-Image", "/StartComponentCleanup"])
        .output()
        .map_err(|e| format!("could not run dism: {e}"))?;
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    if out.status.success() {
        Ok(0)
    } else if text.contains("0x800f0806") || text.to_lowercase().contains("pending") {
        Err("pending servicing operations - reboot Windows first, then retry".into())
    } else {
        let last = text
            .lines()
            .rfind(|l| !l.trim().is_empty())
            .unwrap_or("")
            .trim()
            .to_string();
        Err(format!("DISM failed: {last}"))
    }
}
