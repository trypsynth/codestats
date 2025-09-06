mod analyzer;
mod cli;
mod comments;
mod langs;
mod stats;
mod utils;

use anyhow::{Result, ensure};
use clap::Parser;

use crate::{
	analyzer::{AnalyzerArgs, CodeAnalyzer},
	cli::{Cli, Commands},
};

fn main() -> Result<()> {
	let cli = Cli::parse();
	match cli.command {
		Commands::Langs => {
			langs::print_supported_languages();
			Ok(())
		}
		Commands::Analyze { path, verbose, no_gitignore, hidden, symlinks } => {
			ensure!(path.exists(), "Path `{}` not found", path.display());
			let analyzer_args =
				AnalyzerArgs::new(path).verbose(verbose).gitignore(!no_gitignore).hidden(hidden).symlinks(symlinks);
			let mut analyzer = CodeAnalyzer::new(analyzer_args);
			analyzer.analyze()?;
			analyzer.print_stats();
			Ok(())
		}
	}
}
