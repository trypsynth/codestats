use std::{
	fs,
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
				lang["files"].as_array().unwrap().iter().map(|file| file["path"].as_str().unwrap().to_owned())
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
