//! hygiene - interactive cleaner for Windows + WSL dev machines.
//!
//! Zero-dependency, single-binary, offline. This entry point only parses
//! arguments and dispatches; all logic lives in the modules below
//! (see `session`, `catalog`, `cleaners`, `compact`).

mod catalog;
mod cleaners;
mod cli;
mod compact;
mod fsutil;
mod model;
mod platform;
mod session;
mod ui;

use cli::Command;
use ui::Ui;

fn main() {
    // Enable UTF-8 output + virtual-terminal escape codes up front
    // (harmless on consoles that do not support them).
    let vt = ui::init_terminal();

    let command = match cli::parse() {
        Ok(cmd) => cmd,
        Err(err) => {
            eprintln!("hygiene: {err} (try --help)");
            std::process::exit(2);
        }
    };

    let color = command.color() && vt;
    match command {
        Command::Help => println!("{}", cli::HELP),
        Command::Version => println!("hygiene {}", env!("CARGO_PKG_VERSION")),
        Command::CompactDocker { .. } => compact::docker(&Ui::new(color)),
        Command::CompactWsl { .. } => compact::wsl(&Ui::new(color)),
        Command::Clean(opts) => session::run(&opts, &Ui::new(color)),
    }
}
