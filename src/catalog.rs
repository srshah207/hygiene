//! The target catalog: the single place that declares *what* hygiene cleans.
//!
//! Items 1-13 need no admin; items 14-17 require an elevated terminal.
//! Every path here is an explicit allowlist entry - nothing is guessed
//! or pattern-deleted at runtime.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cleaners::browser::BrowserCaches;
use crate::cleaners::docker::DockerPruneCleaner;
use crate::cleaners::windows::{
    DismComponentStore, ExplorerCaches, RecycleBin, WindowsUpdateCache,
};
use crate::cleaners::wsl::WslCachesCleaner;
use crate::cleaners::{DirCleaner, EntriesCleaner};
use crate::model::{BrowserRoots, Ctx, Entry, Target};

/// Build the full catalog (user items first, then admin items).
pub fn build(ctx: &Ctx) -> Vec<Target> {
    let mut targets = user_targets(ctx);
    targets.extend(admin_targets());
    targets
}

fn user_targets(ctx: &Ctx) -> Vec<Target> {
    let lad = &ctx.localappdata;
    let profile = &ctx.profile;
    let roaming = env::var("APPDATA").map(PathBuf::from).unwrap_or_default();
    let programdata =
        PathBuf::from(env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".into()));

    vec![
        Target::new(
            "Windows Temp",
            "user temp files (safe)",
            false,
            DirCleaner::new(ctx.temp.clone()),
        ),
        Target::new(
            "Dev package caches",
            "npm, yarn, pnpm, pip, uv, cargo, go, gradle...",
            false,
            EntriesCleaner::new(dev_cache_entries(lad, &roaming, profile)),
        ),
        Target::new(
            "Browser caches",
            "Chrome/Edge/Brave/Firefox - close browsers first",
            false,
            BrowserCaches::new(BrowserRoots {
                chromium: vec![
                    lad.join("Google").join("Chrome").join("User Data"),
                    lad.join("Microsoft").join("Edge").join("User Data"),
                    lad.join("BraveSoftware")
                        .join("Brave-Browser")
                        .join("User Data"),
                    lad.join("Chromium").join("User Data"),
                    lad.join("Vivaldi").join("User Data"),
                ],
                firefox_profiles: roaming.join("Mozilla").join("Firefox").join("Profiles"),
            }),
        ),
        Target::new(
            "Editor / IDE caches",
            "VS Code, Cursor, JetBrains caches & logs",
            false,
            EntriesCleaner::new(editor_cache_entries(&roaming, lad)),
        ),
        Target::new(
            "App caches",
            "Discord, Slack, Spotify, Teams, Store, OneDrive",
            false,
            EntriesCleaner::new(app_cache_entries(lad, &roaming)),
        ),
        Target::new(
            "GPU shader caches",
            "NVIDIA/AMD/Intel/D3D caches (they rebuild)",
            false,
            EntriesCleaner::new(gpu_cache_entries(lad, &programdata)),
        ),
        Target::new(
            "Thumbnail / icon caches",
            "regenerate; restart explorer if locked",
            false,
            ExplorerCaches::new(
                lad.join("Microsoft").join("Windows").join("Explorer"),
                vec![lad.join("Microsoft").join("Windows").join("Caches")],
            ),
        ),
        Target::new(
            "Crash dumps & reports",
            "user crash dumps and error reports",
            false,
            EntriesCleaner::new(vec![
                Entry::Dir(lad.join("CrashDumps")),
                Entry::Dir(lad.join("Microsoft").join("Windows").join("WER")),
            ]),
        ),
        Target::new(
            "Codex runtimes",
            "Codex CLI runtime cache",
            false,
            DirCleaner::new(profile.join(".cache").join("codex-runtimes")),
        ),
        Target::new(
            "HuggingFace & AI caches",
            "HF, torch, whisper, keras (re-downloadable)",
            false,
            EntriesCleaner::new(vec![
                Entry::Dir(profile.join(".cache").join("huggingface")),
                Entry::Dir(profile.join(".cache").join("torch")),
                Entry::Dir(profile.join(".cache").join("whisper")),
                Entry::Dir(profile.join(".keras")),
            ]),
        ),
        Target::new(
            "Docker cleanup",
            "dangling images, build cache, stopped containers",
            false,
            DockerPruneCleaner,
        ),
        Target::new(
            "WSL dev caches",
            "sccache, pip, Gradle, HF, apt archives (WSL)",
            false,
            WslCachesCleaner,
        ),
        Target::new("Recycle Bin", "empties the recycle bin", false, RecycleBin),
    ]
}

fn dev_cache_entries(localappdata: &Path, roaming: &Path, profile: &Path) -> Vec<Entry> {
    let d = Entry::Dir;
    vec![
        d(localappdata.join("npm-cache")),
        d(roaming.join("npm-cache")),
        d(localappdata.join("Yarn").join("Cache")),
        d(localappdata.join("Yarn").join("Berry").join("cache")),
        d(localappdata.join("pnpm").join("store")),
        d(localappdata.join("pnpm-store")),
        d(profile.join(".bun").join("install").join("cache")),
        d(localappdata.join("pip").join("cache")),
        d(localappdata.join("uv").join("cache")),
        d(localappdata.join("pypoetry").join("Cache")),
        d(roaming.join("pypoetry").join("Cache")),
        d(profile.join(".conda").join("pkgs")),
        d(profile.join("anaconda3").join("pkgs")),
        d(profile.join("miniconda3").join("pkgs")),
        d(localappdata.join("NuGet").join("v3-cache")),
        d(localappdata.join("NuGet").join("plugins-cache")),
        d(profile.join(".nuget").join("packages")),
        d(localappdata.join("go-build")),
        d(profile.join("go").join("pkg").join("mod").join("cache")),
        d(profile.join(".cargo").join("registry").join("cache")),
        d(profile.join(".cargo").join("registry").join("src")),
        d(profile.join(".gradle").join("caches")),
        d(profile.join(".gradle").join("daemon")),
        d(localappdata.join("Pub").join("Cache")),
        d(localappdata.join("Cypress").join("Cache")),
        d(localappdata.join("electron").join("Cache")),
        d(localappdata.join("electron-builder").join("Cache")),
        d(localappdata.join("node-gyp").join("Cache")),
        d(profile.join(".cache").join("puppeteer")),
    ]
}

fn app_cache_entries(localappdata: &Path, roaming: &Path) -> Vec<Entry> {
    let d = Entry::Dir;
    vec![
        d(roaming.join("discord").join("Cache")),
        d(roaming.join("discord").join("Code Cache")),
        d(roaming.join("discord").join("GPUCache")),
        d(roaming.join("Slack").join("Cache")),
        d(roaming.join("Slack").join("Code Cache")),
        d(roaming.join("Slack").join("GPUCache")),
        d(roaming.join("Slack").join("logs")),
        d(roaming
            .join("Slack")
            .join("Service Worker")
            .join("CacheStorage")),
        d(localappdata.join("Spotify").join("Storage")),
        d(roaming.join("Microsoft").join("Teams").join("Cache")),
        d(roaming.join("Microsoft").join("Teams").join("Code Cache")),
        d(roaming.join("Microsoft").join("Teams").join("GPUCache")),
        d(localappdata
            .join("Packages")
            .join("Microsoft.WindowsStore_8wekyb3d8bbwe")
            .join("LocalCache")),
        d(localappdata.join("Microsoft").join("OneDrive").join("logs")),
        d(localappdata
            .join("Microsoft")
            .join("OneDrive")
            .join("setup")
            .join("logs")),
        d(localappdata.join("Steam").join("htmlcache")),
        d(localappdata.join("Steam").join("appcache")),
        d(localappdata
            .join("Microsoft")
            .join("Windows")
            .join("INetCache")),
    ]
}

fn gpu_cache_entries(localappdata: &Path, programdata: &Path) -> Vec<Entry> {
    let d = Entry::Dir;
    vec![
        d(localappdata.join("NVIDIA").join("DXCache")),
        d(localappdata.join("NVIDIA").join("GLCache")),
        d(localappdata.join("D3DSCache")),
        d(localappdata.join("AMD").join("DxCache")),
        d(localappdata.join("AMD").join("DxcCache")),
        d(localappdata.join("AMD").join("GLCache")),
        d(localappdata.join("Intel").join("ShaderCache")),
        d(programdata.join("NVIDIA Corporation").join("NV_Cache")),
    ]
}

fn editor_cache_entries(roaming: &Path, localappdata: &Path) -> Vec<Entry> {
    let mut out = Vec::new();
    for app in ["Code", "Cursor", "VSCodium", "Windsurf"] {
        let base = roaming.join(app);
        for sub in [
            "Cache",
            "CachedData",
            "Code Cache",
            "GPUCache",
            "logs",
            "Crashpad",
        ] {
            out.push(Entry::Dir(base.join(sub)));
        }
        out.push(Entry::Dir(base.join("Service Worker").join("CacheStorage")));
    }
    if let Ok(rd) = fs::read_dir(localappdata.join("JetBrains")) {
        for e in rd.flatten() {
            if e.path().is_dir() {
                out.push(Entry::Dir(e.path().join("caches")));
                out.push(Entry::Dir(e.path().join("log")));
            }
        }
    }
    out
}

fn admin_targets() -> Vec<Target> {
    let windir = PathBuf::from(env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into()));
    let programdata =
        PathBuf::from(env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".into()));

    vec![
        Target::new(
            "Windows Update cache",
            "update downloads + Delivery Optimization",
            true,
            WindowsUpdateCache::new(vec![
                windir.join("SoftwareDistribution").join("Download"),
                windir
                    .join("SoftwareDistribution")
                    .join("DeliveryOptimization"),
                windir
                    .join("ServiceProfiles")
                    .join("NetworkService")
                    .join("AppData")
                    .join("Local")
                    .join("Microsoft")
                    .join("Windows")
                    .join("DeliveryOptimization"),
            ]),
        ),
        Target::new(
            "System temp, logs, dumps",
            "Windows\\Temp, logs, Prefetch, dumps, WER",
            true,
            EntriesCleaner::new(vec![
                Entry::Dir(windir.join("Temp")),
                Entry::Dir(windir.join("Logs")),
                Entry::Dir(windir.join("Panther")),
                Entry::Dir(windir.join("Prefetch")),
                Entry::Dir(windir.join("Minidump")),
                Entry::Dir(windir.join("LiveKernelReports")),
                Entry::File(windir.join("MEMORY.DMP")),
                Entry::Dir(programdata.join("Microsoft").join("Windows").join("WER")),
            ]),
        ),
        Target::new(
            "Post-upgrade leftovers",
            "WinREAgent, Windows.old, upgrade/driver leftovers",
            true,
            EntriesCleaner::new(vec![
                Entry::Owned(PathBuf::from("C:\\$WinREAgent")),
                Entry::Owned(PathBuf::from("C:\\Windows.old")),
                Entry::Owned(PathBuf::from("C:\\$GetCurrent")),
                Entry::Owned(PathBuf::from("C:\\$Windows.~BT")),
                Entry::Owned(PathBuf::from("C:\\$Windows.~WS")),
                Entry::Owned(PathBuf::from("C:\\ESD")),
                Entry::Owned(PathBuf::from("C:\\Intel")),
                Entry::Owned(PathBuf::from("C:\\AMD")),
                Entry::Owned(PathBuf::from("C:\\NVIDIA")),
            ]),
        ),
        Target::new(
            "WinSxS cleanup (DISM)",
            "DISM component store; takes several minutes",
            true,
            DismComponentStore,
        ),
    ]
}
