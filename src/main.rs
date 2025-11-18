#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

mod cli;

use anyhow::{Result, ensure};
use clap::Parser;
use cli::{Cli, Commands};
use codestats::{AnalyzerConfig, CodeAnalyzer, DetailLevel, TraversalOptions, get_formatter, langs};

fn main() -> Result<()> {
	let cli = Cli::parse();
	match cli.command {
		Commands::Langs => {
			langs::print_all_languages();
			Ok(())
		}
		Commands::Analyze { path, verbose, no_gitignore, hidden, symlinks, output } => {
			ensure!(path.exists(), "Path `{}` not found", path.display());
			if path.is_file() {
				ensure!(path.metadata().is_ok(), "Cannot read file metadata for `{}`", path.display());
			}
			let config = AnalyzerConfig {
				verbose,
				traversal: TraversalOptions {
					respect_gitignore: !no_gitignore,
					include_hidden: hidden,
					follow_symlinks: symlinks,
				},
				detail_level: if verbose { DetailLevel::PerFile } else { DetailLevel::Summary },
			};
			let analyzer = CodeAnalyzer::new(path.clone(), config);
			let results = analyzer.analyze()?;
			let formatter = get_formatter(output);
			let output_text = formatter.format(&results, &path, verbose)?;
			print!("{output_text}");
			Ok(())
		}
	}
}
