pub(crate) mod analyzer;
pub(crate) mod cli;
pub(crate) mod comments;
pub(crate) mod langs;
pub(crate) mod stats;
pub(crate) mod utils;

use crate::{
	analyzer::{AnalyzerArgs, CodeAnalyzer},
	cli::Commands,
};
use anyhow::{Result, ensure};

/// Codestats entrypoint.
pub(crate) fn main() -> Result<()> {
	let cli = cli::parse_cli();
	match cli.command {
		Commands::Languages => {
			langs::print_supported_languages();
			Ok(())
		}
		Commands::Analyze { path, verbose, gitignore, hidden, symlinks } => {
			ensure!(path.exists(), "Path `{}` not found", path.display());
			let analyzer_args = AnalyzerArgs { path, verbose, gitignore, hidden, symlinks };
			let mut analyzer = CodeAnalyzer::new(analyzer_args);
			analyzer.analyze()?;
			analyzer.print_stats();
			Ok(())
		}
	}
}
