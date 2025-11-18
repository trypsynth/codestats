use std::path::PathBuf;

use clap::{Parser, Subcommand};
use codestats::display::OutputFormat;

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
		/// This can be either a directory (which will be recursively analyzed)
		/// or a single file. If a directory is provided, all supported source
		/// files within it will be analyzed.
		path: PathBuf,
		/// Enable verbose output
		#[arg(short, long)]
		verbose: bool,
		/// Do not respect .gitignore files
		#[arg(short, long)]
		no_gitignore: bool,
		/// Search hidden files and directories
		#[arg(long)]
		hidden: bool,
		/// Follow symbolic links and include their targets
		/// in the analysis. Use with caution as this can
		/// lead to infinite loops with circular symlinks.
		#[arg(short, long)]
		symlinks: bool,
		/// Output format
		#[arg(short, long, default_value = "human")]
		output: OutputFormat,
	},
	/// List all supported programming languages
	Langs,
}
