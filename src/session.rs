//! The default `hygiene` session: scan, print the table, select, clean, report.
//! This is the composition root that wires context, catalog and cleaners.

use std::env;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use crate::catalog;
use crate::cleaners::wsl;
use crate::cli::{parse_selection, CleanOptions};
use crate::model::Ctx;
use crate::platform;
use crate::ui::{fmt_size, Ui};

/// Run a full scan/select/clean session.
pub fn run(opts: &CleanOptions, ui: &Ui) {
    let ctx = build_ctx();
    let targets = catalog::build(&ctx);

    println!(
        "{} {}",
        ui.bold("hygiene"),
        ui.dim(&format!(
            "v{} - cleaner for Windows + WSL dev machines",
            env!("CARGO_PKG_VERSION")
        ))
    );
    println!("{}", ui.dim("scanning... (WSL sizes take a few seconds)"));
    let sizes: Vec<u64> = targets.iter().map(|t| t.scan(&ctx)).collect();
    println!();

    for (i, t) in targets.iter().enumerate() {
        let mut label = t.label.clone();
        if t.admin && !ctx.elevated {
            label.push_str(" (needs admin)");
        }
        println!(
            "  {:>2}.  {:<38} {}  {}",
            i + 1,
            label,
            ui.cyan_right(9, &fmt_size(sizes[i])),
            ui.dim(t.desc)
        );
    }
    let total: u64 = sizes.iter().sum();
    println!();
    println!(
        "  {}  {}",
        ui.bold("Total reclaimable:"),
        ui.cyan(&fmt_size(total))
    );

    if opts.list {
        return;
    }

    let Some(selected) = select_targets(opts, targets.len(), ui) else {
        return;
    };

    let sum_sel: u64 = selected.iter().map(|&i| sizes[i]).sum();
    if !opts.yes && !opts.dry_run {
        let ans = ask(&format!(
            "Clean {} item(s), up to {}? [y/N] ",
            selected.len(),
            fmt_size(sum_sel)
        ));
        if !(ans.eq_ignore_ascii_case("y") || ans.eq_ignore_ascii_case("yes")) {
            println!("{}", ui.dim("aborted"));
            return;
        }
    }

    if opts.dry_run {
        println!(
            "\n{}",
            ui.yellow("dry run - nothing was deleted. would clean:")
        );
        for &i in &selected {
            println!("  *  {:<38} {}", targets[i].label, fmt_size(sizes[i]));
        }
        return;
    }

    println!();
    let mut freed_total = 0u64;
    let mut skipped_admin = 0usize;
    let mut failed = 0usize;
    for &i in &selected {
        let t = &targets[i];
        print!("  {:<38} ", t.label);
        let _ = io::stdout().flush();
        if t.admin && !ctx.elevated {
            println!("{}", ui.yellow("skipped (needs admin)"));
            skipped_admin += 1;
            continue;
        }
        match t.clean(&ctx) {
            Ok(freed) => {
                freed_total += freed;
                if freed > 0 {
                    println!(
                        "{}  {}",
                        ui.green("cleaned"),
                        ui.dim(&format!("freed {}", fmt_size(freed)))
                    );
                } else {
                    println!("{}", ui.green("cleaned"));
                }
            }
            Err(e) => {
                failed += 1;
                println!("{}", ui.red(&format!("failed: {e}")));
            }
        }
    }

    println!();
    println!(
        "  {} {}",
        ui.bold("Total freed:"),
        ui.green(&fmt_size(freed_total))
    );
    if skipped_admin > 0 {
        println!(
            "  {}",
            ui.yellow(&format!(
                "{skipped_admin} item(s) need admin - re-run hygiene from an elevated terminal"
            ))
        );
    }
    if failed > 0 {
        println!(
            "  {}",
            ui.red(&format!(
                "{failed} item(s) failed (files in use are skipped)"
            ))
        );
    }
    println!(
        "  {}",
        ui.dim("tip: \"hygiene compact-docker\" / \"hygiene compact-wsl\" shrink disk images (one UAC each).")
    );
}

/// Resolve the target selection from flags or an interactive prompt.
/// Returns `None` when the user quits (message already printed).
fn select_targets(opts: &CleanOptions, count: usize, ui: &Ui) -> Option<Vec<usize>> {
    if let Some(spec) = &opts.only {
        return match parse_selection(spec, count) {
            Ok(v) if !v.is_empty() => Some(v),
            _ => {
                eprintln!("hygiene: bad --only list '{spec}' (use item numbers, e.g. 2,5 or 3-6)");
                std::process::exit(2);
            }
        };
    }
    if opts.all {
        return Some((0..count).collect());
    }
    loop {
        let input = ask("Select items to clean (e.g. \"1 3 5\", \"all\", \"q\" to quit): ");
        if input.is_empty() || input == "q" || input == "quit" {
            println!("{}", ui.dim("nothing to do - bye"));
            return None;
        }
        match parse_selection(&input, count) {
            Ok(v) if !v.is_empty() => return Some(v),
            _ => println!(
                "  {}",
                ui.yellow("didn't understand that - try \"1 3\", \"2-4\", \"all\" or \"q\"")
            ),
        }
    }
}

fn ask(prompt: &str) -> String {
    print!("{prompt}");
    let _ = io::stdout().flush();
    let mut line = String::new();
    let stdin = io::stdin();
    let mut lock = stdin.lock();
    match lock.read_line(&mut line) {
        Ok(0) => "q".to_string(),
        Ok(_) => line.trim().to_string(),
        Err(_) => String::new(),
    }
}

/// Resolve the runtime context once at startup.
fn build_ctx() -> Ctx {
    Ctx {
        temp: env::var("TEMP")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("C:\\Windows\\Temp")),
        localappdata: env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_default(),
        profile: env::var("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_default(),
        elevated: platform::is_elevated(),
        wsl: wsl::detect(),
    }
}
