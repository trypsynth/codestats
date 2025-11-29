#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

mod analysis;
mod cli;
mod display;
mod langs;
mod utils;

use std::io::{self, Write};

use anyhow::{Result, ensure};
use clap::Parser;
use cli::{Cli, Commands};

use crate::{
	analysis::{AnalyzerConfig, CodeAnalyzer, DetailLevel, TraversalOptions},
	display::get_formatter,
};

fn main() -> Result<()> {
	let cli = Cli::parse();
	match cli.command {
		Commands::Langs => {
			let mut stdout = io::stdout();
			langs::print_all_languages(&mut stdout)?;
			stdout.flush()?;
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
			let mut stdout = io::stdout();
			formatter.write_output(&results, &path, verbose, &mut stdout)?;
			stdout.flush()?;
			Ok(())
		}
	}
}
