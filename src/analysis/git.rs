//! Counting the files git tracks, in the working tree (`--git`) or in a revision (`cs diff`).

use std::{
	borrow::Cow,
	collections::HashMap,
	io::{BufRead as _, BufReader, ErrorKind, Read as _, Write as _},
	path::{Component, Path, PathBuf},
	process::{Child, ChildStdin, ChildStdout, Command, Stdio},
	sync::atomic::{AtomicUsize, Ordering},
	thread,
};

use anyhow::{Context as _, Result, ensure};
use ignore::overrides::{Override, OverrideBuilder};

use super::{
	pipeline::{self, LanguageFilter},
	stats::AnalysisResults,
};
use crate::config::AnalysisConfig;

/// The exclusions a walk applies as overrides: generated files unless they're included, and `--exclude` patterns.
pub(super) fn exclusions(root: &Path, config: &AnalysisConfig) -> Result<Option<Override>> {
	if config.exclude_patterns.is_empty() && config.include_generated {
		return Ok(None);
	}
	let mut builder = OverrideBuilder::new(root);
	if !config.include_generated {
		for pattern in super::generated::PATTERNS {
			builder.add(&format!("!{pattern}"))?;
		}
	}
	for pattern in &config.exclude_patterns {
		// OverrideBuilder treats patterns without '!' as include rules, so we invert to enforce exclusion.
		let glob: Cow<'_, str> =
			if pattern.starts_with('!') { Cow::Borrowed(pattern.as_str()) } else { Cow::Owned(format!("!{pattern}")) };
		builder.add(&glob)?;
	}
	Ok(Some(builder.build()?))
}

fn git(dir: &Path, args: &[&str]) -> Result<Vec<u8>> {
	let output = Command::new("git").arg("-C").arg(dir).args(args).output().context("git isn't installed")?;
	ensure!(
		output.status.success(),
		"{}",
		String::from_utf8_lossy(&output.stderr).trim().trim_start_matches("fatal: ")
	);
	Ok(output.stdout)
}

/// The directory to run git in for an analysis root, and the pathspec that selects the root within it.
fn locate(root: &Path) -> (PathBuf, PathBuf) {
	if root.is_file() {
		let dir = root.parent().filter(|dir| !dir.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
		(dir.to_path_buf(), root.file_name().map_or_else(PathBuf::new, PathBuf::from))
	} else {
		(root.to_path_buf(), PathBuf::from("."))
	}
}

/// A path as git prints it with `-z`: raw bytes on Unix, UTF-8 elsewhere.
#[cfg(unix)]
fn path_from_git(bytes: &[u8]) -> PathBuf {
	use std::os::unix::ffi::OsStrExt as _;
	PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
}

#[cfg(not(unix))]
fn path_from_git(bytes: &[u8]) -> PathBuf {
	PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}

/// Which version of the tracked files to count.
pub enum Snapshot<'a> {
	/// The current contents of the tracked files on disk, and optionally of those tracked in submodules.
	WorkingTree { submodules: bool },
	/// The files as they are in a commit, tag or other revision.
	Revision(&'a str),
}

/// A tracked file: its path relative to the directory git ran in, and its object id.
struct Entry<'a> {
	path: PathBuf,
	object: &'a [u8],
}

/// Count the files git tracks under `root` in `snapshot`, with the walk's filters: exclusions, hidden files, depth and languages. `.gitignore` doesn't apply, since a tracked file is part of the repository whatever its patterns say. Symlinks aren't counted, nor are submodules unless asked for, since a revision's submodule contents aren't in this repository.
///
/// # Errors
///
/// Returns an error if git isn't installed, `root` isn't in a repository, or the revision doesn't exist.
pub fn count_snapshot(
	root: &Path,
	snapshot: &Snapshot,
	config: &AnalysisConfig,
	collect_details: bool,
) -> Result<AnalysisResults> {
	let (dir, pathspec) = locate(root);
	let pathspec = pathspec.to_string_lossy();
	let listing = match snapshot {
		Snapshot::WorkingTree { submodules } => {
			let recurse = if *submodules { "--recurse-submodules" } else { "--no-recurse-submodules" };
			git(&dir, &["ls-files", "-s", "-z", recurse, "--", &pathspec])?
		}
		Snapshot::Revision(revision) => git(&dir, &["ls-tree", "-r", "-z", revision, "--", &pathspec])?,
	};
	let filter = PathFilter::new(&dir, config, root.is_file())?;
	let languages = LanguageFilter { include: &config.include_languages, exclude: &config.exclude_languages };
	let records: Vec<&[u8]> = listing.split(|&b| b == 0).filter(|record| !record.is_empty()).collect();
	// Workers take batches from a shared counter, so they stay evenly loaded, and each batch is a run of nearby paths.
	let next = AtomicUsize::new(0);
	let threads = thread::available_parallelism().map_or(4, usize::from).min(records.len().div_ceil(BATCH)).max(1);
	let partials = thread::scope(|scope| {
		let workers: Vec<_> = (0..threads)
			.map(|_| {
				scope.spawn(|| -> Result<AnalysisResults> {
					let mut counter = Counter::new(snapshot, &dir, root, languages, collect_details)?;
					let mut excluded_dirs = HashMap::new();
					let mut batch = Vec::with_capacity(BATCH);
					loop {
						let start = next.fetch_add(BATCH, Ordering::Relaxed);
						if start >= records.len() {
							break;
						}
						batch.clear();
						for i in start..records.len().min(start + BATCH) {
							let (mode, object, path) = parse_record(records[i], snapshot)?;
							// Only regular files: not symlinks (120000) or submodules (160000). A conflicted file is listed once per stage.
							let repeated =
								i > 0 && parse_record(records[i - 1], snapshot).is_ok_and(|(_, _, last)| last == path);
							if !matches!(mode, b"100644" | b"100755") || repeated {
								continue;
							}
							let path = path_from_git(path);
							if filter.allows(&path, &mut excluded_dirs) && pipeline::could_count(&path, languages) {
								batch.push(Entry { path, object });
							}
						}
						counter.count(&batch)?;
					}
					counter.finish()
				})
			})
			.collect();
		workers.into_iter().map(|worker| worker.join().expect("counting thread panicked")).collect::<Result<Vec<_>>>()
	})?;
	Ok(partials.into_iter().fold(AnalysisResults::with_language_capacity(), |mut all, part| {
		all.merge(part);
		all
	}))
}

/// How many listed files a worker takes at a time.
const BATCH: usize = 256;

/// A record's mode, object id and path. `ls-files -s` prints "<mode> <object> <stage>\t<path>", `ls-tree` "<mode> <type> <object>\t<path>".
fn parse_record<'a>(record: &'a [u8], snapshot: &Snapshot) -> Result<(&'a [u8], &'a [u8], &'a [u8])> {
	let tab = memchr::memchr(b'\t', record).context("unexpected git output")?;
	let mut fields = record[..tab].split(|&b| b == b' ');
	let mode = fields.next().context("unexpected git output")?;
	if matches!(snapshot, Snapshot::Revision(_)) {
		fields.next();
	}
	let object = fields.next().context("unexpected git output")?;
	Ok((mode, object, &record[tab + 1..]))
}

/// A worker's way of counting the files it's given: from disk, or from git objects through its own `git cat-file --batch`, since git decompresses objects one at a time.
enum Counter<'a> {
	Files {
		dir: &'a Path,
		root: &'a Path,
		languages: LanguageFilter<'a>,
		collect_details: bool,
		results: AnalysisResults,
		skipped: u64,
	},
	Objects {
		languages: LanguageFilter<'a>,
		collect_details: bool,
		results: AnalysisResults,
		git: Child,
		requests: ChildStdin,
		output: BufReader<ChildStdout>,
		contents: Vec<u8>,
	},
}

impl<'a> Counter<'a> {
	fn new(
		snapshot: &Snapshot,
		dir: &'a Path,
		root: &'a Path,
		languages: LanguageFilter<'a>,
		collect_details: bool,
	) -> Result<Self> {
		let results = AnalysisResults::default();
		if matches!(snapshot, Snapshot::WorkingTree { .. }) {
			return Ok(Self::Files { dir, root, languages, collect_details, results, skipped: 0 });
		}
		let mut git = Command::new("git")
			.arg("-C")
			.arg(dir)
			.args(["cat-file", "--batch"])
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.spawn()
			.context("git isn't installed")?;
		let requests = git.stdin.take().context("no stdin for git")?;
		let output = BufReader::with_capacity(256 * 1024, git.stdout.take().context("no stdout for git")?);
		Ok(Self::Objects { languages, collect_details, results, git, requests, output, contents: Vec::new() })
	}

	fn count(&mut self, batch: &[Entry]) -> Result<()> {
		match self {
			Self::Files { dir, root, languages, collect_details, results, skipped } => {
				for entry in batch {
					// A tracked file deleted from disk isn't there to count; other failures count as skipped.
					if let Err(err) =
						pipeline::process_file(&dir.join(&entry.path), root, results, *collect_details, *languages)
						&& !err.chain().any(|cause| {
							cause.downcast_ref::<std::io::Error>().is_some_and(|e| e.kind() == ErrorKind::NotFound)
						}) {
						*skipped += 1;
					}
				}
			}
			Self::Objects { languages, collect_details, results, requests, output, contents, .. } => {
				// A batch's requests (256 object ids) fit in the pipe, so git can't block us by not reading them yet.
				let mut ids = Vec::with_capacity(batch.len() * 41);
				for entry in batch {
					ids.extend_from_slice(entry.object);
					ids.push(b'\n');
				}
				requests.write_all(&ids)?;
				requests.flush()?;
				let mut header = Vec::new();
				for entry in batch {
					header.clear();
					output.read_until(b'\n', &mut header)?;
					// "<object> blob <size>\n", then the contents and a newline.
					let size = std::str::from_utf8(&header)
						.ok()
						.and_then(|header| header.trim_end().rsplit(' ').next()?.parse::<u64>().ok())
						.with_context(|| {
							format!(
								"git couldn't read {}: {}",
								entry.path.display(),
								String::from_utf8_lossy(&header).trim()
							)
						})?;
					contents.clear();
					output.by_ref().take(size + 1).read_to_end(contents)?;
					contents.pop();
					if let Some((language, counts)) = pipeline::count_bytes(&entry.path, contents, *languages) {
						results.add_file(
							language,
							&counts,
							size,
							collect_details.then(|| entry.path.display().to_string()),
						);
					}
				}
			}
		}
		Ok(())
	}

	fn finish(self) -> Result<AnalysisResults> {
		match self {
			Self::Files { mut results, skipped, .. } => {
				results.set_skipped_entries(skipped);
				Ok(results)
			}
			Self::Objects { results, mut git, requests, .. } => {
				drop(requests);
				ensure!(git.wait()?.success(), "git cat-file failed");
				Ok(results)
			}
		}
	}
}

/// The filters a walk of the root applies, for a list of paths relative to it.
struct PathFilter {
	exclusions: Option<Override>,
	include_hidden: bool,
	max_depth: Option<usize>,
	/// A single file named as the root is counted whatever its name, as a walk would.
	single_file: bool,
}

impl PathFilter {
	fn new(dir: &Path, config: &AnalysisConfig, single_file: bool) -> Result<Self> {
		Ok(Self {
			exclusions: exclusions(dir, config)?,
			include_hidden: config.include_hidden,
			max_depth: config.max_depth,
			single_file,
		})
	}

	/// `excluded_dirs` remembers which directories are excluded, since nearby files share them.
	fn allows(&self, path: &Path, excluded_dirs: &mut HashMap<PathBuf, bool>) -> bool {
		if self.single_file {
			return true;
		}
		let components: Vec<_> = path.components().collect();
		if self.max_depth.is_some_and(|depth| components.len() > depth) {
			return false;
		}
		if !self.include_hidden
			&& components
				.iter()
				.any(|c| matches!(c, Component::Normal(name) if name.as_encoded_bytes().starts_with(b".")))
		{
			return false;
		}
		// A walk doesn't enter an excluded directory, so a path is out if any directory above it is.
		let Some(exclusions) = &self.exclusions else { return true };
		let Some((name, dirs)) = components.split_last() else { return true };
		let mut prefix = PathBuf::new();
		for component in dirs {
			prefix.push(component);
			if let Some(&excluded) = excluded_dirs.get(&prefix) {
				if excluded {
					return false;
				}
				continue;
			}
			let excluded = exclusions.matched(&prefix, true).is_ignore();
			excluded_dirs.insert(prefix.clone(), excluded);
			if excluded {
				return false;
			}
		}
		prefix.push(name);
		!exclusions.matched(&prefix, false).is_ignore()
	}
}
