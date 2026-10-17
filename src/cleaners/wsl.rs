//! WSL support: distro detection and cache cleanup across all distros.

use crate::model::{Cleaner, Ctx, WslDistro};
use crate::platform::{decode_output, run_capture};

/// Cleans caches inside every detected (non-Docker) WSL distro.
pub struct WslCachesCleaner;

impl Cleaner for WslCachesCleaner {
    fn scan(&self, ctx: &Ctx) -> u64 {
        scan(ctx)
    }

    fn clean(&self, ctx: &Ctx) -> Result<u64, String> {
        clean(ctx)
    }
}

/// All non-Docker distros, each with its default user's home directory
/// (`None` when that could not be determined - the distro is then only
/// used for root-level cleanup).
pub fn detect() -> Vec<WslDistro> {
    let Some(bytes) = run_capture("wsl", &["-l", "-q"]) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in decode_output(&bytes).lines() {
        let name = line.trim().trim_matches('\u{feff}').trim().to_string();
        if name.is_empty() || name.to_lowercase().contains("docker-desktop") {
            continue;
        }
        let home = run_capture("wsl", &["-d", name.as_str(), "--", "printenv", "HOME"])
            .map(|b| decode_output(&b).trim().to_string())
            .filter(|s| !s.is_empty());
        out.push(WslDistro { name, home });
    }
    out
}

/// Cache paths relative to a distro user's home directory.
const CACHE_RELS: &[&str] = &[
    ".cache/sccache",
    ".cache/pip",
    ".cache/pnpm",
    ".cache/uv",
    ".cache/node",
    ".cache/node-gyp",
    ".cache/d2",
    ".cache/rclone",
    ".cache/opencode",
    ".cache/antigravity",
    ".cache/immich-go",
    ".cache/node-systray",
    ".cache/huggingface",
    ".cache/torch",
    ".cache/whisper",
    ".npm/_cacache",
    ".gradle/caches",
];

fn cache_paths(home: &str) -> Vec<String> {
    CACHE_RELS.iter().map(|r| format!("{home}/{r}")).collect()
}

fn du_bytes(out: Option<Vec<u8>>) -> u64 {
    out.map(|b| {
        decode_output(&b)
            .lines()
            .filter_map(|l| l.split_whitespace().next())
            .filter_map(|n| n.parse::<u64>().ok())
            .sum::<u64>()
    })
    .unwrap_or(0)
        * 1024
        * 1024
}

fn scan(ctx: &Ctx) -> u64 {
    let mut total = 0u64;
    for d in &ctx.wsl {
        if let Some(home) = &d.home {
            let paths = cache_paths(home);
            let mut args: Vec<&str> = vec!["-d", d.name.as_str(), "--", "du", "-sm"];
            args.extend(paths.iter().map(|s| s.as_str()));
            total += du_bytes(run_capture("wsl", &args));
        }
        total += du_bytes(run_capture(
            "wsl",
            &[
                "-d",
                d.name.as_str(),
                "-u",
                "root",
                "--",
                "du",
                "-sm",
                "/var/cache/apt/archives",
            ],
        ));
    }
    total
}

fn clean(ctx: &Ctx) -> Result<u64, String> {
    if ctx.wsl.is_empty() {
        return Err("no WSL distro found".into());
    }
    let before = scan(ctx);
    for d in &ctx.wsl {
        let _ = run_capture(
            "wsl",
            &["-d", d.name.as_str(), "--", "sccache", "--stop-server"],
        );
        let _ = run_capture(
            "wsl",
            &["-d", d.name.as_str(), "--", "pkill", "-x", "sccache"],
        );
        if let Some(home) = &d.home {
            let paths = cache_paths(home);
            let mut args: Vec<&str> = vec!["-d", d.name.as_str(), "--", "rm", "-rf"];
            args.extend(paths.iter().map(|s| s.as_str()));
            run_capture("wsl", &args);
        }
        let _ = run_capture(
            "wsl",
            &[
                "-d",
                d.name.as_str(),
                "-u",
                "root",
                "--",
                "apt-get",
                "clean",
            ],
        );
    }
    let after = scan(ctx);
    Ok(before.saturating_sub(after))
}
