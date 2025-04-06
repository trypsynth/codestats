use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about)]
pub struct Cli {
    #[arg(help = "The path to analyze")]
    pub path: PathBuf,
    #[arg(short, long, help = "Show much more verbose output")]
    pub verbose: bool,
    #[arg(
        short,
        long,
        default_value_t = true,
        help = "Respect .gitignore/.ignore files"
    )]
    pub ignores: bool,
    #[arg(short, long, help = "Follow symlinks")]
    pub symlinks: bool,
    #[arg(short, long, help = default_value_t = true, "Ignore hidden files")]
    pub no_hidden: bool,
}

// Wrapper function to avoid needing to `use clap::Parser;` in `main.rs`.
pub fn parse_cli() -> Cli {
    Cli::parse()
}
