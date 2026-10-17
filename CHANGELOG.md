# Changelog

All notable changes to this project are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/); versioning is
[SemVer](https://semver.org/). The CLI flag surface freezes at 1.0.

## [0.4.1] - 2026-10-17

### Added

- Full CI/CD automation: weekly scheduled CI runs, MSRV (1.70) validation,
  Dependabot dependency updates with auto-merge, automatic version tagging on
  `main` (`vX.Y.Z` created from `Cargo.toml`), release dispatch, and a smoke
  test of the packaged release binary.

### Changed

- Windows CI now runs the test suite natively before the release build.

## [0.4.0] - 2026-10-17

### Changed

- **WSL cleanup now covers every distro**, not just the first one found
  (per-distro home detection; aggregate scan; distros without a detectable
  home still get root-level cleanup).
- Docker disk-image discovery tolerates different Docker Desktop layouts
  (default path first, bounded scan with `docker_data*` preference).
- `compact-docker` / `compact-wsl` restart Docker Desktop only if it was
  running beforehand, and locate it via the standard install path with a
  `docker desktop start` fallback.

### Added

- Unit tests: CLI parsing, item-selection grammar, size formatting, docker
  size parsing (9 tests).
- GitHub Actions CI (fmt + clippy + tests on Linux, release build on
  Windows) and a tag-driven release workflow with SHA256 checksums.
- README safety model, exclusion rationale, project layout; MIT LICENSE;
  CONTRIBUTING guide.

### Internal

- Rewritten as a module tree with a `Cleaner` strategy trait; `main` is a
  thin dispatcher, the catalog is data, and each category owns its logic
  (zero clippy warnings).

## [0.3.0] - 2026-10-16

### Added

- Full catalog: 13 user items + 4 admin items (Windows Update cache, system
  temp/logs/dumps, post-upgrade leftovers, WinSxS/DISM cleanup).
- `--only` flag with lists and ranges (e.g. `--only 2,5` / `--only 3-6`).

## [0.2.0] - 2026-10-16

### Added

- Extended browser caches: Chrome, Edge, Brave, Chromium, Vivaldi, Firefox.
- `compact-wsl`: fstrim + diskpart compaction for registry-detected WSL
  distros, with Docker restart if it was running.

## [0.1.0] - 2026-10-16

### Added

- Initial release: temp files, dev package caches, IDE caches, app caches,
  Docker prune (never volumes), WSL cache cleanup, `compact-docker`.
