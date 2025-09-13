#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use anyhow::{Result, ensure};
use clap::Parser;
use codestats::{
	AnalysisOptions, CodeAnalyzer, ResultFormatter,
	cli::{Cli, Commands},
	language,
};

fn main() -> Result<()> {
	let cli = Cli::parse();
	match cli.command {
		Commands::Langs => {
			language::print_all_languages();
			Ok(())
		}
		Commands::Analyze { path, verbose, no_gitignore, hidden, symlinks } => {
			ensure!(path.exists(), "Path `{}` not found", path.display());
			let options = AnalysisOptions::new(path.clone())
				.verbose(verbose)
				.respect_gitignore(!no_gitignore)
				.include_hidden(hidden)
				.follow_symlinks(symlinks);
			let analyzer = CodeAnalyzer::new(options);
			let results = analyzer.analyze()?;
			ResultFormatter::print_summary(&results, &path, verbose);
			Ok(())
		}
	}
}
