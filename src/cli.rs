use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::display::{LanguageSortKey, NumberStyle, OutputFormat, SizeStyle, SortDirection};

/// A tool for analyzing code statistics across different programming languages
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
	#[command(subcommand)]
	pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
	/// Analyze a directory or file for code statistics.
	Analyze {
		/// The path to analyze.
		/// This can be either a directory (which will be recursively analyzed)
		/// or a single file. If a directory is provided, all supported source
		/// files within it will be analyzed.
		path: PathBuf,
		/// Enable verbose output.
		#[arg(short, long)]
		verbose: bool,
		/// Do not respect .gitignore files.
		#[arg(short = 'i', long)]
		no_gitignore: bool,
		/// Search hidden files and directories.
		#[arg(short = 'H', long = "hidden")]
		hidden: bool,
		/// Follow symbolic links and include their targets
		/// in the analysis. Use with caution as this can
		/// lead to infinite loops with circular symlinks.
		#[arg(short, long)]
		symlinks: bool,
		/// Output number formatting style.
		#[arg(short = 'n', long, value_enum, default_value = "plain")]
		number_style: NumberStyle,
		/// Human-readable size units.
		#[arg(short = 'u', long = "size-units", value_enum, default_value = "binary")]
		size_style: SizeStyle,
		/// Percentage precision.
		#[arg(short = 'p', long = "precision", default_value_t = 1, value_parser = clap::value_parser!(u8).range(0..=6))]
		percent_precision: u8,
		/// Sorting key for languages (and per-file details when verbose).
		#[arg(short = 'S', long = "sort-by", value_enum, default_value = "lines")]
		language_sort: LanguageSortKey,
		/// Sorting direction.
		#[arg(short = 'd', long = "sort-dir", value_enum, default_value = "desc")]
		sort_direction: SortDirection,
		/// Output format.
		#[arg(short, long, default_value = "human")]
		output: OutputFormat,
	},
	/// List all supported programming languages.
	Langs,
}
