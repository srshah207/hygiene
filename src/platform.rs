//! Thin wrappers over OS plumbing (processes, elevation, console helpers).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Run a program, capture stdout. Returns `None` when the program could not
/// be started (e.g. not installed) - callers treat that as "nothing found".
pub fn run_capture(program: &str, args: &[&str]) -> Option<Vec<u8>> {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .map(|o| o.stdout)
}

/// Decode captured output; `wsl.exe` sometimes emits UTF-16LE (nul bytes).
pub fn decode_output(bytes: &[u8]) -> String {
    if bytes.contains(&0) {
        let cleaned: Vec<u8> = bytes.iter().copied().filter(|b| *b != 0).collect();
        String::from_utf8_lossy(&cleaned).into_owned()
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

/// Whether the current process runs with an elevated token.
pub fn is_elevated() -> bool {
    run_capture("whoami", &["/groups"])
        .map(|b| {
            let t = decode_output(&b);
            t.contains("S-1-16-12288") || t.contains("High Mandatory Level")
        })
        .unwrap_or(false)
}

/// Whether a process with this image name is currently running.
pub fn is_process_running(exe: &str) -> bool {
    let filter = format!("IMAGENAME eq {exe}");
    run_capture("tasklist", &["/FI", filter.as_str(), "/NH"])
        .map(|b| decode_output(&b).contains(exe))
        .unwrap_or(false)
}

/// Run a `.ps1` file in a new elevated PowerShell, waiting for it to finish.
/// The user sees a UAC prompt; returning `false` means it was cancelled/failed.
pub fn run_elevated_file(ps1: &Path) -> bool {
    let cmd = format!(
        "Start-Process powershell -Verb RunAs -Wait -WindowStyle Hidden -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','{}'",
        ps1.display()
    );
    matches!(
        Command::new("powershell")
            .args(["-NoProfile", "-Command", cmd.as_str()])
            .status(),
        Ok(s) if s.success()
    )
}

/// A scratch directory for helper scripts (`%TEMP%\hygiene`).
pub fn helper_dir() -> Option<PathBuf> {
    let temp = env::var("TEMP").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(temp).join("hygiene");
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}
