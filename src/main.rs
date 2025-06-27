pub mod analyzer;
pub mod cli;
pub mod comments;
pub mod langs;
pub mod stats;
pub mod utils;

use crate::analyzer::CodeAnalyzer;
use anyhow::{Result, ensure};

/// Codestats entrypoint.
pub(crate) fn main() -> Result<()> {
	let args = cli::parse_cli();
	ensure!(args.path.exists(), "Path `{}` not found", args.path.display());
	let mut analyzer = CodeAnalyzer::new(args);
	analyzer.analyze()?;
	analyzer.print_stats();
	Ok(())
}
