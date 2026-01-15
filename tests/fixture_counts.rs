use std::{
	collections::HashMap,
	fs,
	path::{Path, PathBuf},
	process::Command,
};

use serde::Deserialize;

#[derive(Debug, PartialEq, Eq)]
struct ExpectedCounts {
	total: u64,
	code: u64,
	comment: u64,
	blank: u64,
	shebang: u64,
}

#[derive(Debug, Deserialize)]
struct AnalysisOutput {
	languages: Vec<LanguageOutput>,
}

#[derive(Debug, Deserialize)]
struct LanguageOutput {
	files_detail: Option<Vec<FileOutput>>,
}

#[derive(Debug, Deserialize)]
struct FileOutput {
	path: String,
	total_lines: u64,
	code_lines: u64,
	comment_lines: u64,
	blank_lines: u64,
	shebang_lines: u64,
}

#[test]
fn fixtures_match_expected_counts() {
	let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
	let fixtures_root = manifest_dir.join("tests/fixtures");
	let fixtures = collect_fixtures(&fixtures_root);
	assert!(!fixtures.is_empty(), "Add at least one fixture under {}", fixtures_root.display());
	let binary = env!("CARGO_BIN_EXE_cs");
	let output = Command::new(binary)
		.args([fixtures_root.to_str().expect("Non-UTF-8 fixtures root"), "-o", "json", "-v"])
		.output()
		.unwrap_or_else(|err| panic!("Failed to run codestats for {}: {err}", fixtures_root.display()));
	assert!(
		output.status.success(),
		"codestats run failed for {}\nstatus: {:?}\nstderr: {}",
		fixtures_root.display(),
		output.status.code(),
		String::from_utf8_lossy(&output.stderr),
	);
	let analysis: AnalysisOutput = serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
		panic!(
			"Failed to parse JSON output for {}: {err}\nstdout: {}\nstderr: {}",
			fixtures_root.display(),
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr)
		)
	});
	let file_map = build_file_map(&analysis);
	for fixture in fixtures {
		let expected = parse_expectations(&fixture);
		let normalized = normalize_path(&fixture);
		let actual =
			file_map.get(&normalized).unwrap_or_else(|| panic!("Missing file detail for {}", fixture.display()));
		assert_eq!(expected.total, actual.total, "total lines mismatch for {}", fixture.display());
		assert_eq!(expected.code, actual.code, "code lines mismatch for {}", fixture.display());
		assert_eq!(expected.comment, actual.comment, "comment lines mismatch for {}", fixture.display());
		assert_eq!(expected.blank, actual.blank, "blank lines mismatch for {}", fixture.display());
		assert_eq!(expected.shebang, actual.shebang, "shebang lines mismatch for {}", fixture.display());
	}
}

fn collect_fixtures(root: &Path) -> Vec<PathBuf> {
	let mut files = Vec::new();
	let mut stack = vec![root.to_path_buf()];
	while let Some(path) = stack.pop() {
		for entry in fs::read_dir(&path).unwrap_or_else(|err| panic!("Failed to read {}: {err}", path.display())) {
			let entry = entry.unwrap();
			let file_type = entry.file_type().unwrap();
			let entry_path = entry.path();
			if file_type.is_dir() {
				stack.push(entry_path);
			} else if file_type.is_file() {
				files.push(entry_path);
			}
		}
	}
	files
}

fn build_file_map(analysis: &AnalysisOutput) -> HashMap<PathBuf, ExpectedCounts> {
	let mut map = HashMap::new();
	for language in &analysis.languages {
		let Some(files) = &language.files_detail else { continue };
		for file in files {
			let path = normalize_path(&file.path);
			map.insert(
				path,
				ExpectedCounts {
					total: file.total_lines,
					code: file.code_lines,
					comment: file.comment_lines,
					blank: file.blank_lines,
					shebang: file.shebang_lines,
				},
			);
		}
	}
	map
}

fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
	let path = path.as_ref();
	// Canonicalize for stable fixture matching across platforms and output formats.
	fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn parse_expectations(path: &Path) -> ExpectedCounts {
	let contents =
		fs::read_to_string(path).unwrap_or_else(|err| panic!("Failed to read fixture {}: {err}", path.display()));
	for line in contents.lines() {
		let trimmed = line.trim_start();
		if trimmed.starts_with("#!") || trimmed.is_empty() {
			continue;
		}
		// Expect the first meaningful line to contain "expect: total=... code=... comment=... blank=... shebang=...".
		if let Some(expectation) = parse_expectation_line(line) {
			return expectation;
		}
		break;
	}
	panic!(
		"Fixture {} must start with an expectation comment (optionally after a shebang or blank line)",
		path.display()
	);
}

fn parse_expectation_line(line: &str) -> Option<ExpectedCounts> {
	let trimmed = line.trim_start();
	let meaningful = trimmed.trim_start_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_');
	let rest = meaningful.strip_prefix("expect:")?.trim();
	let rest = rest.trim_end_matches(|c: char| !c.is_ascii_alphanumeric() && c != '_');
	let mut counts = ExpectedCounts { total: 0, code: 0, comment: 0, blank: 0, shebang: 0 };
	let mut seen_mask = 0u8;
	for token in rest.split_whitespace() {
		let (key, value) = token.split_once('=')?;
		let parsed: u64 = value.parse().ok()?;
		match key.to_ascii_lowercase().as_str() {
			"total" => {
				counts.total = parsed;
				seen_mask |= 1 << 0;
			}
			"code" => {
				counts.code = parsed;
				seen_mask |= 1 << 1;
			}
			"comment" | "comments" => {
				counts.comment = parsed;
				seen_mask |= 1 << 2;
			}
			"blank" | "blanks" => {
				counts.blank = parsed;
				seen_mask |= 1 << 3;
			}
			"shebang" | "shebangs" => {
				counts.shebang = parsed;
				seen_mask |= 1 << 4;
			}
			_ => {}
		}
	}
	if seen_mask == 0b11111 { Some(counts) } else { None }
}
