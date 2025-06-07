use clap::{ArgAction, Parser};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about)]
pub struct Cli {
    #[arg(help = "The path to analyze")]
    pub path: PathBuf,
    #[arg(short, long, help = "Enable verbose output")]
    pub verbose: bool,
    #[arg(
        long,
        default_value_t = true,
        action = ArgAction::Set,
        help = "Respect .gitignore/.ignore files"
    )]
    pub gitignore: bool,
    #[arg(
        long,
        default_value_t = true,
        action = ArgAction::Set,
        help = "Ignore hidden files"
    )]
    pub hidden: bool,
    #[arg(short, long, help = "Follow symlinks")]
    pub symlinks: bool,
}

/// Wrapper function to avoid needing to `use clap::Parser;` in `main.rs`.
#[must_use]
pub fn parse_cli() -> Cli {
    Cli::parse()
}
