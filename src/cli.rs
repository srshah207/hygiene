//! Command-line parsing: maps argv to a [`Command`]. Pure and side-effect
//! free (aside from reading `env::args`) so it can be unit-tested.

use std::env;

pub const HELP: &str = r#"hygiene - interactive cleaner for Windows + WSL dev machines

USAGE:
  hygiene                     scan, pick items interactively, clean
  hygiene --list              only show sizes, clean nothing
  hygiene --all --yes         clean everything (non-admin items) without prompts
  hygiene --only 2,5 --yes    clean only the numbered items (ranges ok: 2-5)
  hygiene --all --dry-run     show what --all would clean
  hygiene compact-docker      shrink the Docker disk image (asks for UAC)
  hygiene compact-wsl         trim + shrink WSL distro disk images (asks for UAC)

OPTIONS:
  -a, --all       select all items
      --only LIST select specific items (e.g. 2,5 or 3-6)
  -y, --yes       skip confirmation
  -n, --dry-run   never delete anything
  -l, --list      list items and sizes
      --no-color  disable coloring
  -h, --help      show this help
  -V, --version   print version
"#;

/// What the user asked hygiene to do.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Help,
    Version,
    CompactDocker { color: bool },
    CompactWsl { color: bool },
    Clean(CleanOptions),
}

/// Options for a full scan / clean session.
#[derive(Debug, PartialEq, Eq)]
pub struct CleanOptions {
    pub all: bool,
    pub yes: bool,
    pub dry_run: bool,
    pub list: bool,
    pub only: Option<String>,
    pub color: bool,
}

impl Default for CleanOptions {
    fn default() -> Self {
        Self {
            all: false,
            yes: false,
            dry_run: false,
            list: false,
            only: None,
            color: true,
        }
    }
}

impl Command {
    /// Whether colored output was requested for this command.
    pub fn color(&self) -> bool {
        match self {
            Command::Help | Command::Version => false,
            Command::CompactDocker { color } | Command::CompactWsl { color } => *color,
            Command::Clean(opts) => opts.color,
        }
    }
}

/// Parse the real process arguments.
pub fn parse() -> Result<Command, String> {
    parse_from(env::args().skip(1))
}

/// Parse an argument list (kept separate for unit tests).
pub fn parse_from<I>(args: I) -> Result<Command, String>
where
    I: IntoIterator<Item = String>,
{
    let args: Vec<String> = args.into_iter().collect();
    let mut opts = CleanOptions::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--all" | "-a" => opts.all = true,
            "--yes" | "-y" => opts.yes = true,
            "--dry-run" | "-n" => opts.dry_run = true,
            "--list" | "-l" => opts.list = true,
            "--no-color" => opts.color = false,
            "--only" => {
                i += 1;
                let value = args
                    .get(i)
                    .ok_or("--only needs a value (e.g. --only 2,5)")?;
                opts.only = Some(value.clone());
            }
            arg if arg.starts_with("--only=") => {
                opts.only = Some(arg["--only=".len()..].to_string());
            }
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "compact-docker" => return Ok(Command::CompactDocker { color: opts.color }),
            "compact-wsl" => return Ok(Command::CompactWsl { color: opts.color }),
            other => return Err(format!("unknown argument '{other}'")),
        }
        i += 1;
    }
    Ok(Command::Clean(opts))
}

/// Parse user input like `1 3 5`, `2-4`, or `all` into zero-based indices.
pub fn parse_selection(input: &str, count: usize) -> Result<Vec<usize>, ()> {
    let lower = input.to_lowercase();
    if lower == "all" || lower == "a" {
        return Ok((0..count).collect());
    }
    let mut out = Vec::new();
    for tok in input
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
    {
        if let Some((a, b)) = tok.split_once('-') {
            let a: usize = a.trim().parse().map_err(|_| ())?;
            let b: usize = b.trim().parse().map_err(|_| ())?;
            if a == 0 || b == 0 || a > b || b > count {
                return Err(());
            }
            out.extend(a - 1..b);
        } else {
            let n: usize = tok.parse().map_err(|_| ())?;
            if n == 0 || n > count {
                return Err(());
            }
            out.push(n - 1);
        }
    }
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(args: &[&str]) -> Result<Command, String> {
        parse_from(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn selection_accepts_lists_ranges_and_all() {
        assert_eq!(parse_selection("1 3", 5).unwrap(), vec![0, 2]);
        assert_eq!(parse_selection("2-4", 5).unwrap(), vec![1, 2, 3]);
        assert_eq!(parse_selection("1,1,3", 5).unwrap(), vec![0, 2]);
        assert_eq!(parse_selection("all", 3).unwrap(), vec![0, 1, 2]);
    }

    #[test]
    fn selection_rejects_out_of_range_and_zero() {
        assert!(parse_selection("0", 3).is_err());
        assert!(parse_selection("4", 3).is_err());
        assert!(parse_selection("3-1", 3).is_err());
        assert!(parse_selection("x", 3).is_err());
    }

    #[test]
    fn parses_clean_flags() {
        match parse_args(&["--only", "2,5", "--yes", "--no-color"]).unwrap() {
            Command::Clean(opts) => {
                assert!(opts.yes);
                assert!(!opts.color);
                assert_eq!(opts.only.as_deref(), Some("2,5"));
                assert!(!opts.all);
            }
            other => panic!("expected Clean, got {other:?}"),
        }
    }

    #[test]
    fn parses_only_equals_syntax() {
        match parse_args(&["--only=7"]).unwrap() {
            Command::Clean(opts) => assert_eq!(opts.only.as_deref(), Some("7")),
            other => panic!("expected Clean, got {other:?}"),
        }
    }

    #[test]
    fn parses_subcommands_and_help() {
        assert_eq!(parse_args(&["--help"]).unwrap(), Command::Help);
        assert_eq!(parse_args(&["-V"]).unwrap(), Command::Version);
        assert_eq!(
            parse_args(&["compact-wsl"]).unwrap(),
            Command::CompactWsl { color: true }
        );
        assert_eq!(
            parse_args(&["--no-color", "compact-docker"]).unwrap(),
            Command::CompactDocker { color: false }
        );
    }

    #[test]
    fn rejects_unknown_arguments() {
        assert!(parse_args(&["--wat"]).is_err());
        assert!(parse_args(&["--only"]).is_err());
    }
}
