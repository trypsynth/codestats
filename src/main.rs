pub(crate) mod analyzer;
pub(crate) mod cli;
pub(crate) mod comments;
pub(crate) mod langs;
pub(crate) mod stats;
pub(crate) mod utils;

use anyhow::{Result, ensure};

use crate::{
	analyzer::{AnalyzerArgs, CodeAnalyzer},
	cli::Commands,
};

/// Codestats entrypoint.
pub(crate) fn main() -> Result<()> {
	let cli = cli::parse_cli();
	match cli.command {
		Commands::Langs => {
			langs::print_supported_languages();
			Ok(())
		}
		Commands::Analyze { path, verbose, no_gitignore, hidden, symlinks } => {
			ensure!(path.exists(), "Path `{}` not found", path.display());
			let analyzer_args = AnalyzerArgs { path, verbose, gitignore: !no_gitignore, hidden, symlinks };
			let mut analyzer = CodeAnalyzer::new(analyzer_args);
			analyzer.analyze()?;
			analyzer.print_stats();
			Ok(())
		}
	}
}
