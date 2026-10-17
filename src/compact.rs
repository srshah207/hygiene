//! Disk-image compaction: `hygiene compact-docker` and `hygiene compact-wsl`.
//!
//! WSL/Docker VHDX files grow on use but never shrink by themselves. These
//! flows trim filesystems, stop the VM, run an elevated `diskpart`
//! compaction, and restore whatever was running.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::platform;
use crate::ui::{fmt_size, Ui};

/// Shrink the Docker Desktop disk image.
pub fn docker(ui: &Ui) {
    let Some(vhdx) = find_docker_vhdx() else {
        println!(
            "{}",
            ui.red("no Docker Desktop disk image found (is Docker Desktop installed?)")
        );
        return;
    };
    let before = fs::metadata(&vhdx).map(|m| m.len()).unwrap_or(0);

    println!("{}", ui.bold("Docker disk compaction"));
    println!("  image: {}", vhdx.display());
    println!("  size:  {}", fmt_size(before));

    let Some(dir) = platform::helper_dir() else {
        println!("{}", ui.red("could not create helper dir"));
        return;
    };
    let dp_path = dir.join("hygiene-diskpart.txt");
    let ps1_path = dir.join("hygiene-compact.ps1");

    let docker_running = platform::is_process_running("Docker Desktop.exe");
    let restart_cmd = if docker_running {
        "$dd = Join-Path $env:ProgramFiles 'Docker\\Docker\\Docker Desktop.exe'\n\
         if (Test-Path $dd) { Start-Process $dd } else { try { & docker desktop start | Out-Null } catch {} }\n"
    } else {
        ""
    };

    let dp_content = format!("select vdisk file=\"{}\"\ncompact vdisk\n", vhdx.display());
    let ps1_content = format!(
        "Start-Service vds -ErrorAction SilentlyContinue\n\
         Start-Sleep 2\n\
         Get-Process 'Docker Desktop','com.docker.backend','com.docker.build' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue\n\
         wsl --shutdown\n\
         Start-Sleep 5\n\
         diskpart /s \"{dp}\"\n\
         {restart}",
        dp = dp_path.display(),
        restart = restart_cmd
    );
    if fs::write(&dp_path, dp_content).is_err() || fs::write(&ps1_path, ps1_content).is_err() {
        println!("{}", ui.red("could not write helper scripts"));
        return;
    }

    if docker_running {
        println!("  {}", ui.dim("stopping Docker gracefully..."));
        let _ = Command::new("docker").args(["desktop", "stop"]).output();
    }
    println!(
        "  {}",
        ui.yellow("a UAC prompt will appear - approve it to compact the disk")
    );
    if platform::run_elevated_file(&ps1_path) {
        let after = fs::metadata(&vhdx).map(|m| m.len()).unwrap_or(0);
        println!(
            "  {} {} -> {}",
            ui.green("done:"),
            fmt_size(before),
            fmt_size(after)
        );
        if docker_running {
            println!("  {}", ui.dim("Docker Desktop is starting back up."));
        }
    } else {
        println!(
            "  {}",
            ui.yellow("elevated compaction was cancelled or failed")
        );
    }
}

/// Find the Docker Desktop disk image; layouts vary between versions.
fn find_docker_vhdx() -> Option<PathBuf> {
    let local = env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_default();
    let base = local.join("Docker").join("wsl");
    let default = base.join("disk").join("docker_data.vhdx");
    if default.exists() {
        return Some(default);
    }
    // Scan a few levels down; prefer docker_data*.vhdx candidates.
    let mut candidates: Vec<PathBuf> = Vec::new();
    let mut stack = vec![(base, 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        if depth > 4 {
            continue;
        }
        if let Ok(rd) = fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push((p, depth + 1));
                } else if p
                    .extension()
                    .map(|x| x.eq_ignore_ascii_case("vhdx"))
                    .unwrap_or(false)
                {
                    candidates.push(p);
                }
            }
        }
    }
    candidates.sort_by_key(|p| {
        let n = p
            .file_name()
            .map(|x| x.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let pri = if n.contains("docker_data") { 0 } else { 1 };
        (pri, n)
    });
    candidates.into_iter().next()
}

/// Trim filesystems inside every WSL distro, then shrink their disk images.
pub fn wsl(ui: &Ui) {
    println!("{}", ui.bold("WSL disk compaction"));

    let script = "Get-ChildItem 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Lxss' -ErrorAction SilentlyContinue | ForEach-Object { $p = Get-ItemProperty $_.PSPath; '{0}|{1}' -f $p.DistributionName, $p.BasePath }";
    let mut distros: Vec<(String, PathBuf, u64)> = Vec::new();
    if let Some(bytes) = platform::run_capture("powershell", &["-NoProfile", "-Command", script]) {
        for line in platform::decode_output(&bytes).lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Some((name, base)) = line.split_once('|') else {
                continue;
            };
            let name = name.trim().to_string();
            if name.is_empty() || name.to_lowercase().contains("docker-desktop") {
                continue;
            }
            let base = base.trim().trim_end_matches('\\');
            if base.is_empty() {
                continue;
            }
            let vhdx = PathBuf::from(format!("{base}\\ext4.vhdx"));
            if vhdx.exists() {
                let size = fs::metadata(&vhdx).map(|m| m.len()).unwrap_or(0);
                distros.push((name, vhdx, size));
            }
        }
    }

    // Fallback: scan %LOCALAPPDATA%\Packages for store-installed distros.
    if distros.is_empty() {
        let pkgs = PathBuf::from(env::var("LOCALAPPDATA").unwrap_or_default()).join("Packages");
        if let Ok(rd) = fs::read_dir(&pkgs) {
            for e in rd.flatten() {
                let vhdx = e.path().join("LocalState").join("ext4.vhdx");
                if vhdx.exists() {
                    let name = e.file_name().to_string_lossy().into_owned();
                    let size = fs::metadata(&vhdx).map(|m| m.len()).unwrap_or(0);
                    distros.push((name, vhdx, size));
                }
            }
        }
    }

    if distros.is_empty() {
        println!("  {}", ui.yellow("no WSL distro disk images found"));
        return;
    }
    for (name, vhdx, size) in &distros {
        println!("  {name:<24} {:>9}  {}", fmt_size(*size), vhdx.display());
    }

    println!(
        "  {}",
        ui.dim("trimming free space inside WSL (fstrim as root, no password needed)...")
    );
    for (name, _, _) in &distros {
        let _ = platform::run_capture(
            "wsl",
            &["-d", name.as_str(), "-u", "root", "--", "fstrim", "-v", "/"],
        );
    }

    let docker_running = platform::is_process_running("Docker Desktop.exe");

    let Some(dir) = platform::helper_dir() else {
        println!("{}", ui.red("could not create helper dir"));
        return;
    };
    let dp_path = dir.join("hygiene-diskpart-wsl.txt");
    let ps1_path = dir.join("hygiene-compact-wsl.ps1");

    let mut dp_text = String::new();
    for (_, vhdx, _) in &distros {
        dp_text.push_str(&format!(
            "select vdisk file=\"{}\"\ncompact vdisk\n",
            vhdx.display()
        ));
    }
    let ps1 = format!(
        "Start-Service vds -ErrorAction SilentlyContinue\n\
         Start-Sleep 2\n\
         wsl --shutdown\n\
         Start-Sleep 5\n\
         diskpart /s \"{dp}\"\n",
        dp = dp_path.display()
    );
    if fs::write(&dp_path, dp_text).is_err() || fs::write(&ps1_path, ps1).is_err() {
        println!("{}", ui.red("could not write helper scripts"));
        return;
    }

    println!(
        "  {}",
        ui.yellow("a UAC prompt will appear - approve it to compact the disk(s)")
    );
    if platform::run_elevated_file(&ps1_path) {
        for (name, vhdx, before) in &distros {
            let after = fs::metadata(vhdx).map(|m| m.len()).unwrap_or(0);
            println!(
                "  {} {name}: {} -> {}",
                ui.green("done:"),
                fmt_size(*before),
                fmt_size(after)
            );
        }
        if docker_running {
            let _ = Command::new("docker").args(["desktop", "start"]).output();
            println!("  {}", ui.dim("Docker Desktop is starting back up."));
        }
        println!("  {}", ui.dim("WSL restarts on next use."));
    } else {
        println!(
            "  {}",
            ui.yellow("elevated compaction was cancelled or failed")
        );
    }
}
