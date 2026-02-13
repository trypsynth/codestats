use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

const DEFAULT_CONFIG_PATH: &str = ".codestats.toml";

const DEFAULT_CONFIG_TEMPLATE: &str = "\
# Codestats configuration file
# See: cs --help for details on each option

[analysis]
# Show per-file detail in output
# verbose = false

# Respect .gitignore rules when scanning
# respect_gitignore = true

# Include hidden files and directories
# include_hidden = false

# Follow symbolic links
# follow_symlinks = false

# Glob patterns to exclude (can specify multiple)
# exclude_patterns = [\"*.tmp\", \"node_modules/*\"]

# Only analyze these languages (case-insensitive)
# include_languages = []

# Skip these languages (case-insensitive, conflicts with include_languages)
# exclude_languages = []

# Exit with non-zero status if any files are skipped due to errors
# fail_on_error = false

[display]
# Number formatting: plain, comma, underscore, space
# number_style = \"plain\"

# Size units: binary (KiB) or decimal (KB)
# size_units = \"binary\"

# Decimal places for percentages (0-6)
# precision = 1

# Sort by: lines, code, comments, blanks, files, size, name
# sort_by = \"lines\"

# Sort direction: asc, desc
# sort_direction = \"desc\"

# Output format: human, json, json-compact, csv, tsv, markdown, html
# output = \"human\"
";

pub fn run_init(output: Option<PathBuf>, force: bool) -> Result<()> {
	let path = output.unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH));

	if !force && Path::new(&path).exists() {
		bail!("File `{}` already exists. Use --force to overwrite.", path.display());
	}

	std::fs::write(&path, DEFAULT_CONFIG_TEMPLATE)?;
	println!("Created {}", path.display());
	Ok(())
}
