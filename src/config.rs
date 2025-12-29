use std::{
	env, fs,
	path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::{ArgMatches, parser::ValueSource};
use etcetera::{BaseStrategy, choose_base_strategy};
use serde::{Deserialize, Serialize};

use crate::{
	cli::Cli,
	display::{LanguageSortKey, NumberStyle, OutputFormat, SizeStyle, SortDirection, ViewOptions},
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
	pub path: PathBuf,
	pub analysis: AnalysisConfig,
	pub display: DisplayConfig,
	#[serde(skip)]
	pub source: Option<PathBuf>,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			path: PathBuf::from("."),
			analysis: AnalysisConfig::default(),
			display: DisplayConfig::default(),
			source: None,
		}
	}
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct AnalysisConfig {
	pub verbose: bool,
	pub respect_gitignore: bool,
	pub include_hidden: bool,
	pub follow_symlinks: bool,
}

impl Default for AnalysisConfig {
	fn default() -> Self {
		Self { verbose: false, respect_gitignore: true, include_hidden: false, follow_symlinks: false }
	}
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct DisplayConfig {
	pub number_style: NumberStyle,
	pub size_units: SizeStyle,
	pub precision: u8,
	pub sort_by: LanguageSortKey,
	pub sort_direction: SortDirection,
	pub output: OutputFormat,
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
		}
	}
}

#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct AnalyzerConfig {
	pub analysis: AnalysisConfig,
	pub collect_file_details: bool,
}

impl Default for AnalyzerConfig {
	fn default() -> Self {
		Self { analysis: AnalysisConfig::default(), collect_file_details: false }
	}
}

impl Config {
	pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
		let path = path.as_ref();
		let contents = fs::read_to_string(path).with_context(|| read_config_context(path))?;
		let mut config: Self = toml::from_str(&contents).with_context(|| parse_config_context(path))?;
		config.source = Some(path.to_path_buf());
		Ok(config)
	}

	pub fn load_default() -> Result<Self> {
		Self::find_config_file().map_or_else(|| Ok(Self::default()), Self::from_file)
	}

	pub fn find_config_file() -> Option<PathBuf> {
		let mut candidates = vec![PathBuf::from(".codestats.toml"), PathBuf::from("codestats.toml")];
		if let Ok(strategy) = choose_base_strategy() {
			candidates.push(strategy.config_dir().join("codestats").join("config.toml"));
			candidates.push(strategy.home_dir().join(".codestats.toml"));
		} else if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
			candidates.push(home.join(".codestats.toml"));
		}
		candidates.into_iter().find(|path| path.is_file())
	}

	/// Merge CLI arguments into this configuration, with CLI taking precedence.
	pub fn merge_with_cli(mut self, cli: &Cli, matches: &ArgMatches) -> Self {
		let path_overridden = Self::cli_overrode(matches, "path");
		if path_overridden {
			self.path.clone_from(&cli.path);
		}
		macro_rules! apply {
			($id:literal, $body:expr) => {
				if Self::cli_overrode(matches, $id) {
					$body
				}
			};
		}
		apply!("verbose", self.analysis.verbose = cli.verbose);
		apply!("no_gitignore", self.analysis.respect_gitignore = !cli.no_gitignore);
		apply!("hidden", self.analysis.include_hidden = cli.hidden);
		apply!("symlinks", self.analysis.follow_symlinks = cli.symlinks);
		apply!("number_style", self.display.number_style = cli.number_style);
		apply!("size_style", self.display.size_units = cli.size_style);
		apply!("percent_precision", self.display.precision = cli.percent_precision);
		apply!("language_sort", self.display.sort_by = cli.language_sort);
		apply!("sort_direction", self.display.sort_direction = cli.sort_direction);
		apply!("output", self.display.output = cli.output);
		if !path_overridden
			&& let Some(source) = &self.source
			&& self.path.is_relative()
			&& let Some(parent) = source.parent()
		{
			self.path = parent.join(&self.path);
		}
		self.display.precision = self.display.precision.min(6);
		self
	}

	fn cli_overrode(matches: &ArgMatches, id: &str) -> bool {
		matches.value_source(id) == Some(ValueSource::CommandLine)
	}
}

impl From<&Config> for AnalyzerConfig {
	fn from(config: &Config) -> Self {
		Self { analysis: config.analysis.clone(), collect_file_details: config.analysis.verbose }
	}
}

impl From<&Config> for ViewOptions {
	fn from(config: &Config) -> Self {
		Self {
			number_style: config.display.number_style,
			size_style: config.display.size_units,
			percent_precision: config.display.precision,
			language_sort_key: config.display.sort_by,
			sort_direction: config.display.sort_direction,
		}
	}
}
