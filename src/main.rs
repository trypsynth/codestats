#![warn(clippy::all, clippy::cargo, clippy::nursery, clippy::pedantic, clippy::perf)]
#![deny(warnings)]

mod analysis;
mod cli;
mod completions;
mod config;
mod config_discovery;
mod display;
mod init;
mod langs;
mod swar;

use std::io::{self, Write as _};

use analysis::CodeAnalyzer;
use anyhow::{Result, anyhow, ensure};
use cli::{Cli, Commands};
use config::{AnalyzerConfig, Config};
use display::ViewOptions;
use terminal_size::terminal_size;

fn main() -> Result<()> {
	match run() {
		// The reader went away (`cs | head`); there's no one left to report to.
		Err(err) if is_broken_pipe(&err) => Ok(()),
		result => result,
	}
}

/// Whether `err` comes from writing to a pipe whose reader has closed it.
fn is_broken_pipe(err: &anyhow::Error) -> bool {
	err.chain().any(|cause| {
		cause.downcast_ref::<io::Error>().is_some_and(|e| e.kind() == io::ErrorKind::BrokenPipe)
			// serde_json wraps the I/O error without exposing it as a source.
			|| cause.downcast_ref::<serde_json::Error>().and_then(serde_json::Error::io_error_kind)
				== Some(io::ErrorKind::BrokenPipe)
	})
}

fn run() -> Result<()> {
	let (cli, matches) = Cli::parse_with_matches();
	if let Some(command) = cli.command {
		match command {
			Commands::Completions { shell } => {
				shell.generate_completions()?;
				return Ok(());
			}
			Commands::Langs => {
				let mut stdout = io::stdout();
				let terminal_width = terminal_size().map_or(80, |(w, _)| usize::from(w.0));
				langs::print_all_languages(&mut stdout, terminal_width)?;
				stdout.flush()?;
				return Ok(());
			}
			Commands::Init { output, force } => {
				init::run_init(output, force)?;
				return Ok(());
			}
		}
	}
	let analyze = &cli.analyze;
	let config = if let Some(ref config_path) = analyze.config {
		Config::from_file(config_path)?
	} else {
		config_discovery::load_default()?
	};
	let config = cli::merge_config(config, analyze, &matches)?;
	ensure!(config.path.exists(), "Path `{}` not found", config.path.display());
	let analyzer_config: AnalyzerConfig = (&config).into();
	let analyzer = CodeAnalyzer::new(&config.path, analyzer_config);
	let results = analyzer.analyze()?;
	let view_options: ViewOptions = (&config).into();
	let mut stdout = io::stdout();
	display::write_output(config.display.output, &results, &config.path, view_options, &mut stdout)?;
	stdout.flush()?;
	if config.analysis.fail_on_error && results.skipped_entries() > 0 {
		return Err(anyhow!("Skipped {} entries due to errors", results.skipped_entries()));
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use std::io;

	use anyhow::Context as _;

	use super::is_broken_pipe;

	#[test]
	fn broken_pipe_is_recognized_through_wrapping() {
		let pipe = || io::Error::from(io::ErrorKind::BrokenPipe);
		assert!(is_broken_pipe(&pipe().into()));
		assert!(is_broken_pipe(&Err::<(), _>(pipe()).context("writing report").unwrap_err()));
		assert!(is_broken_pipe(&serde_json::Error::io(pipe()).into()));
		assert!(!is_broken_pipe(&io::Error::from(io::ErrorKind::NotFound).into()));
		assert!(!is_broken_pipe(&anyhow::anyhow!("Skipped 3 entries due to errors")));
	}
}
