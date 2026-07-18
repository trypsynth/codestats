use std::path::PathBuf;

use anyhow::{Result, ensure};
use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser, Subcommand, parser::ValueSource};
use codestats::{
	config::Config,
	display::{IndentStyle, LanguageSortKey, NumberStyle, OutputFormat, SizeStyle, SortDirection, Verbosity},
};

use crate::completions::Shell;

/// A tool for analyzing code statistics across different programming languages
#[derive(Parser)]
#[command(name = "codestats", version, about, long_about = None)]
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
	/// Generate a default configuration file
	Init {
		/// Output path for the configuration file [default: .codestats.toml]
		#[arg(short, long)]
		output: Option<PathBuf>,
		/// Overwrite the file if it already exists
		#[arg(short, long)]
		force: bool,
	},
}

/// Arguments for the main code analysis functionality
#[derive(Parser)]
// CLI flags necessarily map to booleans, so clippy::struct_excessive_bools would just add noise here.
#[expect(
	clippy::struct_excessive_bools,
	reason = "CLI flags are inherently boolean; grouping them into an enum would add complexity without clarity"
)]
pub struct AnalyzeArgs {
	/// Path to configuration file (TOML format)
	#[arg(short = 'c', long = "config")]
	pub config: Option<PathBuf>,
	/// The path to analyze
	#[arg(value_name = "PATH", default_value = ".")]
	pub path: PathBuf,
	/// Show totals only, no language breakdown
	#[arg(short = 'q', long, conflicts_with = "verbose")]
	pub quiet: bool,
	/// Show per-file details in addition to the language breakdown
	#[arg(short, long, conflicts_with = "quiet")]
	pub verbose: bool,
	/// Do not respect .gitignore files
	#[arg(long)]
	pub no_gitignore: bool,
	/// Count generated files (lockfiles, minified assets) which are excluded by default
	#[arg(long)]
	pub include_generated: bool,
	/// Limit directory traversal to N levels deep
	#[arg(long, value_name = "N")]
	pub max_depth: Option<usize>,
	/// Search hidden files and directories
	#[arg(short = 'H', long = "hidden")]
	pub hidden: bool,
	/// Follow symbolic links and include their targets in the analysis.
	/// Use with caution as this can lead to infinite loops with circular symlinks
	#[arg(long)]
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
	#[arg(short = 'd', long = "sort-direction", value_enum, default_value = "desc")]
	pub sort_direction: SortDirection,
	/// Indentation style: "tab" or a number 1-8 for spaces
	#[arg(long, default_value = "tab")]
	pub indent: IndentStyle,
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
	/// Only show the top N languages in the breakdown
	#[arg(short = 't', long, value_name = "N")]
	pub top_languages: Option<usize>,
	/// Hide languages with fewer than N total lines
	#[arg(long, value_name = "N")]
	pub min_lines: Option<u64>,
	/// Show a breakdown by directory instead of by language
	#[arg(short = 'D', long)]
	pub by_dir: bool,
	/// Exit with a non-zero status code if any files are skipped due to errors.
	#[arg(long = "fail-on-error")]
	pub fail_on_error: bool,
}

impl Cli {
	pub fn parse_with_matches() -> (Self, ArgMatches) {
		let command = Self::command();
		let matches = command.get_matches();
		let cli = Self::from_arg_matches(&matches).expect("clap already validated arguments");
		(cli, matches)
	}
}

/// Merge CLI arguments into `config`, with CLI taking precedence over file-based settings.
///
/// # Errors
///
/// Returns an error when the merged config sets both include and exclude languages.
pub fn merge_config(mut config: Config, args: &AnalyzeArgs, matches: &ArgMatches) -> Result<Config> {
	let path_overridden = cli_overrode(matches, "path");
	if path_overridden {
		config.path.clone_from(&args.path);
	}
	macro_rules! apply {
		($id:literal, $body:expr) => {
			if cli_overrode(matches, $id) {
				$body
			}
		};
	}
	if cli_overrode(matches, "quiet") && args.quiet {
		config.analysis.verbosity = Verbosity::Summary;
	}
	if cli_overrode(matches, "verbose") && args.verbose {
		config.analysis.verbosity = Verbosity::Verbose;
	}
	apply!("no_gitignore", config.analysis.respect_gitignore = !args.no_gitignore);
	apply!("hidden", config.analysis.include_hidden = args.hidden);
	apply!("include_generated", config.analysis.include_generated = args.include_generated);
	apply!("max_depth", config.analysis.max_depth = args.max_depth);
	apply!("symlinks", config.analysis.follow_symlinks = args.symlinks);
	apply!("fail_on_error", config.analysis.fail_on_error = args.fail_on_error);
	apply!("number_style", config.display.number_style = args.number_style);
	apply!("size_style", config.display.size_units = args.size_style);
	apply!("percent_precision", config.display.precision = args.percent_precision);
	apply!("language_sort", config.display.sort_by = args.language_sort);
	apply!("sort_direction", config.display.sort_direction = args.sort_direction);
	apply!("output", config.display.output = args.output);
	apply!("indent", config.display.indent = args.indent);
	apply!("top_languages", config.display.top_languages = args.top_languages);
	apply!("min_lines", config.display.min_lines = args.min_lines);
	apply!("by_dir", config.display.by_dir = args.by_dir);
	if cli_overrode(matches, "exclude") {
		config.analysis.exclude_patterns.extend(args.exclude.clone());
	}
	if cli_overrode(matches, "include_lang") {
		config.analysis.include_languages.extend(args.include_lang.clone());
	}
	if cli_overrode(matches, "exclude_lang") {
		config.analysis.exclude_languages.extend(args.exclude_lang.clone());
	}
	if !path_overridden
		&& config.path_overridden
		&& let Some(source) = &config.source
		&& config.path.is_relative()
		&& let Some(parent) = source.parent()
	{
		config.path = parent.join(&config.path);
	}
	config.display.precision = config.display.precision.min(6);
	ensure!(
		config.analysis.include_languages.is_empty() || config.analysis.exclude_languages.is_empty(),
		"Config cannot set both include_languages and exclude_languages"
	);
	Ok(config)
}

fn cli_overrode(matches: &ArgMatches, id: &str) -> bool {
	matches.value_source(id) == Some(ValueSource::CommandLine)
}

#[cfg(test)]
mod tests {
	use std::{
		env, fs,
		path::PathBuf,
		time::{SystemTime, UNIX_EPOCH},
	};

	use clap::{CommandFactory, FromArgMatches};
	use codestats::{
		config::Config,
		display::{IndentStyle, Verbosity},
	};

	use super::{AnalyzeArgs, Cli, merge_config};

	fn parse_cli(args: &[&str]) -> (AnalyzeArgs, clap::ArgMatches) {
		let matches = Cli::command().get_matches_from(args);
		let cli = Cli::from_arg_matches(&matches).expect("clap already validated arguments");
		(cli.analyze, matches)
	}

	fn write_config(contents: &str) -> PathBuf {
		let unique = SystemTime::now().duration_since(UNIX_EPOCH).expect("system time is set").as_nanos();
		let root = env::temp_dir().join(format!("codestats_config_test_{}_{}", std::process::id(), unique));
		fs::create_dir_all(&root).expect("create temp dir");
		let path = root.join("config.toml");
		fs::write(&path, contents).expect("write temp config");
		path
	}

	#[test]
	fn merge_resolves_config_relative_path() {
		let config_path = write_config("path = \"fixtures\"\n");
		let config = Config::from_file(&config_path).expect("load config");
		let (args, matches) = parse_cli(&["cs"]);
		let merged = merge_config(config, &args, &matches).expect("merge config");
		let expected = config_path.parent().expect("config parent").join("fixtures");
		assert_eq!(merged.path, expected);
	}

	#[test]
	fn merge_preserves_cli_path_override() {
		let config_path = write_config("path = \"fixtures\"\n");
		let config = Config::from_file(&config_path).expect("load config");
		let (args, matches) = parse_cli(&["cs", "custom-path"]);
		let merged = merge_config(config, &args, &matches).expect("merge config");
		assert_eq!(merged.path, PathBuf::from("custom-path"));
	}

	#[test]
	fn merge_clamps_precision() {
		let config_path = write_config("[display]\nprecision = 9\n");
		let config = Config::from_file(&config_path).expect("load config");
		let (args, matches) = parse_cli(&["cs"]);
		let merged = merge_config(config, &args, &matches).expect("merge config");
		assert_eq!(merged.display.precision, 6);
	}

	#[test]
	fn merge_extends_exclude_patterns_from_cli() {
		let config_path = write_config("[analysis]\nexclude_patterns = [\"target\"]\n");
		let config = Config::from_file(&config_path).expect("load config");
		let (args, matches) = parse_cli(&["cs", "--exclude", "node_modules"]);
		let merged = merge_config(config, &args, &matches).expect("merge config");
		assert_eq!(merged.analysis.exclude_patterns, vec!["target".to_string(), "node_modules".to_string()]);
	}

	#[test]
	fn merge_rejects_conflicting_language_filters() {
		let config_path = write_config("[analysis]\ninclude_languages = [\"Rust\"]\n");
		let config = Config::from_file(&config_path).expect("load config");
		let (args, matches) = parse_cli(&["cs", "--exclude-lang", "python"]);
		let err = merge_config(config, &args, &matches).expect_err("conflicting language filters");
		assert_eq!(err.to_string(), "Config cannot set both include_languages and exclude_languages");
	}

	#[test]
	fn merge_applies_boolean_overrides() {
		let config_path = write_config("[analysis]\nrespect_gitignore = true\ninclude_hidden = false\n");
		let config = Config::from_file(&config_path).expect("load config");
		let (args, matches) = parse_cli(&["cs", "--no-gitignore", "--hidden"]);
		let merged = merge_config(config, &args, &matches).expect("merge config");
		assert!(!merged.analysis.respect_gitignore);
		assert!(merged.analysis.include_hidden);
	}

	#[test]
	fn merge_applies_indent_from_cli() {
		let config_path = write_config("");
		let config = Config::from_file(&config_path).expect("load config");
		let (args, matches) = parse_cli(&["cs", "--indent", "4"]);
		let merged = merge_config(config, &args, &matches).expect("merge config");
		assert_eq!(merged.display.indent, IndentStyle::Spaces(4));
	}

	#[test]
	fn merge_applies_indent_from_config() {
		let config_path = write_config("[display]\nindent = \"2\"\n");
		let config = Config::from_file(&config_path).expect("load config");
		let (args, matches) = parse_cli(&["cs"]);
		let merged = merge_config(config, &args, &matches).expect("merge config");
		assert_eq!(merged.display.indent, IndentStyle::Spaces(2));
	}

	#[test]
	fn merge_indent_defaults_to_tab() {
		let config_path = write_config("");
		let config = Config::from_file(&config_path).expect("load config");
		let (args, matches) = parse_cli(&["cs"]);
		let merged = merge_config(config, &args, &matches).expect("merge config");
		assert_eq!(merged.display.indent, IndentStyle::Tab);
	}

	#[test]
	fn merge_applies_verbosity_overrides() {
		let config_path = write_config("[analysis]\nverbosity = \"regular\"\n");
		let config = Config::from_file(&config_path).expect("load config");

		let (args, matches) = parse_cli(&["cs", "--quiet"]);
		let merged = merge_config(config.clone(), &args, &matches).expect("merge config");
		assert_eq!(merged.analysis.verbosity, Verbosity::Summary);

		let (args, matches) = parse_cli(&["cs", "--verbose"]);
		let merged = merge_config(config, &args, &matches).expect("merge config");
		assert_eq!(merged.analysis.verbosity, Verbosity::Verbose);
	}
}
