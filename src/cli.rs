use std::path::PathBuf;

use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser, Subcommand};

use crate::{
	completions::Shell,
	display::{LanguageSortKey, NumberStyle, OutputFormat, SizeStyle, SortDirection},
};

/// A tool for analyzing code statistics across different programming languages
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
	#[command(subcommand)]
	pub command: Option<Commands>,
	#[command(flatten)]
	pub analyze: AnalyzeArgs,
}

/// Available subcommands
#[derive(Subcommand)]
pub enum Commands {
	/// Generate shell completion scripts
	Completions {
		/// The shell to generate completions for
		#[arg(value_enum)]
		shell: Shell,
	},
	/// List all supported programming languages
	Langs,
}

/// Arguments for the main code analysis functionality
#[derive(Parser)]
// CLI flags necessarily map to booleans, so clippy::struct_excessive_bools would just add noise here.
#[allow(clippy::struct_excessive_bools)]
pub struct AnalyzeArgs {
	/// Path to configuration file (TOML format)
	#[arg(short = 'c', long = "config")]
	pub config: Option<PathBuf>,
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
	/// Only analyze files of the specified language(s). Can be specified multiple times, and cannot be used together with --exclude-lang.
	#[arg(short = 'L', long = "lang", conflicts_with = "exclude_lang")]
	pub include_lang: Vec<String>,
	/// Exclude files of the specified language(s). Can be specified multiple times, and cannot be used together with --lang.
	#[arg(long = "exclude-lang", conflicts_with = "include_lang")]
	pub exclude_lang: Vec<String>,
}

impl Cli {
	pub fn parse_with_matches() -> (Self, ArgMatches) {
		let command = Self::command();
		let matches = command.get_matches();
		let cli = Self::from_arg_matches(&matches).expect("clap already validated arguments");
		(cli, matches)
	}
}
