//! The linguist attributes a project sets in its `.gitattributes` files, which GitHub reads for its language statistics: `linguist-generated`, `linguist-vendored`, `linguist-documentation` and `linguist-language`.

use std::{
	collections::HashMap,
	fs,
	hash::{BuildHasherDefault, Hasher},
	path::{Component, Path, PathBuf},
	sync::{Arc, Mutex, PoisonError},
};

use ignore::{
	Match,
	gitignore::{Gitignore, GitignoreBuilder},
};

use crate::langs::{Language, language_named};

/// What a file's attributes say about it. `None` fields say nothing, leaving the file to codestats' own rules.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Verdict {
	/// `linguist-generated`, or `-linguist-generated` (or `=false`) to say a file codestats would take for generated isn't.
	pub generated: Option<bool>,
	/// `linguist-vendored` or `linguist-documentation`: left out, as GitHub leaves them out of its statistics.
	pub excluded: bool,
	/// `linguist-language=Name`, when codestats knows a language by that name.
	pub language: Option<&'static Language>,
}

/// Where `.gitattributes` files come from: the disk, found as each directory is first seen, or given up front in tests.
enum Source {
	Disk,
	/// Contents by the directory holding them.
	#[cfg(test)]
	Loaded(HashMap<PathBuf, Vec<u8>>),
}

/// The linguist attributes of a tree, shared by the threads counting it. Each directory's rules are worked out once.
pub struct Attributes {
	/// The repository's root, or the counted directory outside a repository, as an absolute path.
	base: PathBuf,
	/// What paths given to [`Attributes::verdict`] start with to name the counted directory, and that directory relative to `base`.
	root: PathBuf,
	root_from_base: PathBuf,
	source: Source,
	/// Each directory's rules, by the bytes of its path as given. Most directories share their parent's.
	dirs: Mutex<DirMap>,
}

/// Directories' rules by the bytes of their paths.
type DirMap = HashMap<Box<[u8]>, Arc<Dir>, BuildHasherDefault<Fnv>>;

/// The rules that apply in one directory: those of the `.gitattributes` files in it and above it, shallowest first.
#[derive(Default)]
struct Dir {
	layers: Vec<Arc<Rules>>,
	/// `[attr]` macros, which git only reads from the top-level file, but are kept from any here.
	macros: Arc<HashMap<String, Vec<String>>>,
}

/// The linguist rules of one `.gitattributes` file.
struct Rules {
	place: Place,
	generated: Gitignore,
	excluded: Gitignore,
	/// Patterns setting a language, in the file's order, so the last match wins.
	languages: Vec<(Gitignore, &'static Language)>,
}

/// Where a `.gitattributes` file is, to make a counted file's path relative to it.
enum Place {
	/// In the counted directory or below it, at this path as given paths start.
	Within(PathBuf),
	/// Above the counted directory, which is at this path from it.
	Above(PathBuf),
}

impl Attributes {
	/// The attributes for counting the directory `root` on disk (or the directory holding it, for a file): the `.gitattributes` files inside it, and those above it up to its repository's root. Paths given to [`Attributes::verdict`] start with the directory, as a walk gives them.
	pub fn on_disk(root: &Path) -> Self {
		let dir = if root.is_file() { root.parent().unwrap_or_else(|| Path::new("")) } else { root };
		let absolute = std::env::current_dir().map_or_else(|_| dir.to_path_buf(), |cwd| normalize(&cwd.join(dir)));
		let base = absolute.ancestors().find(|dir| dir.join(".git").exists()).unwrap_or(&absolute).to_path_buf();
		let root_from_base = absolute.strip_prefix(&base).map(Path::to_path_buf).unwrap_or_default();
		Self::new(base, dir.to_path_buf(), root_from_base, Source::Disk)
	}

	/// The attributes of the `.gitattributes` files given by their paths, with paths given to [`Attributes::verdict`] relative to the same root.
	#[cfg(test)]
	fn loaded(files: impl IntoIterator<Item = (PathBuf, Vec<u8>)>) -> Self {
		let files = files
			.into_iter()
			.map(|(path, contents)| (path.parent().map(Path::to_path_buf).unwrap_or_default(), contents))
			.collect();
		Self::new(PathBuf::new(), PathBuf::new(), PathBuf::new(), Source::Loaded(files))
	}

	fn new(base: PathBuf, root: PathBuf, root_from_base: PathBuf, source: Source) -> Self {
		Self { base, root, root_from_base, source, dirs: Mutex::new(HashMap::default()) }
	}

	/// What the attributes say about the file at `path`, which starts with the counted directory. `None` when no rule names it.
	pub fn verdict(&self, path: &Path, cache: &mut DirCache) -> Option<Verdict> {
		// Files come a directory at a time, so compare the directory's bytes with the last one's, which is much cheaper than parsing the path's components.
		let bytes = path.as_os_str().as_encoded_bytes();
		let dir_bytes = &bytes[..bytes.iter().rposition(|&b| is_separator(b)).unwrap_or(0)];
		let dir = match &cache.last {
			Some((last, dir)) if **last == *dir_bytes => Arc::clone(dir),
			_ => {
				let dir = self.dir(path.parent().unwrap_or_else(|| Path::new("")));
				cache.last = Some((dir_bytes.into(), Arc::clone(&dir)));
				dir
			}
		};
		if dir.layers.is_empty() {
			return None;
		}
		let mut verdict = Verdict::default();
		let mut any = false;
		// Deeper files override shallower ones, so the first opinion found from the deepest wins.
		for rules in dir.layers.iter().rev() {
			let relative = match &rules.place {
				Place::Within(dir) => path.strip_prefix(dir).ok()?.to_path_buf(),
				Place::Above(root) => root.join(path.strip_prefix(&self.root).ok()?),
			};
			if verdict.generated.is_none() {
				verdict.generated = opinion(&rules.generated, &relative);
				any |= verdict.generated.is_some();
			}
			if let Some(excluded) = opinion(&rules.excluded, &relative) {
				verdict.excluded |= excluded;
				any = true;
			}
			if verdict.language.is_none() {
				verdict.language = rules
					.languages
					.iter()
					.rev()
					.find(|(pattern, _)| matches!(pattern.matched(&relative, false), Match::Ignore(_)))
					.map(|&(_, language)| language);
				any |= verdict.language.is_some();
			}
		}
		any.then_some(verdict)
	}

	/// The rules for the directory at `path`, as paths given to [`Attributes::verdict`] spell it.
	fn dir(&self, path: &Path) -> Arc<Dir> {
		let key = path.as_os_str().as_encoded_bytes();
		if let Some(dir) = self.dirs.lock().unwrap_or_else(PoisonError::into_inner).get(key) {
			return Arc::clone(dir);
		}
		let root = self.root.as_os_str().as_encoded_bytes();
		let below_root = key.len() > root.len()
			&& key.starts_with(root)
			&& (root.is_empty() || is_separator(key[root.len()]) || is_separator(root[root.len() - 1]));
		let dir = if below_root {
			let parent = self.dir(path.parent().unwrap_or_else(|| Path::new("")));
			self.with_own_rules(&parent, path, || Place::Within(path.to_path_buf()))
		} else {
			Arc::new(self.root_dir())
		};
		self.dirs.lock().unwrap_or_else(PoisonError::into_inner).insert(key.into(), Arc::clone(&dir));
		dir
	}

	/// The rules for the counted directory: from the base down to it.
	fn root_dir(&self) -> Dir {
		let mut dir = Arc::new(Dir::default());
		let mut from_base = PathBuf::new();
		let mut components = self.root_from_base.components();
		loop {
			let place = || {
				let rest = self.root_from_base.strip_prefix(&from_base).unwrap_or(&self.root_from_base);
				if rest.as_os_str().is_empty() {
					Place::Within(self.root.clone())
				} else {
					Place::Above(rest.to_path_buf())
				}
			};
			dir = self.with_own_rules(&dir, &self.base.join(&from_base), place);
			let Some(component) = components.next() else { break };
			from_base.push(component);
		}
		Arc::into_inner(dir).unwrap_or_default()
	}

	/// The rules of the directory at `path`: `parent`'s, followed by those of its own `.gitattributes` file, placed by `place`, if it has one with linguist rules. A directory without one shares its parent's.
	fn with_own_rules(&self, parent: &Arc<Dir>, path: &Path, place: impl FnOnce() -> Place) -> Arc<Dir> {
		let contents = match &self.source {
			Source::Disk => fs::read(path.join(".gitattributes")).ok(),
			#[cfg(test)]
			Source::Loaded(files) => files.get(path).cloned(),
		};
		let Some(contents) = contents else {
			return Arc::clone(parent);
		};
		let (rules, own_macros) = parse(&String::from_utf8_lossy(&contents), place, &parent.macros);
		if rules.is_none() && own_macros.is_empty() {
			return Arc::clone(parent);
		}
		let macros = if own_macros.is_empty() {
			Arc::clone(&parent.macros)
		} else {
			let mut all = (*parent.macros).clone();
			all.extend(own_macros);
			Arc::new(all)
		};
		let mut layers = parent.layers.clone();
		layers.extend(rules.map(Arc::new));
		Arc::new(Dir { layers, macros })
	}
}

/// A worker's memory of the last directory it asked about, as files come a directory at a time.
#[derive(Default)]
pub struct DirCache {
	last: Option<(Box<[u8]>, Arc<Dir>)>,
}

/// Whether `byte` separates a path's components.
const fn is_separator(byte: u8) -> bool {
	byte == b'/' || (cfg!(windows) && byte == b'\\')
}

/// The FNV-1a hash, much cheaper than the default for the short keys of paths, which needn't resist attacks.
#[derive(Default)]
struct Fnv(u64);

impl Hasher for Fnv {
	fn finish(&self) -> u64 {
		self.0
	}

	fn write(&mut self, bytes: &[u8]) {
		let mut hash = if self.0 == 0 { 0xcbf2_9ce4_8422_2325 } else { self.0 };
		for &byte in bytes {
			hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
		}
		self.0 = hash;
	}
}

/// Whether a set of patterns sets an attribute for `path` (`Some(true)`), unsets it (`Some(false)`), or says nothing.
fn opinion(patterns: &Gitignore, path: &Path) -> Option<bool> {
	match patterns.matched(path, false) {
		Match::Ignore(_) => Some(true),
		Match::Whitelist(_) => Some(false),
		Match::None => None,
	}
}

/// The linguist rules in a `.gitattributes` file, placed by `place`, or `None` when it has none, and the macros it defines.
fn parse(
	contents: &str,
	place: impl FnOnce() -> Place,
	macros: &HashMap<String, Vec<String>>,
) -> (Option<Rules>, HashMap<String, Vec<String>>) {
	let mut own_macros: HashMap<String, Vec<String>> = HashMap::new();
	let mut generated = GitignoreBuilder::new(".");
	let mut excluded = GitignoreBuilder::new(".");
	let mut languages = Vec::new();
	let mut any = false;
	for line in contents.lines() {
		let line = line.trim();
		if line.is_empty() || line.starts_with('#') {
			continue;
		}
		let mut fields = line.split_ascii_whitespace();
		let Some(pattern) = fields.next() else { continue };
		if let Some(name) = pattern.strip_prefix("[attr]") {
			own_macros.insert(name.to_owned(), fields.map(str::to_owned).collect());
			continue;
		}
		let expanded: Vec<&str> = fields
			.flat_map(|field| {
				let expansion = own_macros.get(field).or_else(|| macros.get(field));
				expansion.map_or_else(|| vec![field], |attributes| attributes.iter().map(String::as_str).collect())
			})
			.collect();
		for attribute in expanded {
			let (name, value) = match attribute.split_once('=') {
				Some((name, value)) => (name, Some(value)),
				None => (attribute, None),
			};
			// `-name` and `name=false` unset an attribute; `name` and `name=true` set it.
			let set = !name.starts_with('-') && value.is_none_or(|value| !value.eq_ignore_ascii_case("false"));
			let name = name.trim_start_matches('-');
			let line = if set { pattern.to_owned() } else { format!("!{pattern}") };
			match name {
				"linguist-generated" => {
					any |= generated.add_line(None, &line).is_ok();
				}
				"linguist-vendored" | "linguist-documentation" => {
					any |= excluded.add_line(None, &line).is_ok();
				}
				"linguist-language" => {
					if let Some(language) = value.and_then(language_named) {
						let mut builder = GitignoreBuilder::new(".");
						if builder.add_line(None, pattern).is_ok()
							&& let Ok(matcher) = builder.build()
						{
							languages.push((matcher, language));
							any = true;
						}
					}
				}
				_ => {}
			}
		}
	}
	let rules = any.then(|| Rules {
		place: place(),
		generated: generated.build().unwrap_or_else(|_| Gitignore::empty()),
		excluded: excluded.build().unwrap_or_else(|_| Gitignore::empty()),
		languages,
	});
	(rules, own_macros)
}

/// `path` with `.` and `..` worked out without touching the disk.
fn normalize(path: &Path) -> PathBuf {
	let mut normal = PathBuf::new();
	for component in path.components() {
		match component {
			Component::CurDir => {}
			Component::ParentDir => {
				normal.pop();
			}
			other => normal.push(other),
		}
	}
	normal
}

#[cfg(test)]
mod tests {
	use super::*;

	fn verdict(files: &[(&str, &str)], path: &str) -> Option<Verdict> {
		let attributes =
			Attributes::loaded(files.iter().map(|&(path, contents)| (PathBuf::from(path), contents.into())));
		attributes.verdict(Path::new(path), &mut DirCache::default())
	}

	#[expect(clippy::unnecessary_wraps, reason = "compared with what `verdict` returns")]
	fn generated(value: bool) -> Option<Verdict> {
		Some(Verdict { generated: Some(value), ..Verdict::default() })
	}

	#[test]
	fn generated_and_its_negations() {
		let root = ".gitattributes";
		let rules = "**/zz_generated.*.go linguist-generated=true\nCargo.lock linguist-generated=false\nvendor.js -linguist-generated\n";
		assert_eq!(verdict(&[(root, rules)], "pkg/api/zz_generated.deepcopy.go"), generated(true));
		assert_eq!(verdict(&[(root, rules)], "Cargo.lock"), generated(false));
		assert_eq!(verdict(&[(root, rules)], "vendor.js"), generated(false));
		assert_eq!(verdict(&[(root, rules)], "pkg/api/types.go"), None);
	}

	#[test]
	fn later_lines_and_deeper_files_win() {
		let files = [
			(".gitattributes", "*.js linguist-generated\nkeep.js -linguist-generated\n"),
			("lib/.gitattributes", "keep.js linguist-generated\n"),
		];
		assert_eq!(verdict(&files, "keep.js"), generated(false));
		assert_eq!(verdict(&files, "lib/keep.js"), generated(true));
		assert_eq!(verdict(&files, "lib/other.js"), generated(true));
	}

	#[test]
	fn patterns_are_relative_to_their_file() {
		let files = [("lib/.gitattributes", "/top.js linguist-generated\n")];
		assert_eq!(verdict(&files, "lib/top.js"), generated(true));
		assert_eq!(verdict(&files, "lib/sub/top.js"), None);
		assert_eq!(verdict(&files, "top.js"), None);
	}

	#[test]
	fn vendored_and_documentation_are_excluded() {
		let files = [(".gitattributes", "lib/libc/** linguist-vendored\nruntime/doc/* linguist-documentation\n")];
		assert!(verdict(&files, "lib/libc/include/stdio.h").is_some_and(|verdict| verdict.excluded));
		assert!(verdict(&files, "runtime/doc/api.txt").is_some_and(|verdict| verdict.excluded));
		assert_eq!(verdict(&files, "lib/std/fs.zig"), None);
	}

	#[test]
	fn language_is_set_by_name() {
		let files = [(".gitattributes", "*.fixed linguist-language=Rust\n*.m4 linguist-language=NoSuchLanguage\n")];
		assert_eq!(
			verdict(&files, "tests/ui/a.fixed").and_then(|verdict| verdict.language).map(|l| l.name),
			Some("Rust")
		);
		assert_eq!(verdict(&files, "configure.m4"), None);
	}

	#[test]
	fn macros_expand() {
		let files =
			[(".gitattributes", "[attr]generated linguist-generated=true diff=generated\nPython/opcode.h generated\n")];
		assert_eq!(verdict(&files, "Python/opcode.h"), generated(true));
	}
}
