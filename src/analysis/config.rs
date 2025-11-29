/// Configuration that controls how [`CodeAnalyzer`] traverses the filesystem and how much information it gathers.
#[derive(Clone, Debug, Default)]
pub struct AnalyzerConfig {
	/// Emit additional progress messages and per-file diagnostics.
	pub verbose: bool,
	/// Controls what should be considered while walking directories.
	pub traversal: TraversalOptions,
	/// Select whether only aggregated totals or per-file data should be collected.
	pub detail_level: DetailLevel,
}

/// Options that influence how [`CodeAnalyzer`] traverses directories.
#[derive(Clone, Copy, Debug)]
pub struct TraversalOptions {
	/// Respect `.gitignore` files while walking.
	pub respect_gitignore: bool,
	/// Include hidden files and directories.
	pub include_hidden: bool,
	/// Follow symbolic links discovered during traversal.
	pub follow_symlinks: bool,
}

impl Default for TraversalOptions {
	fn default() -> Self {
		Self { respect_gitignore: true, include_hidden: false, follow_symlinks: false }
	}
}

/// Controls how much information is tracked for each file that matches the filters.
#[derive(Clone, Copy, Debug, Default)]
pub enum DetailLevel {
	/// Collect only aggregated totals per language.
	#[default]
	Summary,
	/// Collect aggregated totals plus detailed statistics for every file.
	PerFile,
}

impl DetailLevel {
	#[must_use]
	pub(crate) const fn collect_file_details(self) -> bool {
		matches!(self, Self::PerFile)
	}
}
