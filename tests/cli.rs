use std::{
	fs,
	io::Write,
	path::{Path, PathBuf},
	process::{Command, Stdio},
};

fn cs() -> Command {
	Command::new(env!("CARGO_BIN_EXE_cs"))
}

fn fixtures() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures")
}

/// A fresh, empty directory for one test under cargo's temporary directory.
fn scratch_dir(name: &str) -> PathBuf {
	let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
	let _ = fs::remove_dir_all(&dir);
	fs::create_dir_all(&dir).unwrap();
	dir
}

#[test]
fn closed_stdout_exits_quietly() {
	let mut child =
		cs().arg(fixtures()).args(["-v", "-o", "json"]).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
	// Close the reading end before codestats writes anything, as `cs | head -0` would.
	drop(child.stdout.take());
	let output = child.wait_with_output().unwrap();
	assert!(output.status.success(), "exit status {:?}", output.status);
	assert!(output.stderr.is_empty(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn by_dir_sorts_directories_by_file_count() {
	let root = scratch_dir("by_dir_sorts_directories_by_file_count");
	// `long` has the most lines, `many` the most files.
	fs::create_dir(root.join("long")).unwrap();
	fs::write(root.join("long").join("main.rs"), "fn main() {}\n".repeat(100)).unwrap();
	fs::create_dir(root.join("many")).unwrap();
	for name in ["a.rs", "b.rs", "c.rs"] {
		fs::write(root.join("many").join(name), "fn f() {}\n").unwrap();
	}
	let output = cs().arg(&root).args(["-D", "-s", "files", "-o", "csv"]).output().unwrap();
	assert!(output.status.success());
	let csv = String::from_utf8(output.stdout).unwrap();
	let dirs: Vec<_> = csv.lines().skip(1).map(|row| row.split(',').next().unwrap()).collect();
	assert_eq!(dirs, ["many", "long"]);
}

#[test]
fn invalid_value_names_the_option_and_suggests_one() {
	let output = cs().args(["-o", "xml"]).output().unwrap();
	assert!(!output.status.success());
	let stderr = String::from_utf8(output.stderr).unwrap();
	assert!(stderr.contains("'--output <OUTPUT>'"), "{stderr}");
	assert!(stderr.contains("possible values: human, json"), "{stderr}");
	assert!(stderr.contains("'html'"), "{stderr}");
}

#[test]
fn git_counts_only_tracked_files() {
	let root = scratch_dir("git_counts_only_tracked_files");
	let git =
		|args: &[&str]| assert!(Command::new("git").arg("-C").arg(&root).args(args).output().unwrap().status.success());
	git(&["init", "-q"]);
	fs::create_dir(root.join("src")).unwrap();
	fs::write(root.join("src").join("tracked.rs"), "fn main() {}\n").unwrap();
	fs::write(root.join("top.py"), "x = 1\n").unwrap();
	git(&["add", "src/tracked.rs", "top.py"]);
	fs::write(root.join("src").join("untracked.rs"), "fn f() {}\n").unwrap();
	let files = |args: &[&str]| {
		let output = cs().args(args).args(["-v", "-o", "json"]).output().unwrap();
		assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
		let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
		let mut paths: Vec<String> = report["languages"]
			.as_array()
			.unwrap()
			.iter()
			.flat_map(|lang| {
				lang["files"].as_array().unwrap().iter().map(|file| file["path"].as_str().unwrap().replace('\\', "/"))
			})
			.collect();
		paths.sort();
		paths
	};
	let root_arg = root.to_str().unwrap();
	assert_eq!(files(&[root_arg]), ["src/tracked.rs", "src/untracked.rs", "top.py"]);
	assert_eq!(files(&[root_arg, "--git"]), ["src/tracked.rs", "top.py"]);
	// What git tracks counts even if an ignore pattern matches it.
	fs::write(root.join(".gitignore"), "*.py\n").unwrap();
	assert_eq!(files(&[root_arg, "--git"]), ["src/tracked.rs", "top.py"]);
	// A subdirectory or single file as the root is resolved against the same repository.
	assert_eq!(files(&[root.join("src").to_str().unwrap(), "--git"]), ["tracked.rs"]);
	assert_eq!(files(&[root.join("src").join("untracked.rs").to_str().unwrap(), "--git"]), Vec::<String>::new());
}

#[test]
fn git_outside_a_repository_is_an_error() {
	let root = scratch_dir("git_outside_a_repository_is_an_error");
	fs::write(root.join("a.rs"), "fn main() {}\n").unwrap();
	let output = cs().arg(&root).arg("--git").env("GIT_CEILING_DIRECTORIES", root.parent().unwrap()).output().unwrap();
	assert!(!output.status.success());
	assert!(String::from_utf8(output.stderr).unwrap().contains("--git: not a git repository"));
}

/// A repository with two commits and uncommitted changes, for `cs diff`.
fn diff_repo(name: &str) -> PathBuf {
	let root = scratch_dir(name);
	let git = |args: &[&str]| {
		let output = Command::new("git")
			.arg("-C")
			.arg(&root)
			.args(["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"])
			.args(args)
			.output()
			.unwrap();
		assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	};
	git(&["init", "-q"]);
	fs::create_dir(root.join("src")).unwrap();
	fs::write(root.join("src").join("main.rs"), "// entry\nfn main() {}\n").unwrap();
	fs::write(root.join("tool.py"), "x = 1\n\ny = 2\n").unwrap();
	git(&["add", "."]);
	git(&["commit", "-qm", "one"]);
	git(&["tag", "one"]);
	fs::write(root.join("src").join("main.rs"), "// entry\nfn main() {\n\tlet x = 1;\n}\n").unwrap();
	fs::remove_file(root.join("tool.py")).unwrap();
	fs::write(root.join("src").join("lib.go"), "package lib\n").unwrap();
	git(&["add", "-A"]);
	git(&["commit", "-qm", "two"]);
	// Uncommitted: an edit to a tracked file, and a file git doesn't track.
	fs::write(root.join("src").join("main.rs"), "// entry\nfn main() {\n\tlet x = 1;\n\tlet y = 2;\n}\n").unwrap();
	fs::write(root.join("src").join("scratch.rs"), "fn scratch() {}\n").unwrap();
	root
}

fn diff_json(root: &Path, args: &[&str]) -> serde_json::Value {
	let output = cs().arg("diff").args(args).arg(root).args(["-o", "json"]).output().unwrap();
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	serde_json::from_slice(&output.stdout).unwrap()
}

/// A language's (file count, total, code, comment, blank) on one side, or `None` if it isn't there.
fn side(report: &serde_json::Value, language: &str, which: &str) -> Option<(u64, u64, u64, u64, u64)> {
	let languages = report["languages"].as_array().unwrap();
	let side = &languages.iter().find(|l| l["name"] == language)?[which];
	(!side.is_null()).then(|| {
		let n = |v: &serde_json::Value| v.as_u64().unwrap();
		let lines = &side["lines"];
		(n(&side["file_count"]), n(&lines["total"]), n(&lines["code"]), n(&lines["comment"]), n(&lines["blank"]))
	})
}

#[test]
fn diff_between_revisions_shows_new_removed_and_changed_languages() {
	let root = diff_repo("diff_between_revisions");
	let report = diff_json(&root, &["one", "--to", "HEAD"]);
	assert_eq!(report["from"], "one");
	assert_eq!(report["to"], "HEAD");
	assert_eq!(side(&report, "Rust", "from"), Some((1, 2, 1, 1, 0)));
	assert_eq!(side(&report, "Rust", "to"), Some((1, 4, 3, 1, 0)));
	assert_eq!(side(&report, "Python", "from"), Some((1, 3, 2, 0, 1)));
	assert_eq!(side(&report, "Python", "to"), None);
	assert_eq!(side(&report, "Go", "from"), None);
	assert_eq!(side(&report, "Go", "to"), Some((1, 1, 1, 0, 0)));
	assert_eq!(report["totals"]["from"]["file_count"], 2);
	assert_eq!(report["totals"]["to"]["file_count"], 2);
}

#[test]
fn diff_to_the_working_tree_counts_edits_but_not_untracked_files() {
	let root = diff_repo("diff_to_working_tree");
	let report = diff_json(&root, &["HEAD"]);
	assert!(report["to"].is_null());
	assert_eq!(side(&report, "Rust", "from"), Some((1, 4, 3, 1, 0)));
	// main.rs gained a line; scratch.rs isn't tracked.
	assert_eq!(side(&report, "Rust", "to"), Some((1, 5, 4, 1, 0)));
	assert_eq!(side(&report, "Go", "from"), side(&report, "Go", "to"));
}

#[test]
fn diff_applies_filters_and_paths_to_both_sides() {
	let root = diff_repo("diff_filters");
	let rust_only = diff_json(&root, &["one", "-L", "rust"]);
	let names: Vec<_> =
		rust_only["languages"].as_array().unwrap().iter().map(|l| l["name"].as_str().unwrap()).collect();
	assert_eq!(names, ["Rust"]);
	let output = cs().arg("diff").arg("one").arg(root.join("src")).args(["-o", "json"]).output().unwrap();
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	let src: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
	assert!(side(&src, "Python", "from").is_none(), "tool.py is outside src/");
	assert_eq!(side(&src, "Go", "to"), Some((1, 1, 1, 0, 0)));
}

#[test]
fn diff_reports_unknown_revisions_and_non_repositories() {
	let root = diff_repo("diff_errors");
	let output = cs().arg("diff").arg("no-such-revision").arg(&root).output().unwrap();
	assert!(!output.status.success());
	assert!(String::from_utf8(output.stderr).unwrap().contains("no-such-revision"));
	let plain = scratch_dir("diff_not_a_repository");
	let output = cs()
		.arg("diff")
		.arg("HEAD")
		.arg(&plain)
		.env("GIT_CEILING_DIRECTORIES", plain.parent().unwrap())
		.output()
		.unwrap();
	assert!(!output.status.success());
	assert!(String::from_utf8(output.stderr).unwrap().contains("not a git repository"));
}

#[test]
fn several_paths_are_counted_together_once() {
	let root = scratch_dir("several_paths_are_counted_together_once");
	for dir in ["a", "b"] {
		fs::create_dir(root.join(dir)).unwrap();
		fs::write(root.join(dir).join("main.rs"), "fn main() {}\n").unwrap();
	}
	// `a` given twice and `a/main.rs` inside it are counted once; files are told apart by the path they were reached by.
	let output = cs().current_dir(&root).args(["a", "b", "a", "a/main.rs", "-v", "-o", "csv"]).output().unwrap();
	assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
	let csv = String::from_utf8(output.stdout).unwrap();
	let mut files: Vec<_> =
		csv.lines().filter(|row| row.contains("main.rs")).map(|row| row.replace('\\', "/")).collect();
	files.sort_unstable();
	assert_eq!(files.len(), 2, "{csv}");
	assert!(files[0].starts_with("a/main.rs") && files[1].starts_with("b/main.rs"), "{csv}");
}

#[test]
fn files_marked_generated_are_skipped_unless_included() {
	let root = scratch_dir("files_marked_generated_are_skipped_unless_included");
	fs::write(root.join("main.go"), "package main\n\nfunc main() {}\n").unwrap();
	fs::write(
		root.join("kind_string.go"),
		"// Code generated by \"stringer -type=Kind\"; DO NOT EDIT.\n\npackage main\n",
	)
	.unwrap();
	let files = |extra: &[&str]| {
		let output = cs().current_dir(&root).args(["-o", "json"]).args(extra).output().unwrap();
		assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
		let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
		report["totals"]["file_count"].as_u64().unwrap()
	};
	assert_eq!(files(&[]), 1);
	assert_eq!(files(&["--include-generated"]), 2);
}

#[test]
fn gitattributes_mark_files_from_above_and_below() {
	let root = scratch_dir("gitattributes_mark_files_from_above_and_below");
	fs::create_dir(root.join(".git")).unwrap();
	for dir in ["src", "gen", "vendor", "src/keep"] {
		fs::create_dir_all(root.join(dir)).unwrap();
	}
	fs::write(
		root.join(".gitattributes"),
		"gen/** linguist-generated\nvendor/** linguist-vendored\n*.fixed linguist-language=Rust\n",
	)
	.unwrap();
	fs::write(root.join("src/keep/.gitattributes"), "*.lock -linguist-generated\n").unwrap();
	for file in ["src/main.rs", "gen/table.rs", "vendor/lib.rs", "tests.fixed"] {
		fs::write(root.join(file), "fn main() {}\n").unwrap();
	}
	fs::write(root.join("src/keep/Cargo.lock"), "version = 4\n").unwrap();
	let files = |dir: &str, extra: &[&str]| {
		let output = cs().current_dir(root.join(dir)).args(["-o", "json"]).args(extra).output().unwrap();
		assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
		let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
		report["totals"]["file_count"].as_u64().unwrap()
	};
	// main.rs, tests.fixed as Rust, and the lockfile its directory says isn't generated.
	assert_eq!(files(".", &[]), 3);
	// The root's rule reaches a count started inside `gen`.
	assert_eq!(files("gen", &[]), 0);
	assert_eq!(files(".", &["--include-generated"]), 4);
	// Without attributes: main.rs, gen/table.rs and vendor/lib.rs; the lockfile is generated by name and `.fixed` is no language.
	assert_eq!(files(".", &["--no-gitattributes"]), 3);
}

#[test]
fn paths_can_come_from_standard_input() {
	use std::io::Write as _;
	let root = scratch_dir("paths_can_come_from_standard_input");
	fs::create_dir_all(root.join("src/nested")).unwrap();
	fs::create_dir_all(root.join(".hidden")).unwrap();
	for file in ["src/a.rs", "src/nested/b.rs", "c.rs", ".hidden/d.rs", "unlisted.rs"] {
		fs::write(root.join(file), "fn main() {}\n").unwrap();
	}
	let count = |list: &[u8], extra: &[&str]| {
		let mut child = cs()
			.current_dir(&root)
			.args(["-", "-o", "json"])
			.args(extra)
			.stdin(std::process::Stdio::piped())
			.stdout(std::process::Stdio::piped())
			.spawn()
			.unwrap();
		child.stdin.take().unwrap().write_all(list).unwrap();
		let output = child.wait_with_output().unwrap();
		assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
		let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
		report["totals"]["file_count"].as_u64().unwrap()
	};
	// A listed directory is walked, a file counted once however often it's listed, and a hidden one left out as in a walk.
	assert_eq!(count(b"src\nc.rs\nc.rs\n.hidden/d.rs\n", &[]), 3);
	assert_eq!(count(b"c.rs\0src/a.rs\0", &[]), 2);
	assert_eq!(count(b"src\n", &["-e", "nested"]), 1);
	let output = cs().current_dir(&root).args(["-", "c.rs"]).output().unwrap();
	assert!(!output.status.success());
}

#[test]
fn a_name_only_codestats_knows_keeps_its_language() {
	let root = scratch_dir("a_name_only_codestats_knows_keeps_its_language");
	fs::write(root.join(".gitattributes"), "*.bgt linguist-language=AngelScript\n*.pp linguist-language=Rust\n")
		.unwrap();
	fs::write(root.join("game.bgt"), "void main() {}\n").unwrap();
	fs::write(root.join("pretty.pp"), "fn main() {}\n").unwrap();
	let output = cs().current_dir(&root).args(["-o", "json"]).output().unwrap();
	let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
	let mut names: Vec<&str> =
		report["languages"].as_array().unwrap().iter().map(|l| l["name"].as_str().unwrap()).collect();
	names.sort_unstable();
	// `.bgt` is only BGT, but `.pp` is Pascal or Puppet, so the attribute decides.
	assert_eq!(names, ["BGT", "Rust"]);
}

#[test]
fn ext_counts_only_those_extensions_and_git_internals_never_count() {
	let root = scratch_dir("ext_counts_only_those_extensions_and_git_internals_never_count");
	fs::create_dir_all(root.join(".git/hooks")).unwrap();
	fs::write(root.join(".git/hooks/pre-commit.sample"), "#!/bin/sh\nexit 0\n").unwrap();
	fs::write(root.join("main.rs"), "fn main() {}\n").unwrap();
	fs::write(root.join("Cargo.toml"), "[package]\n").unwrap();
	fs::write(root.join("build"), "#!/bin/sh\nmake\n").unwrap();
	let files = |extra: &[&str]| {
		let output = cs().current_dir(&root).args(["-o", "json", "--hidden"]).args(extra).output().unwrap();
		assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
		let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
		report["totals"]["file_count"].as_u64().unwrap()
	};
	// main.rs, Cargo.toml, and the `build` script, but nothing under `.git` even with `--hidden`.
	assert_eq!(files(&[]), 3);
	assert_eq!(files(&["--ext", "rs"]), 1);
	assert_eq!(files(&["--ext", ".RS,toml"]), 2);
}

#[test]
fn an_unknown_language_is_warned_about() {
	let root = scratch_dir("an_unknown_language_is_warned_about");
	fs::write(root.join("main.rs"), "fn main() {}\n").unwrap();
	let warnings = |flag: &str, name: &str| {
		let output = cs().arg(&root).args([flag, name, "-q"]).output().unwrap();
		assert!(output.status.success());
		String::from_utf8(output.stderr).unwrap()
	};
	assert!(warnings("--lang", "rust,nosuch").contains("no language is named `nosuch`"));
	assert!(warnings("--exclude-lang", "nosuch").contains("no language is named `nosuch`"));
	assert_eq!(warnings("--lang", "RUST"), "");
}

#[cfg(unix)]
#[test]
fn markdown_rows_survive_a_line_break_in_a_file_name() {
	let root = scratch_dir("markdown_rows_survive_a_line_break_in_a_file_name");
	fs::write(root.join("a\nb.rs"), "fn main() {}\n").unwrap();
	let output = cs().arg(&root).args(["-v", "-o", "markdown"]).output().unwrap();
	let report = String::from_utf8(output.stdout).unwrap();
	assert!(report.contains("| a b.rs | 1 |"), "{report}");
}

#[test]
fn one_line_is_not_written_as_lines() {
	let root = scratch_dir("one_line_is_not_written_as_lines");
	fs::write(root.join("main.rs"), "fn main() {}\n").unwrap();
	let output = cs().arg(&root).arg("-v").output().unwrap();
	let report = String::from_utf8(output.stdout).unwrap();
	assert!(report.contains("main.rs: 1 line,"), "{report}");
	assert!(report.contains("Code: 1 line ("), "{report}");
}

#[test]
fn hidden_languages_are_mentioned_in_every_report() {
	let root = scratch_dir("hidden_languages_are_mentioned_in_every_report");
	fs::write(root.join("main.rs"), "fn main() {}\n").unwrap();
	fs::write(root.join("main.py"), "x = 1\n").unwrap();
	let report = |args: &[&str]| {
		let output = cs().arg(&root).args(args).output().unwrap();
		String::from_utf8(output.stdout).unwrap()
	};
	for format in ["markdown", "html"] {
		assert!(report(&["-o", format, "-t", "1"]).contains("1 language not shown."), "{format}");
		let all_hidden = report(&["-o", format, "--min-lines", "9"]);
		assert!(all_hidden.contains("2 languages not shown."), "{all_hidden}");
		assert!(!all_hidden.contains("No recognized"), "{all_hidden}");
	}
	let all_hidden = report(&["--min-lines", "9"]);
	assert!(all_hidden.contains("(2 languages not shown)"), "{all_hidden}");
	assert!(!all_hidden.contains("No recognized"), "{all_hidden}");
	assert!(report(&["--by-dir", "-o", "markdown", "--min-lines", "9"]).contains("1 directory not shown."));
}

#[test]
fn paths_from_standard_input_are_titled_in_words() {
	let root = scratch_dir("paths_from_standard_input_are_titled_in_words");
	fs::write(root.join("main.rs"), "fn main() {}\n").unwrap();
	let report = |format: &str| {
		let mut child = cs()
			.current_dir(&root)
			.args(["-", "-q", "-o", format])
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.spawn()
			.unwrap();
		child.stdin.take().unwrap().write_all(b"main.rs\n").unwrap();
		String::from_utf8(child.wait_with_output().unwrap().stdout).unwrap()
	};
	assert!(report("human").starts_with("Codestats for standard input:"));
	assert!(report("markdown").contains("Codestats for `standard input`"));
	assert!(report("html").contains("Codestats for standard input"));
	assert!(report("json-compact").contains(r#""paths":["-"]"#));
}

#[test]
fn an_xml_declaration_outweighs_a_code_extension() {
	let root = scratch_dir("an_xml_declaration_outweighs_a_code_extension");
	fs::write(root.join("app_en.ts"), "<?xml version=\"1.0\"?>\n<!-- note -->\n<TS/>\n").unwrap();
	fs::write(root.join("app.ts"), "// note\nlet a = 1;\n").unwrap();
	let output = cs().arg(&root).args(["-o", "json-compact"]).output().unwrap();
	let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
	let languages: Vec<&str> =
		report["languages"].as_array().unwrap().iter().map(|l| l["name"].as_str().unwrap()).collect();
	assert_eq!(languages.len(), 2, "{languages:?}");
	assert!(languages.contains(&"XML") && languages.contains(&"TypeScript"), "{languages:?}");
}

#[test]
fn no_config_ignores_the_configuration_in_the_working_directory() {
	let root = scratch_dir("no_config_ignores_the_configuration_in_the_working_directory");
	fs::write(root.join("main.rs"), "fn main() {}\n").unwrap();
	fs::write(root.join("main.py"), "print(1)\n").unwrap();
	fs::write(root.join(".codestats.toml"), "[analysis]\ninclude_languages = [\"Rust\"]\n").unwrap();
	let files = |extra: &[&str]| {
		let output = cs().current_dir(&root).args(["-o", "json"]).args(extra).output().unwrap();
		assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
		serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["totals"]["file_count"].as_u64().unwrap()
	};
	assert_eq!(files(&[]), 1);
	assert_eq!(files(&["--no-config"]), 2);
	let both = cs().current_dir(&root).args(["--no-config", "--config", ".codestats.toml"]).output().unwrap();
	assert_eq!(both.status.code(), Some(2));
}
