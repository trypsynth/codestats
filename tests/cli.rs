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
