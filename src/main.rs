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

use analysis::{CodeAnalyzer, Snapshot, count_snapshot};
use anyhow::{Context as _, Result, anyhow, ensure};
use clap::ArgMatches;
use cli::{Cli, Commands, DiffArgs, SourceArgs};
use config::{AnalyzerConfig, Config};
use display::{Versions, ViewOptions};
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
			Commands::Diff(args) => {
				let matches = matches.subcommand_matches("diff").expect("clap matched the diff subcommand");
				return run_diff(&args, matches);
			}
		}
	}
	let config = cli::merge_config(base_config(&cli.analyze.source)?, &cli.analyze, &matches)?;
	ensure_path(&config)?;
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

/// The config file given with `-c`, or else the one discovered.
fn base_config(source: &SourceArgs) -> Result<Config> {
	source.config.as_ref().map_or_else(config_discovery::load_default, Config::from_file)
}

fn ensure_path(config: &Config) -> Result<()> {
	ensure!(config.path.exists(), "Path `{}` not found", config.path.display());
	Ok(())
}

fn run_diff(args: &DiffArgs, matches: &ArgMatches) -> Result<()> {
	let config = cli::merge_diff_config(base_config(&args.source)?, args, matches)?;
	ensure_path(&config)?;
	let from = count_snapshot(&config.path, &Snapshot::Revision(&args.from), &config.analysis, false)
		.with_context(|| format!("couldn't read revision `{}`", args.from))?;
	let to = match &args.to {
		Some(revision) => count_snapshot(&config.path, &Snapshot::Revision(revision), &config.analysis, false)
			.with_context(|| format!("couldn't read revision `{revision}`"))?,
		// The working tree's submodules are left out, as a revision's can't be read.
		None => count_snapshot(&config.path, &Snapshot::WorkingTree { submodules: false }, &config.analysis, false)?,
	};
	let versions = Versions { from_name: &args.from, to_name: args.to.as_deref(), from: &from, to: &to };
	let mut stdout = io::stdout();
	display::write_diff(config.display.output, &versions, &config.path, (&config).into(), &mut stdout)?;
	stdout.flush()?;
	let skipped = from.skipped_entries() + to.skipped_entries();
	if config.analysis.fail_on_error && skipped > 0 {
		return Err(anyhow!("Skipped {skipped} entries due to errors"));
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
