<p align="center">
  <img src="assets/icon.svg" alt="hygiene" width="96">
</p>

<h1 align="center">hygiene</h1>

<p align="center">
  <b>The dev-machine cleaner for Windows + WSL.</b><br>
  One binary, zero dependencies, fully offline, no telemetry — it finds the
  gigabytes your caches quietly accumulate and gives you a safe, reviewed
  list to reclaim.
</p>

<p align="center">
  <a href="https://github.com/srshah207/hygiene/actions/workflows/ci.yml"><img src="https://github.com/srshah207/hygiene/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/srshah207/hygiene/releases"><img src="https://img.shields.io/github/v/release/srshah207/hygiene" alt="Latest release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"></a>
</p>

```text
hygiene v0.4.0 - cleaner for Windows + WSL dev machines
scanning... (WSL sizes take a few seconds)

   1.  Windows Temp                            413.1 MB  user temp files (safe)
   2.  Developer package caches                   1.2 GB  npm, yarn, pnpm, pip, uv, cargo, go, gradle...
   3.  Browser caches                             1.4 GB  Chrome/Edge/Brave/Firefox - close browsers first
   ...
  12.  WSL dev caches                             1.0 MB  sccache, pip, Gradle, HF, apt archives (WSL)
  16.  Post-upgrade leftovers (needs admin)      2.8 GB  WinREAgent, Windows.old, upgrade/driver leftovers
  17.  WinSxS cleanup (DISM) (needs admin)               DISM component store; takes several minutes

  Total reclaimable:  4.21 GB
```

## Why

Developer machines leak disk space in places Explorer doesn't tell you about:
package-manager caches, shader caches, crash dumps, WSL distro caches, and
Windows servicing data. Generic cleaners either miss the dev-specific caches
or take liberties with your data.

`hygiene` is a small, auditable tool with one rule: **every path it deletes is
an explicit allowlist entry, reviewed in the source you are running.**

## Install

### Download

1. Grab `hygiene-windows-x86_64.exe` from [Releases](../../releases).
2. Rename it to `hygiene.exe` and put it on your `PATH`.

### Build from source

Requires Rust 1.70+ (build-time only).

```powershell
cargo build --release        # → target\release\hygiene.exe
```

Cross-compiling from Linux/WSL:

```bash
# with system mingw-w64
sudo apt install mingw-w64
rustup target add x86_64-pc-windows-gnu
./build.sh

# no-sudo fallback: see the header of build.sh (uses rust-lld + HYGIENE_W64)
```

## Usage

```text
hygiene                     scan, pick items interactively, clean
hygiene --list              show sizes, clean nothing
hygiene --all --yes         clean all non-admin items without prompts
hygiene --only 2,5 --yes    clean specific items (ranges ok: 3-6)
hygiene --all --dry-run     preview exactly what --all would clean
hygiene compact-docker      shrink the Docker disk image (one UAC prompt)
hygiene compact-wsl         trim + shrink all WSL distro disks (one UAC prompt)
```

Items 14–17 touch system locations and require an elevated terminal; when run
without admin they are clearly marked and skipped. Exit code is `0` on
success, `2` on usage errors.

## What it cleans

| # | Item | Scope |
|---|------|-------|
| 1 | Windows Temp | user temp files |
| 2 | Dev package caches | npm, yarn, pnpm, bun, pip, uv, poetry, conda, NuGet, cargo registry, Go build/mod, Gradle, Pub, Cypress, Electron, node-gyp, Puppeteer |
| 3 | Browser caches | Chrome, Edge, Brave, Chromium, Vivaldi, Firefox (all profiles) |
| 4 | Editor / IDE caches | VS Code, Cursor, VSCodium, Windsurf, JetBrains caches & logs |
| 5 | App caches | Discord, Slack, Spotify, Teams, Microsoft Store, OneDrive logs, Steam htmlcache |
| 6 | GPU shader caches | NVIDIA, AMD, Intel, D3D (they rebuild automatically) |
| 7 | Thumbnail / icon caches | Explorer thumbcache/iconcache + cache dirs |
| 8 | Crash dumps & reports | user CrashDumps, WER |
| 9 | Codex runtimes | Codex CLI runtime cache |
| 10 | HuggingFace & AI caches | HF, torch, whisper, keras (re-downloadable) |
| 11 | Docker cleanup | stopped containers, dangling images, build cache, unused networks |
| 12 | WSL dev caches | sccache, pip, pnpm, uv, npm, Gradle, HF, torch, apt archives — **all distros** |
| 13 | Recycle Bin | empties it |
| 14 | Windows Update cache | `SoftwareDistribution` + Delivery Optimization (**admin**) |
| 15 | System temp, logs, dumps | `Windows\Temp`, Logs, Panther, Prefetch, Minidump, WER (**admin**) |
| 16 | Post-upgrade leftovers | `$WinREAgent`, `Windows.old`, `$GetCurrent`, upgrade/driver leftovers (**admin**) |
| 17 | WinSxS cleanup (DISM) | `Dism /StartComponentCleanup` without `/ResetBase` (**admin**) |

Plus **compaction**: `hygiene compact-docker` and `hygiene compact-wsl` shrink
VHDX disk images (filesystem trim + `diskpart compact vdisk` under one UAC),
and Docker Desktop is restarted afterwards only if it was running.

## Safety model

- **Allowlist only** — every deletion target is hardcoded; nothing is
  discovered by wildcard at delete time.
- **Symlinks are never followed or deleted** during scan or clean.
- **Locked files are skipped**, not forced (best-effort deletion).
- **`--dry-run`** shows the exact item list before anything is touched.
- **Admin items are gated** behind an elevated terminal; no self-elevation
  except the explicitly requested `compact-*` flows.
- **No network access, no telemetry.** The only external programs used are
  built-in Windows ones (`powershell`, `takeown`, `icacls`, `diskpart`,
  `dism`, `tasklist`, `whoami`) plus `wsl` and `docker` if you have them.

## What hygiene deliberately never touches

| Excluded | Reason |
|----------|--------|
| `hiberfil.sys` / `pagefile.sys` | Power the OS; manage via `powercfg`, never delete |
| System Restore points | Your recovery net for bad updates |
| Windows Event Logs | Required for diagnosing failures |
| `Windows\Installer` / deep WinSxS surgery | Breaks uninstall/repair; only the official DISM cleanup |
| Docker volumes | May contain databases (e.g. self-hosted services) |
| WSL `ms-playwright` and browser bundles | Re-download cost exceeds the space saved |
| `~/.m2`, global package stores | Re-download churn without meaningful savings |
| Your projects, documents, and personal files | Never. |

## Project layout

```text
src/
  main.rs       entry point: parse args, dispatch
  cli.rs        argument parsing + --help (unit-tested)
  session.rs    the default scan → select → clean flow
  catalog.rs    the item catalog: what hygiene may clean
  model.rs      shared types + the Cleaner strategy trait
  cleaners/     one strategy per category (browser, docker, wsl, windows)
  compact.rs    compact-docker / compact-wsl flows
  platform.rs   process, elevation, console helpers
  fsutil.rs     filesystem measure/delete helpers
  ui.rs         colors, sizes, terminal setup
```

## Development

```bash
cargo test                   # unit tests (parsing, selection, sizes)
cargo clippy --all-targets   # zero warnings
cargo fmt --check
```

CI builds and tests on every push; tagged releases (`v*`) publish a
checksummed `hygiene-windows-x86_64.exe` on GitHub Releases.

## Contributing

Issues and PRs are welcome — see [CONTRIBUTING.md](CONTRIBUTING.md). The one
hard rule: keep the safety model intact.

## License

[MIT](LICENSE)
