use std::path::PathBuf;

use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser};

use crate::display::{LanguageSortKey, NumberStyle, OutputFormat, SizeStyle, SortDirection};

/// A tool for analyzing code statistics across different programming languages
#[derive(Parser)]
#[command(version, about, long_about = None)]
// CLI flags necessarily map to booleans, so clippy::struct_excessive_bools would just add noise here.
#[allow(clippy::struct_excessive_bools)]
pub struct Cli {
	/// Path to configuration file (TOML format)
	#[arg(short = 'c', long = "config")]
	pub config: Option<PathBuf>,
	/// List all supported programming languages and exit without running analysis
	#[arg(short = 'l', long = "langs")]
	pub langs: bool,
	/// The path to analyze
	#[arg(value_name = "PATH", default_value = ".")]
	pub path: PathBuf,
	/// Enable verbose output
	#[arg(short, long)]
	pub verbose: bool,
	/// Do not respect .gitignore files
	#[arg(short = 'i', long)]
	pub no_gitignore: bool,
	/// Search hidden files and directories
	#[arg(short = 'H', long = "hidden")]
	pub hidden: bool,
	/// Follow symbolic links and include their targets in the analysis.
	/// Use with caution as this can lead to infinite loops with circular symlinks
	#[arg(short = 'S', long)]
	pub symlinks: bool,
	/// Output number formatting style
	#[arg(short, long, value_enum, default_value = "plain")]
	pub number_style: NumberStyle,
	/// Human-readable size units
	#[arg(short = 'u', long = "size-units", value_enum, default_value = "binary")]
	pub size_style: SizeStyle,
	/// Percentage precision (0-6)
	#[arg(short = 'p', long = "precision", default_value_t = 1, value_parser = clap::value_parser!(u8).range(0..=6))]
	pub percent_precision: u8,
	/// Sorting key for languages (and per-file details when verbose)
	#[arg(short = 's', long = "sort-by", value_enum, default_value = "lines")]
	pub language_sort: LanguageSortKey,
	/// Sorting direction
	#[arg(short = 'd', long = "sort-dir", value_enum, default_value = "desc")]
	pub sort_direction: SortDirection,
	/// Output format
	#[arg(short, long, default_value = "human")]
	pub output: OutputFormat,
	/// Exclude files or directories matching the given glob patterns. Can be specified more than once.
	#[arg(short, long)]
	pub exclude: Vec<String>,
}

impl Cli {
	pub fn parse_with_matches() -> (Self, ArgMatches) {
		let command = Self::command();
		let matches = command.get_matches();
		let cli = Self::from_arg_matches(&matches).expect("clap already validated arguments");
		(cli, matches)
	}
}
