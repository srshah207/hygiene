//! Docker CLI helpers: estimate reclaimable space, then prune safely.
//!
//! Only stopped containers, dangling images, the build cache and unused
//! networks are pruned - never volumes or tagged images (they may hold
//! user data or slow-to-rebuild content).

use std::process::Command;

use crate::model::{Cleaner, Ctx};
use crate::platform::{decode_output, run_capture};

/// Prunes stopped containers, dangling images, build cache, unused networks.
pub struct DockerPruneCleaner;

impl Cleaner for DockerPruneCleaner {
    fn scan(&self, _ctx: &Ctx) -> u64 {
        reclaimable()
    }

    fn clean(&self, _ctx: &Ctx) -> Result<u64, String> {
        prune()
    }
}

/// Sum the reclaimable bytes reported by `docker system df`.
fn reclaimable() -> u64 {
    let Some(bytes) = run_capture(
        "docker",
        &["system", "df", "--format", "{{.Type}}|{{.Reclaimable}}"],
    ) else {
        return 0;
    };
    let mut total = 0u64;
    for line in decode_output(&bytes).lines() {
        if let Some((ty, rec)) = line.trim().split_once('|') {
            if ty.starts_with("Containers") || ty.starts_with("Build Cache") {
                total += parse_docker_size(rec);
            }
        }
    }
    total
}

/// Run the safe prune set; returns bytes freed according to docker itself.
fn prune() -> Result<u64, String> {
    match Command::new("docker")
        .args(["info", "--format", "{{.ServerVersion}}"])
        .output()
    {
        Ok(o) if o.status.success() => {}
        _ => return Err("docker daemon is not running - start Docker Desktop first".into()),
    }
    let mut freed = 0u64;
    for prune_args in [
        ["container", "prune", "-f"],
        ["image", "prune", "-f"],
        ["builder", "prune", "-f"],
        ["network", "prune", "-f"],
    ] {
        if let Ok(o) = Command::new("docker").args(prune_args).output() {
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                if let Some(rest) = line.trim().strip_prefix("Total reclaimed space:") {
                    freed += parse_docker_size(rest);
                }
            }
        }
    }
    Ok(freed)
}

/// Parse docker sizes like `1.234GB` or `0B` (tolerates `(12%)` suffixes).
fn parse_docker_size(s: &str) -> u64 {
    let tok = s.split_whitespace().next().unwrap_or("");
    let idx = tok
        .find(|c: char| c.is_ascii_alphabetic())
        .unwrap_or(tok.len());
    let (num, unit) = tok.split_at(idx);
    let n: f64 = num.parse().unwrap_or(0.0);
    let mult = match unit.to_ascii_lowercase().as_str() {
        "" | "b" => 1.0,
        "kb" => 1e3,
        "mb" => 1e6,
        "gb" => 1e9,
        "tb" => 1e12,
        _ => 0.0,
    };
    (n * mult) as u64
}

#[cfg(test)]
mod tests {
    use super::parse_docker_size;

    #[test]
    fn parses_docker_sizes() {
        assert_eq!(parse_docker_size("1.234GB (12%)"), 1_234_000_000);
        assert_eq!(parse_docker_size("0B"), 0);
        assert_eq!(parse_docker_size("512kB"), 512_000);
        assert_eq!(parse_docker_size("garbage"), 0);
    }
}
