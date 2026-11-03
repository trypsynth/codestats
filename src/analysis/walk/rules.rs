//! Which ignore rules apply where in a walk: a chain from each directory up to the root, in which a directory with no rules of its own uses its parent's link, so most directories cost nothing here.

use std::{
	env,
	fs::{self, File, FileType},
	io::{BufRead, BufReader},
	path::{Path, PathBuf},
	sync::Arc,
};

use ignore::{
	Error, Match,
	gitignore::{Gitignore, GitignoreBuilder},
	overrides::Override,
};

const EXCLUDES_KEY: &[u8] = b"excludesfile";

/// What's true of the whole walk.
pub(super) struct Shared {
	root: PathBuf,
	skip_hidden: bool,
	respect_ignores: bool,
	overrides: Option<Override>,
	global: Gitignore,
	global_dir: PathBuf,
	/// The rules from the directories above the root, nearest first, and the root's absolute path. Left out if there are none.
	above_root: Option<(Vec<DirRules>, PathBuf)>,
}

impl Shared {
	/// Also returns any errors in the files of rules above the root.
	pub(super) fn new(
		root: &Path,
		skip_hidden: bool,
		respect_ignores: bool,
		overrides: Option<Override>,
	) -> (Self, Vec<Error>) {
		let mut errors = Vec::new();
		let ((global, global_dir), above_root) = if respect_ignores {
			(global_rules(), rules_above(root, &mut errors))
		} else {
			((Gitignore::empty(), PathBuf::new()), None)
		};
		let root = rules_dir(root);
		(Self { root, skip_hidden, respect_ignores, overrides, global, global_dir, above_root }, errors)
	}
}

/// The rules from git's global ignore file, and the working directory, which its paths are relative to. Without one, they can't be matched.
fn global_rules() -> (Gitignore, PathBuf) {
	let path = global_ignore_path();
	let cwd = env::current_dir();
	let (Some(path), Ok(cwd)) = (path.filter(|path| path.is_file()), cwd) else {
		return (Gitignore::empty(), PathBuf::new());
	};
	let mut builder = GitignoreBuilder::new(".");
	let _ = builder.add(path);
	(builder.build().unwrap_or_else(|_| Gitignore::empty()), cwd)
}

/// The file that git's `core.excludesFile` names, looking through its config files in order of priority, or else the file git uses by default.
fn global_ignore_path() -> Option<PathBuf> {
	let nonempty = |name| env::var_os(name).filter(|value| !value.is_empty()).map(PathBuf::from);
	let config_home = nonempty("XDG_CONFIG_HOME").or_else(|| env::home_dir().map(|home| home.join(".config")));
	let configs = [
		nonempty("GIT_CONFIG_GLOBAL"),
		env::home_dir().map(|home| home.join(".gitconfig")),
		config_home.as_ref().map(|dir| dir.join("git/config")),
		Some(nonempty("GIT_CONFIG_SYSTEM").unwrap_or_else(|| PathBuf::from("/etc/gitconfig"))),
	];
	configs
		.into_iter()
		.flatten()
		.find_map(|config| excludes_file_in(&fs::read(config).ok()?))
		.or_else(|| config_home.map(|dir| dir.join("git/ignore")))
}

/// The `excludesfile` setting in a git config file's contents, in whatever section. A full parser isn't worth it: this is what the `ignore` crate reads, without the regular expression that costs a noticeable part of a small run.
fn excludes_file_in(config: &[u8]) -> Option<PathBuf> {
	config.split(|&byte| byte == b'\n').find_map(|line| {
		let line = line.trim_ascii();
		let rest =
			line.get(EXCLUDES_KEY.len()..).filter(|_| line[..EXCLUDES_KEY.len()].eq_ignore_ascii_case(EXCLUDES_KEY))?;
		let value = rest.trim_ascii_start().strip_prefix(b"=")?.trim_ascii();
		let value = value.strip_prefix(b"\"").unwrap_or(value).trim_ascii_start();
		let value = value.strip_suffix(b"\"").unwrap_or(value).trim_ascii_end();
		if value.is_empty() || value.iter().any(u8::is_ascii_whitespace) {
			return None;
		}
		let value = str::from_utf8(value).ok()?;
		Some(PathBuf::from(
			env::home_dir().map_or_else(|| value.to_string(), |home| value.replace('~', &home.to_string_lossy())),
		))
	})
}

/// The rules in the directories above `root`, if any.
fn rules_above(root: &Path, errors: &mut Vec<Error>) -> Option<(Vec<DirRules>, PathBuf)> {
	// There's nothing to do about a failure here, and the rules that remain still apply.
	let absolute = root.canonicalize().ok()?;
	let parents: Vec<DirRules> = absolute
		.ancestors()
		.skip(1)
		.map(|parent| DirRules::read(parent, &Found::unknown(), errors))
		.filter(|rules| !rules.is_empty())
		.collect();
	(!parents.is_empty()).then_some((parents, absolute))
}

/// Which files of rules a directory listing showed.
#[derive(Default)]
pub(super) struct Found {
	ignore: bool,
	git_ignore: bool,
	git: bool,
}

impl Found {
	/// Without a listing, each has to be looked for.
	const fn unknown() -> Self {
		Self { ignore: true, git_ignore: true, git: true }
	}

	pub(super) fn note(&mut self, path: &Path) {
		match listed_name(path) {
			b".ignore" => self.ignore = true,
			b".gitignore" => self.git_ignore = true,
			b".git" => self.git = true,
			_ => {}
		}
	}

	const fn any(&self) -> bool {
		self.ignore || self.git_ignore || self.git
	}
}

/// The rules from one directory's files.
struct DirRules {
	dir: PathBuf,
	ignore: Option<Gitignore>,
	git_ignore: Option<Gitignore>,
	git_exclude: Option<Gitignore>,
}

impl DirRules {
	fn read(dir: &Path, found: &Found, errors: &mut Vec<Error>) -> Self {
		let ignore = found.ignore.then(|| read_rules(dir, ".ignore", errors)).flatten();
		let git_ignore = found.git_ignore.then(|| read_rules(dir, ".gitignore", errors)).flatten();
		let git_exclude = if found.git {
			let git_type = dir.join(".git").metadata().ok().map(|metadata| metadata.file_type());
			match git_common_dir(dir, git_type) {
				Ok(git_dir) => read_rules(&git_dir, "info/exclude", errors),
				Err(err) => {
					errors.extend(err);
					None
				}
			}
		} else {
			None
		};
		Self { dir: rules_dir(dir), ignore, git_ignore, git_exclude }
	}

	const fn is_empty(&self) -> bool {
		self.ignore.is_none() && self.git_ignore.is_none() && self.git_exclude.is_none()
	}
}

/// The rules in `directory/name`, if it has any. I/O errors just mean there are none.
///
/// They're built with `.` as their root, and given paths relative to where they apply. With any other root, `Gitignore` compares it with `.` by parsing path components for every path, which takes up to 6% of a run.
fn read_rules(directory: &Path, name: &str, errors: &mut Vec<Error>) -> Option<Gitignore> {
	let path = directory.join(name);
	// A `stat` is cheaper than failing to open the file, except on Windows.
	if !cfg!(windows) && !path.exists() {
		return None;
	}
	let mut builder = GitignoreBuilder::new(".");
	if let Some(err) = builder.add(path)
		&& !err.is_io()
	{
		errors.push(err);
	}
	match builder.build() {
		Ok(rules) => (!rules.is_empty()).then_some(rules),
		Err(err) => {
			errors.push(err);
			None
		}
	}
}

/// The directory holding a repository's `info/exclude`. A linked worktree's `.git` is a file that leads to it.
fn git_common_dir(dir: &Path, git_type: Option<FileType>) -> Result<PathBuf, Option<Error>> {
	let git = dir.join(".git");
	if !git_type.is_some_and(|file_type| file_type.is_file()) {
		return Ok(git);
	}
	let first_line = |path: &Path| -> Result<Option<String>, Error> {
		let file = File::open(path).map_err(|err| super::io_error(err, path.to_path_buf()))?;
		BufReader::new(file).lines().next().transpose().map_err(|err| super::io_error(err, path.to_path_buf()))
	};
	let line = first_line(&git).map_err(Some)?.ok_or(None)?;
	let git_dir = PathBuf::from(line.strip_prefix("gitdir: ").ok_or(None)?);
	let common_dir_file = git_dir.join("commondir");
	if !common_dir_file.is_file() {
		return Err(None);
	}
	let common_dir = first_line(&common_dir_file).map_err(Some)?.ok_or(None)?;
	Ok(if common_dir.starts_with('.') { git_dir.join(common_dir) } else { PathBuf::from(common_dir) })
}

/// A directory's link in the chain of rules. A link's rules outrank those of the links above it.
pub(super) struct Rules {
	own: Option<DirRules>,
	dir: Option<PathBuf>,
	parent: Option<Arc<Self>>,
}

impl Rules {
	/// The rules that apply in the root's parent, which is none of the ones the walk's `Shared` holds.
	pub(super) fn for_root() -> Arc<Self> {
		Arc::new(Self { own: None, dir: None, parent: None })
	}

	/// The rules for a child directory, given what its listing showed. With `keep_every_dir`, each directory gets a link, which is how a link that loops finds the directories above it.
	pub(super) fn for_child(
		self: &Arc<Self>,
		dir: &Path,
		found: &Found,
		shared: &Shared,
		keep_every_dir: bool,
	) -> Arc<Self> {
		let own = (shared.respect_ignores && found.any())
			// Errors in a directory's rules leave those rules out, and aren't reported.
			.then(|| DirRules::read(dir, found, &mut Vec::new()))
			.filter(|rules| !rules.is_empty());
		if own.is_none() && !keep_every_dir {
			return Arc::clone(self);
		}
		Arc::new(Self { own, dir: Some(dir.to_path_buf()), parent: Some(Arc::clone(self)) })
	}

	/// The directories above this point in the walk, nearest first, for a walk that follows links.
	pub(super) fn ancestors(&self) -> impl Iterator<Item = &Path> {
		self.links().filter_map(|link| link.dir.as_deref())
	}

	fn links(&self) -> impl Iterator<Item = &Self> {
		std::iter::successors(Some(self), |link| link.parent.as_deref())
	}

	/// Whether the walk leaves this path out. `--exclude` decides first, then ignore files, then whether it's hidden. A path an ignore file lets back in counts even if hidden.
	pub(super) fn skips(&self, path: &Path, is_dir: bool, hidden_attribute: bool, shared: &Shared) -> bool {
		let path = strip_start(Path::new("./"), path).unwrap_or(path);
		let matched =
			shared.overrides.as_ref().map_or(Match::None, |overrides| overrides.matched(path, is_dir).map(|_| ()));
		let matched =
			if matched.is_none() && shared.respect_ignores { self.matched(path, is_dir, shared) } else { matched };
		if matched.is_ignore() {
			return true;
		}
		matched.is_none() && shared.skip_hidden && (hidden_attribute || listed_name(path).starts_with(b"."))
	}

	fn matched(&self, path: &Path, is_dir: bool, shared: &Shared) -> Match<()> {
		let mut totals = RuleMatches::default();
		for rules in self.links().filter_map(|link| link.own.as_ref()) {
			totals.add(rules, path, is_dir);
		}
		if let Some((above, absolute)) = &shared.above_root {
			let path = absolute.join(relative_to(&shared.root, path));
			for rules in above {
				totals.add(rules, &path, is_dir);
			}
		}
		let global = shared.global.matched(relative_to(&shared.global_dir, path), is_dir).map(|_| ());
		totals.ignore.or(totals.git_ignore).or(totals.git_exclude).or(global)
	}
}

/// Each kind of file's nearest match. Which kind matched outranks which directory did.
struct RuleMatches {
	ignore: Match<()>,
	git_ignore: Match<()>,
	git_exclude: Match<()>,
}

impl Default for RuleMatches {
	fn default() -> Self {
		Self { ignore: Match::None, git_ignore: Match::None, git_exclude: Match::None }
	}
}

impl RuleMatches {
	fn add(&mut self, rules: &DirRules, path: &Path, is_dir: bool) {
		let path = relative_to(&rules.dir, path);
		fill(&mut self.ignore, rules.ignore.as_ref(), path, is_dir);
		fill(&mut self.git_ignore, rules.git_ignore.as_ref(), path, is_dir);
		fill(&mut self.git_exclude, rules.git_exclude.as_ref(), path, is_dir);
	}
}

fn fill(found: &mut Match<()>, rules: Option<&Gitignore>, path: &Path, is_dir: bool) {
	if found.is_none()
		&& let Some(rules) = rules
	{
		*found = rules.matched(path, is_dir).map(|_| ());
	}
}

/// `dir` as [`relative_to`] takes it: the walk's paths lose any leading `./`, and `.` is where they already start.
fn rules_dir(dir: &Path) -> PathBuf {
	if dir == Path::new(".") {
		return PathBuf::new();
	}
	strip_start(Path::new("./"), dir).unwrap_or(dir).to_path_buf()
}

/// `path` relative to `dir`, or all of it if it's elsewhere.
fn relative_to<'a>(dir: &Path, path: &'a Path) -> &'a Path {
	let relative = strip_start(dir, path).unwrap_or(path);
	strip_start(Path::new("/"), relative).unwrap_or(relative)
}

/// `path` without `prefix` at its start, as bytes on Unix, which is much cheaper than comparing components.
pub fn strip_start<'a>(prefix: &Path, path: &'a Path) -> Option<&'a Path> {
	#[cfg(unix)]
	{
		use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
		path.as_os_str()
			.as_bytes()
			.strip_prefix(prefix.as_os_str().as_bytes())
			.map(|rest| Path::new(OsStr::from_bytes(rest)))
	}
	#[cfg(not(unix))]
	{
		path.strip_prefix(prefix).ok()
	}
}

/// The name at the end of a path from a directory listing, which never ends in a separator. [`Path::file_name`] parses components, which adds up over every entry of a walk.
pub fn listed_name(path: &Path) -> &[u8] {
	let bytes = path.as_os_str().as_encoded_bytes();
	bytes.iter().rposition(|&byte| std::path::is_separator(char::from(byte))).map_or(bytes, |end| &bytes[end + 1..])
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn excludes_file_is_read_from_git_config() {
		let found = |config: &str| excludes_file_in(config.as_bytes());
		assert_eq!(found("[core]\n\texcludesFile = /a/b\n"), Some(PathBuf::from("/a/b")));
		assert_eq!(found("[core]\nexcludesfile=/a/b\r\n"), Some(PathBuf::from("/a/b")));
		assert_eq!(found("EXCLUDESFILE = \"/a/b\"  \n"), Some(PathBuf::from("/a/b")));
		assert_eq!(found("[user]\n\tname = Quin\n"), None);
		assert_eq!(found("excludesfile = /has space\nexcludesfile = /second\n"), Some(PathBuf::from("/second")));
		assert_eq!(found("excludesfile =\n"), None);
		assert_eq!(found("excludesfilesomething = /a\n"), None);
		assert_eq!(found(""), None);
	}
}
