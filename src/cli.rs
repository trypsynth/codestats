use clap::{ArgAction, Parser};
use std::path::PathBuf;

/// Represents the command-line arguments supported by Codestats.
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// The path to analyze
    ///
    /// This can be either a directory (which will be recursively analyzed)
    /// or a single file. If a directory is provided, all supported source
    /// files within it will be analyzed.
    pub path: PathBuf,
    /// Enable verbose output
    ///
    /// When enabled, provides additional details about the analysis process,
    /// including which files are being processed and any warnings or errors
    /// encountered during analysis.
    #[arg(short, long)]
    pub verbose: bool,
    /// Respect .gitignore/.ignore files
    ///
    /// When enabled (default), files and directories listed in .gitignore,
    /// .ignore, and similar files will be excluded from analysis.
    /// Use `--no-gitignore` to disable this behavior.
    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    pub gitignore: bool,
    /// Ignore hidden files
    ///
    /// When enabled (default), files and directories starting with a dot (.)
    /// will be excluded from analysis, except for common configuration files.
    /// Use `--no-hidden` to include hidden files.
    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    pub hidden: bool,
    /// Follow symlinks
    ///
    /// When enabled, symbolic links will be followed and their targets
    /// will be included in the analysis. Use with caution as this can
    /// lead to infinite loops with circular symlinks.
    #[arg(short, long)]
    pub symlinks: bool,
}

/// Wrapper function to avoid needing to use `clap::Parser` in `main.rs`.
#[must_use]
pub fn parse_cli() -> Cli {
    Cli::parse()
}
