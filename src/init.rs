use std::{
	fs,
	path::{Path, PathBuf},
};

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

# Indentation style: \"tab\" or a number 1-8 for spaces
# indent = \"tab\"
";

pub fn run_init(output: Option<PathBuf>, force: bool) -> Result<()> {
	let path = output.unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH));
	if !force && Path::new(&path).exists() {
		bail!("File `{}` already exists. Use --force to overwrite.", path.display());
	}
	fs::write(&path, DEFAULT_CONFIG_TEMPLATE)?;
	println!("Created {}", path.display());
	Ok(())
}

#[cfg(test)]
mod tests {
	use std::{
		fs,
		time::{SystemTime, UNIX_EPOCH},
	};

	use super::*;

	fn unique_temp_dir(label: &str) -> PathBuf {
		let unique = SystemTime::now().duration_since(UNIX_EPOCH).expect("system time is set").as_nanos();
		let dir = std::env::temp_dir().join(format!("codestats_init_test_{}_{}_{label}", std::process::id(), unique,));
		fs::create_dir_all(&dir).expect("create temp dir");
		dir
	}

	#[test]
	fn default_config_path_constant() {
		assert_eq!(DEFAULT_CONFIG_PATH, ".codestats.toml");
	}

	#[test]
	fn run_init_creates_file_at_specified_path() {
		let dir = unique_temp_dir("specified_path");
		let file = dir.join("custom.toml");

		run_init(Some(file.clone()), false).expect("run_init should succeed");

		assert!(file.exists(), "file should be created at the specified path");
	}

	#[test]
	fn run_init_fails_when_file_exists_without_force() {
		let dir = unique_temp_dir("no_force");
		let file = dir.join("existing.toml");
		fs::write(&file, "old content").expect("seed file");

		let result = run_init(Some(file.clone()), false);

		assert!(result.is_err(), "should fail when file exists and force is false");
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("already exists"), "error should mention 'already exists', got: {err_msg}");
	}

	#[test]
	fn run_init_overwrites_existing_file_with_force() {
		let dir = unique_temp_dir("force");
		let file = dir.join("overwrite.toml");
		fs::write(&file, "old content").expect("seed file");

		run_init(Some(file.clone()), true).expect("run_init with force should succeed");

		let content = fs::read_to_string(&file).expect("read written file");
		assert_ne!(content, "old content", "file should be overwritten");
		assert!(content.contains("[analysis]"), "overwritten file should contain config template");
	}

	#[test]
	fn written_file_contains_expected_config_sections() {
		let dir = unique_temp_dir("content_check");
		let file = dir.join("check.toml");

		run_init(Some(file.clone()), false).expect("run_init should succeed");

		let content = fs::read_to_string(&file).expect("read written file");
		assert!(content.contains("# Codestats configuration file"), "should contain header comment");
		assert!(content.contains("[analysis]"), "should contain [analysis] section");
		assert!(content.contains("[display]"), "should contain [display] section");
		assert!(content.contains("# verbose = false"), "should contain verbose option");
		assert!(content.contains("# sort_by = \"lines\""), "should contain sort_by option");
		assert!(content.contains("# indent = \"tab\""), "should contain indent option");
	}
}
