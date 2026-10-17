# Contributing

Thanks for helping make `hygiene` better.

## The one hard rule

**Keep the safety model intact.** Any PR that deletes outside the explicit
allowlist, follows symlinks, removes Docker volumes or tagged images, or adds
network/telemetry will be declined. The entire value of this tool is that a
user can audit what it deletes in a few minutes.

## Before submitting

```bash
cargo fmt
cargo clippy --all-targets   # must be warning-free
cargo test
```

## Reporting bugs

Include:

- `hygiene --list` output (add `--no-color`),
- Windows version (10/11, build number),
- whether WSL and Docker Desktop are installed,
- for `compact-*` issues: output of `wsl -l -v` and Docker Desktop version.

## Adding a new cleanup item

1. Add a `Cleaner` implementation (or reuse `DirCleaner`/`EntriesCleaner`).
2. Wire it into `src/catalog.rs` — that file is the auditable list.
3. Update the README table and this repo's CHANGELOG.
4. Explain in the PR why the target is *safe to delete without thinking*:
   what regenerates it, and what the worst case is if it's deleted.
