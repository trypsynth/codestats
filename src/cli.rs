use clap::{ArgAction, Parser, Subcommand};
use std::path::PathBuf;

/// A tool for analyzing code statistics across different programming languages
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
	/// Analyze a directory or file for code statistics
	Analyze {
		/// The path to analyze
		///
		/// This can be either a directory (which will be recursively analyzed)
		/// or a single file. If a directory is provided, all supported source
		/// files within it will be analyzed.
		path: PathBuf,
		/// Enable verbose output
		#[arg(short, long)]
		verbose: bool,
		/// Respect .gitignore/.ignore files
		#[arg(long, default_value_t = true, action = ArgAction::Set)]
		gitignore: bool,
		/// Ignore hidden files
		#[arg(long, default_value_t = true, action = ArgAction::Set)]
		hidden: bool,
		/// Follow symlinks
		///
		/// When enabled, symbolic links will be followed and their targets
		/// will be included in the analysis. Use with caution as this can
		/// lead to infinite loops with circular symlinks.
		#[arg(short, long)]
		symlinks: bool,
	},
	/// List all supported programming languages
	Langs,
}

/// Wrapper function to avoid needing to use `clap::Parser` in `main.rs`.
#[must_use]
pub(crate) fn parse_cli() -> Cli {
	Cli::parse()
}
