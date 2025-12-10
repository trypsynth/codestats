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

#[derive(Clone, Debug, Default)]
pub struct AnalyzerConfig {
	pub verbose: bool,
	pub traversal: TraversalOptions,
	pub detail_level: DetailLevel,
}

#[derive(Clone, Copy, Debug)]
pub struct TraversalOptions {
	pub respect_gitignore: bool,
	pub include_hidden: bool,
	pub follow_symlinks: bool,
}

impl Default for TraversalOptions {
	fn default() -> Self {
		Self { respect_gitignore: true, include_hidden: false, follow_symlinks: false }
	}
}

#[derive(Clone, Copy, Debug, Default)]
pub enum DetailLevel {
	#[default]
	Summary,
	PerFile,
}

impl DetailLevel {
	#[must_use]
	pub(crate) const fn collect_file_details(self) -> bool {
		matches!(self, Self::PerFile)
	}
}

impl Config {
	pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
		let path = path.as_ref();
		let contents =
			fs::read_to_string(path).with_context(|| format!("Failed to read config file `{}`", path.display()))?;
		let mut config: Self =
			toml::from_str(&contents).with_context(|| format!("Failed to parse config file `{}`", path.display()))?;
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

	pub fn merge_with_cli(mut self, cli: &Cli, matches: &ArgMatches) -> Self {
		if Self::cli_overrode(matches, "path") {
			self.path.clone_from(&cli.path);
		}
		if Self::cli_overrode(matches, "verbose") {
			self.analysis.verbose = cli.verbose;
		}
		if Self::cli_overrode(matches, "no_gitignore") {
			self.analysis.respect_gitignore = !cli.no_gitignore;
		}
		if Self::cli_overrode(matches, "hidden") {
			self.analysis.include_hidden = cli.hidden;
		}
		if Self::cli_overrode(matches, "symlinks") {
			self.analysis.follow_symlinks = cli.symlinks;
		}
		if Self::cli_overrode(matches, "number_style") {
			self.display.number_style = cli.number_style;
		}
		if Self::cli_overrode(matches, "size_style") {
			self.display.size_units = cli.size_style;
		}
		if Self::cli_overrode(matches, "percent_precision") {
			self.display.precision = cli.percent_precision.min(6);
		}
		if Self::cli_overrode(matches, "language_sort") {
			self.display.sort_by = cli.language_sort;
		}
		if Self::cli_overrode(matches, "sort_direction") {
			self.display.sort_direction = cli.sort_direction;
		}
		if Self::cli_overrode(matches, "output") {
			self.display.output = cli.output;
		}
		if self.display.precision > 6 {
			self.display.precision = 6;
		}
		self
	}

	fn cli_overrode(matches: &ArgMatches, id: &str) -> bool {
		matches.value_source(id) == Some(ValueSource::CommandLine)
	}
}

impl From<&Config> for AnalyzerConfig {
	fn from(config: &Config) -> Self {
		Self {
			verbose: config.analysis.verbose,
			traversal: TraversalOptions {
				respect_gitignore: config.analysis.respect_gitignore,
				include_hidden: config.analysis.include_hidden,
				follow_symlinks: config.analysis.follow_symlinks,
			},
			detail_level: if config.analysis.verbose { DetailLevel::PerFile } else { DetailLevel::Summary },
		}
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
