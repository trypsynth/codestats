use std::{
	fs,
	path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

use crate::display::{
	IndentStyle, LanguageSortKey, NumberStyle, OutputFormat, SizeStyle, SortDirection, Verbosity, ViewOptions,
};

/// Helper to create error context for config file reading operations.
#[inline]
fn read_config_context(path: &Path) -> String {
	format!("Failed to read config file `{}`", path.display())
}

/// Helper to create error context for config file parsing operations.
#[inline]
fn parse_config_context(path: &Path) -> String {
	format!("Failed to parse config file `{}`", path.display())
}

/// Resolved configuration after loading defaults, config files, and CLI overrides.
#[derive(Clone, Debug)]
pub struct Config {
	/// The paths to count together, never empty.
	pub paths: Vec<PathBuf>,
	pub analysis: AnalysisConfig,
	pub display: DisplayConfig,
	/// Path to the config file that provided these settings, if any.
	pub source: Option<PathBuf>,
	/// True when the config file explicitly sets `paths`.
	pub paths_overridden: bool,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			paths: vec![PathBuf::from(".")],
			analysis: AnalysisConfig::default(),
			display: DisplayConfig::default(),
			source: None,
			paths_overridden: false,
		}
	}
}

/// Analysis settings loaded from TOML and the CLI.
#[expect(
	clippy::struct_excessive_bools,
	reason = "each bool maps to a distinct on/off analysis option with no meaningful grouping as an enum"
)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AnalysisConfig {
	pub verbosity: Verbosity,
	pub respect_gitignore: bool,
	/// Read linguist attributes from `.gitattributes` files.
	pub respect_gitattributes: bool,
	pub include_hidden: bool,
	pub follow_symlinks: bool,
	pub include_generated: bool,
	/// Only count files git tracks.
	pub git_tracked_only: bool,
	pub max_depth: Option<usize>,
	pub exclude_patterns: Vec<String>,
	pub include_languages: Vec<String>,
	pub exclude_languages: Vec<String>,
	pub fail_on_error: bool,
}

impl Default for AnalysisConfig {
	fn default() -> Self {
		Self {
			verbosity: Verbosity::Regular,
			respect_gitignore: true,
			respect_gitattributes: true,
			include_hidden: false,
			follow_symlinks: false,
			include_generated: false,
			git_tracked_only: false,
			max_depth: None,
			exclude_patterns: Vec::new(),
			include_languages: Vec::new(),
			exclude_languages: Vec::new(),
			fail_on_error: false,
		}
	}
}

/// Output formatting settings loaded from TOML and the CLI.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DisplayConfig {
	pub number_style: NumberStyle,
	pub size_units: SizeStyle,
	pub precision: u8,
	pub sort_by: LanguageSortKey,
	pub sort_direction: SortDirection,
	pub output: OutputFormat,
	pub indent: IndentStyle,
	pub top_languages: Option<usize>,
	pub min_lines: Option<u64>,
	pub by_dir: bool,
}

impl Default for DisplayConfig {
	fn default() -> Self {
		Self {
			number_style: NumberStyle::Plain,
			size_units: SizeStyle::Binary,
			precision: 1,
			sort_by: LanguageSortKey::Lines,
			sort_direction: SortDirection::Desc,
			output: OutputFormat::Human,
			indent: IndentStyle::Tab,
			top_languages: None,
			min_lines: None,
			by_dir: false,
		}
	}
}

/// Internal analyzer settings derived from the merged config.
#[derive(Clone, Debug, Default)]
pub struct AnalyzerConfig {
	pub analysis: AnalysisConfig,
	pub collect_file_details: bool,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct RawConfig {
	#[serde(alias = "path")]
	paths: Option<Paths>,
	analysis: AnalysisConfig,
	display: DisplayConfig,
}

/// `paths` in a config file: a list, or one path on its own.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Paths {
	One(PathBuf),
	Many(Vec<PathBuf>),
}

impl Config {
	/// # Errors
	///
	/// Returns an error if the file cannot be read or its TOML is invalid.
	pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
		let path = path.as_ref();
		let contents = fs::read_to_string(path).with_context(|| read_config_context(path))?;
		let raw: RawConfig = toml::from_str(&contents).with_context(|| parse_config_context(path))?;
		let paths = match raw.paths {
			None => None,
			Some(Paths::One(path)) => Some(vec![path]),
			Some(Paths::Many(paths)) => {
				ensure!(!paths.is_empty(), "{}: `paths` needs at least one path", parse_config_context(path));
				Some(paths)
			}
		};
		Ok(Self {
			paths_overridden: paths.is_some(),
			paths: paths.unwrap_or_else(|| vec![PathBuf::from(".")]),
			analysis: raw.analysis,
			display: raw.display,
			source: Some(path.to_path_buf()),
		})
	}
}

impl From<&Config> for AnalyzerConfig {
	fn from(config: &Config) -> Self {
		Self {
			analysis: config.analysis.clone(),
			collect_file_details: config.analysis.verbosity == Verbosity::Verbose || config.display.by_dir,
		}
	}
}

impl From<&Config> for ViewOptions {
	fn from(config: &Config) -> Self {
		Self {
			verbosity: config.analysis.verbosity,
			number_style: config.display.number_style,
			size_style: config.display.size_units,
			percent_precision: config.display.precision,
			language_sort_key: config.display.sort_by,
			sort_direction: config.display.sort_direction,
			indent_style: config.display.indent,
			top_languages: config.display.top_languages,
			min_lines: config.display.min_lines,
			by_dir: config.display.by_dir,
		}
	}
}
