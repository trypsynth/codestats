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
