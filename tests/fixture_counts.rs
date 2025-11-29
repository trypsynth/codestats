use std::{
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
	summary: AnalysisSummary,
}

#[derive(Debug, Deserialize)]
struct AnalysisSummary {
	total_lines: u64,
	total_code_lines: u64,
	total_comment_lines: u64,
	total_blank_lines: u64,
	total_shebang_lines: u64,
}

#[test]
fn fixtures_match_expected_counts() {
	let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
	let fixtures_root = manifest_dir.join("tests/fixtures");
	let fixtures = collect_fixtures(&fixtures_root);
	assert!(
		!fixtures.is_empty(),
		"Add at least one fixture under {}",
		fixtures_root.display()
	);
	let binary = env!("CARGO_BIN_EXE_codestats");
	for fixture in fixtures {
		let expected = parse_expectations(&fixture);
		let output = Command::new(binary)
			.args(["analyze", fixture.to_str().expect("Non-UTF-8 fixture path"), "-o", "json"])
			.output()
			.unwrap_or_else(|err| panic!("Failed to run codestats for {}: {err}", fixture.display()));
		assert!(
			output.status.success(),
			"codestats analyze failed for {}\nstatus: {:?}\nstderr: {}",
			fixture.display(),
			output.status.code(),
			String::from_utf8_lossy(&output.stderr),
		);
		let analysis: AnalysisOutput = serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
			panic!(
				"Failed to parse JSON output for {}: {err}\nstdout: {}\nstderr: {}",
				fixture.display(),
				String::from_utf8_lossy(&output.stdout),
				String::from_utf8_lossy(&output.stderr)
			)
		});
		let summary = analysis.summary;
		assert_eq!(expected.total, summary.total_lines, "total lines mismatch for {}", fixture.display());
		assert_eq!(expected.code, summary.total_code_lines, "code lines mismatch for {}", fixture.display());
		assert_eq!(expected.comment, summary.total_comment_lines, "comment lines mismatch for {}", fixture.display());
		assert_eq!(expected.blank, summary.total_blank_lines, "blank lines mismatch for {}", fixture.display());
		assert_eq!(expected.shebang, summary.total_shebang_lines, "shebang lines mismatch for {}", fixture.display());
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

fn parse_expectations(path: &Path) -> ExpectedCounts {
	let contents =
		fs::read_to_string(path).unwrap_or_else(|err| panic!("Failed to read fixture {}: {err}", path.display()));
	for line in contents.lines() {
		let trimmed = line.trim_start();
		if trimmed.starts_with("#!") || trimmed.is_empty() {
			continue;
		}
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
	let without_prefix = trimmed
		.strip_prefix("//")
		.or_else(|| trimmed.strip_prefix('#'))
		.or_else(|| trimmed.strip_prefix("--"))
		.or_else(|| trimmed.strip_prefix(';'))
		.or_else(|| trimmed.strip_prefix("\\"))
		.unwrap_or(trimmed)
		.trim();
	let rest = without_prefix.strip_prefix("expect:")?.trim();
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
