#![warn(clippy::all, clippy::cargo, clippy::nursery, clippy::pedantic)]

mod analysis;
mod cli;
mod display;
mod filters;
mod langs;
mod utils;

use std::io::{self, Write as _};

use anyhow::{Result, ensure};
use clap::Parser as _;
use cli::Cli;
use terminal_size::terminal_size;

use crate::{
	analysis::{AnalyzerConfig, CodeAnalyzer, DetailLevel, TraversalOptions},
	display::{ViewOptions, get_formatter},
};

fn main() -> Result<()> {
	let Cli {
		langs,
		path,
		verbose,
		no_gitignore,
		hidden,
		symlinks,
		number_style,
		size_style,
		percent_precision,
		language_sort,
		sort_direction,
		output,
	} = Cli::parse();
	if langs {
		let mut stdout = io::stdout();
		let terminal_width = terminal_size().map_or(80, |(w, _)| usize::from(w.0));
		langs::print_all_languages(&mut stdout, terminal_width)?;
		stdout.flush()?;
		return Ok(());
	}
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
	let view_options =
		ViewOptions { number_style, size_style, percent_precision, language_sort_key: language_sort, sort_direction };
	let formatter = get_formatter(output);
	let mut stdout = io::stdout();
	formatter.write_output(&results, &path, verbose, view_options, &mut stdout)?;
	stdout.flush()?;
	Ok(())
}
