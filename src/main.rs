#![warn(clippy::all, clippy::cargo, clippy::nursery, clippy::pedantic)]

mod analysis;
mod cli;
mod config;
mod display;
mod langs;
mod utils;

use std::io::{self, Write as _};

use anyhow::{Result, ensure};
use clap::{CommandFactory, FromArgMatches};
use cli::Cli;
use terminal_size::terminal_size;

use crate::{
	analysis::CodeAnalyzer,
	config::{AnalyzerConfig, Config},
	display::{ViewOptions, get_formatter},
};

fn main() -> Result<()> {
	let matches = Cli::command().get_matches();
	let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());
	if cli.langs {
		let mut stdout = io::stdout();
		let terminal_width = terminal_size().map_or(80, |(w, _)| usize::from(w.0));
		langs::print_all_languages(&mut stdout, terminal_width)?;
		stdout.flush()?;
		return Ok(());
	}
	let config =
		if let Some(ref config_path) = cli.config { Config::from_file(config_path)? } else { Config::load_default()? };
	let config = config.merge_with_cli(&cli, &matches);
	ensure!(config.path.exists(), "Path `{}` not found", config.path.display());
	if config.path.is_file() {
		ensure!(config.path.metadata().is_ok(), "Cannot read file metadata for `{}`", config.path.display());
	}
	let analyzer_config: AnalyzerConfig = (&config).into();
	let verbose = config.analysis.verbose;
	let analyzer = CodeAnalyzer::new(&config.path, analyzer_config);
	let results = analyzer.analyze()?;
	let view_options: ViewOptions = (&config).into();
	let formatter = get_formatter(config.display.output);
	let mut stdout = io::stdout();
	formatter.write_output(&results, &config.path, verbose, view_options, &mut stdout)?;
	stdout.flush()?;
	Ok(())
}
