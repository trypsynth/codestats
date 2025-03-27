use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(help = "The path to analyze.")]
    pub path: PathBuf,
    #[arg(
        short,
        long,
        default_value_t = false,
        help = "Show much more verbose output."
    )]
    pub verbose: bool,
    #[arg(
        short,
        long,
        default_value_t = true,
        help = "Respect .gitignore files."
    )]
    pub gitignores: bool,
}

// Wrapper function to avoid needing to use clap::Parser in main.rs.
pub fn parse_cli() -> Cli {
    Cli::parse()
}
