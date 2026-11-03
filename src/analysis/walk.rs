//! A parallel directory walk that respects ignore files, adapted from the `ignore` crate's (Unlicense), whose matchers it still uses.
//!
//! That crate's idle workers sleep for a millisecond before looking for work again, and it starts every thread up front. Both cost more than a small repository takes to count. Here, the first thread starts alone, others start only once there's work for them, and sleeping ones are woken.

mod rules;

use std::{
	fs::{self, FileType},
	io,
	path::{Path, PathBuf},
	sync::{
		Arc, Condvar, Mutex, PoisonError,
		atomic::{AtomicBool, AtomicUsize, Ordering},
	},
	thread::{self, Scope},
	time::Duration,
};

use crossbeam_deque::{Stealer, Worker as Deque};
use ignore::{Error, overrides::Override};
pub(super) use rules::strip_start;
use rules::{Rules, Shared};
use same_file::Handle;

/// How much work a worker holds before it wakes a sleeping one. Waking one for every new file costs more on large trees than the wait does.
const WAKE_ABOVE: usize = 32;
/// How long a sleeping worker waits before looking for work anyway, in case it missed being woken.
const SLEEP: Duration = Duration::from_millis(1);

/// What a walk skips and how many threads it may use.
pub(super) struct WalkOptions {
	pub follow_links: bool,
	pub skip_hidden: bool,
	pub max_depth: Option<usize>,
	/// Whether to apply `.gitignore`, `.ignore`, `.git/info/exclude`, and git's global ignore file.
	pub respect_ignores: bool,
	pub overrides: Option<Override>,
	pub threads: usize,
}

/// A file a walk found. With `follow_links`, a link to a file counts as that file.
pub(super) struct Entry {
	path: PathBuf,
	file_type: FileType,
	depth: usize,
	#[cfg(windows)]
	hidden_attribute: bool,
}

impl Entry {
	pub(super) fn path(&self) -> &Path {
		&self.path
	}

	fn from_path(path: PathBuf, depth: usize) -> Result<Self, Error> {
		match fs::metadata(&path) {
			Ok(metadata) => Ok(Self {
				file_type: metadata.file_type(),
				depth,
				#[cfg(windows)]
				hidden_attribute: has_hidden_attribute(&metadata),
				path,
			}),
			Err(err) => Err(io_error(err, path)),
		}
	}

	fn is_dir(&self) -> bool {
		self.file_type.is_dir()
	}
}

#[cfg(windows)]
fn has_hidden_attribute(metadata: &fs::Metadata) -> bool {
	const FILE_ATTRIBUTE_HIDDEN: u32 = 2;
	std::os::windows::fs::MetadataExt::file_attributes(metadata) & FILE_ATTRIBUTE_HIDDEN != 0
}

fn io_error(err: io::Error, path: PathBuf) -> Error {
	Error::WithPath { path, err: Box::new(Error::Io(err)) }
}

/// Walk `root` on up to `options.threads` threads, giving each thread's `visit`, made by `make_visitor`, every regular file and error it finds. A `root` that's a file is visited whatever the ignore rules say.
pub(super) fn walk<V, F>(root: &Path, options: WalkOptions, make_visitor: &F)
where
	V: FnMut(Result<Entry, Error>),
	F: Fn() -> V + Sync,
{
	let threads = options.threads.max(1);
	let follow_links = options.follow_links;
	let max_depth = options.max_depth;
	let root_entry = match Entry::from_path(root.to_path_buf(), 0) {
		Ok(entry) if entry.is_dir() => entry,
		Ok(entry) => {
			if entry.file_type.is_file() {
				make_visitor()(Ok(entry));
			}
			return;
		}
		Err(err) => {
			make_visitor()(Err(err));
			return;
		}
	};
	let (shared, parent_errors) = Shared::new(root, options.skip_hidden, options.respect_ignores, options.overrides);
	let deques: Vec<Deque<Message>> = std::iter::repeat_with(Deque::new_lifo).take(threads).collect();
	let stealers: Vec<Stealer<Message>> = deques.iter().map(Deque::stealer).collect();
	let mut stacks: Vec<_> = deques.into_iter().enumerate().map(|(index, deque)| Stack { index, deque }).collect();
	let first = stacks.remove(0);
	first.deque.push(Message::Work(Work::Dir(root_entry, Rules::for_root())));
	let pool = Pool {
		shared: &shared,
		follow_links,
		max_depth,
		stealers: &stealers,
		unstarted: Mutex::new(stacks),
		starting: AtomicBool::new(false),
		active: AtomicUsize::new(1),
		quit_now: AtomicBool::new(false),
		sleepers: AtomicUsize::new(0),
		lock: Mutex::new(()),
		wake: Condvar::new(),
	};
	thread::scope(|scope| {
		let mut visit = make_visitor();
		for err in parent_errors {
			visit(Err(err));
		}
		Worker { pool: &pool, stack: first, visit, scope, make_visitor }.run();
	});
}

enum Message {
	Work(Work),
	Quit,
}

enum Work {
	/// A directory to read, with the rules that apply in its parent.
	Dir(Entry, Arc<Rules>),
	File(Entry),
}

struct Stack {
	index: usize,
	deque: Deque<Message>,
}

/// What the workers share.
struct Pool<'a> {
	shared: &'a Shared,
	follow_links: bool,
	max_depth: Option<usize>,
	stealers: &'a [Stealer<Message>],
	/// The stacks of the workers that haven't started.
	unstarted: Mutex<Vec<Stack>>,
	/// Whether a worker is on its way up. Workers start one at a time, each starting the next if there's still work, so the one that found the work doesn't stop to start them all.
	starting: AtomicBool,
	active: AtomicUsize,
	/// Whether a worker panicked, so the rest should stop.
	quit_now: AtomicBool,
	sleepers: AtomicUsize,
	lock: Mutex<()>,
	wake: Condvar,
}

struct Worker<'pool, 'scope, 'env, V, F> {
	pool: &'pool Pool<'env>,
	stack: Stack,
	visit: V,
	scope: &'scope Scope<'scope, 'env>,
	make_visitor: &'env F,
}

impl<'pool: 'scope, 'scope, 'env: 'scope, V, F> Worker<'pool, 'scope, 'env, V, F>
where
	V: FnMut(Result<Entry, Error>),
	F: Fn() -> V + Sync,
{
	fn run(mut self) {
		while let Some(work) = self.next_work() {
			match work {
				Work::File(entry) => (self.visit)(Ok(entry)),
				Work::Dir(entry, rules) => self.read_dir(&entry, &rules),
			}
		}
	}

	/// Read a directory, and queue what it holds that the rules allow.
	fn read_dir(&mut self, dir: &Entry, parent_rules: &Arc<Rules>) {
		if self.pool.max_depth.is_some_and(|max_depth| dir.depth >= max_depth) {
			return;
		}
		let shared = self.pool.shared;
		let reader = match fs::read_dir(&dir.path) {
			Ok(reader) => reader,
			Err(err) => {
				(self.visit)(Err(io_error(err, dir.path.clone())));
				return;
			}
		};
		let mut found = rules::Found::default();
		let mut entries = Vec::new();
		for entry in reader {
			match entry {
				Ok(entry) => {
					let path = entry.path();
					found.note(&path);
					entries.push(entry_parts(&entry, path));
				}
				Err(err) => (self.visit)(Err(io_error(err, dir.path.clone()))),
			}
		}
		let rules = parent_rules.for_child(&dir.path, &found, shared, self.pool.follow_links);
		for (path, file_type, attribute) in entries {
			self.queue(&rules, dir.depth + 1, path, file_type, attribute);
		}
	}

	/// Queue an entry of a directory whose rules are `rules`, unless they skip it.
	fn queue(
		&mut self,
		rules: &Arc<Rules>,
		depth: usize,
		path: PathBuf,
		file_type: io::Result<FileType>,
		attribute: bool,
	) {
		// Git's own directory is never source, even when hidden files are counted.
		if rules::listed_name(&path) == b".git" {
			return;
		}
		let file_type = match file_type {
			Ok(file_type) => file_type,
			Err(err) => {
				(self.visit)(Err(io_error(err, path)));
				return;
			}
		};
		let entry = if self.pool.follow_links && file_type.is_symlink() {
			let entry = match Entry::from_path(path, depth) {
				Ok(entry) => entry,
				Err(err) => {
					(self.visit)(Err(err));
					return;
				}
			};
			if entry.is_dir()
				&& let Err(err) = check_loop(rules, &entry.path)
			{
				(self.visit)(Err(err));
				return;
			}
			entry
		} else {
			#[cfg(windows)]
			let entry = Entry { path, file_type, depth, hidden_attribute: attribute };
			#[cfg(not(windows))]
			let entry = {
				let _ = attribute;
				Entry { path, file_type, depth }
			};
			entry
		};
		#[cfg(windows)]
		let hidden_attribute = entry.hidden_attribute;
		#[cfg(not(windows))]
		let hidden_attribute = false;
		if rules.skips(&entry.path, entry.is_dir(), hidden_attribute, self.pool.shared) {
			return;
		}
		if entry.is_dir() {
			self.send(Work::Dir(entry, Arc::clone(rules)));
		} else if entry.file_type.is_file() {
			self.send(Work::File(entry));
		}
	}

	fn send(&self, work: Work) {
		self.stack.deque.push(Message::Work(work));
		let queued = self.stack.deque.len();
		if queued > 1 {
			self.start_another();
		}
		if queued > WAKE_ABOVE && self.pool.sleepers.load(Ordering::SeqCst) > 0 {
			drop(self.pool.lock.lock().unwrap_or_else(PoisonError::into_inner));
			self.pool.wake.notify_one();
		}
	}

	/// Start another worker, unless one's already starting or they all have.
	fn start_another(&self) {
		let pool = self.pool;
		if pool.starting.swap(true, Ordering::AcqRel) {
			return;
		}
		let Some(stack) = pool.unstarted.lock().unwrap_or_else(PoisonError::into_inner).pop() else {
			// Left set, so nobody takes this path again.
			return;
		};
		pool.active.fetch_add(1, Ordering::AcqRel);
		let scope = self.scope;
		let make_visitor = self.make_visitor;
		scope.spawn(move || {
			pool.starting.store(false, Ordering::Release);
			Worker { pool, stack, visit: make_visitor(), scope, make_visitor }.run();
		});
	}

	/// The next thing to do, or `None` when the walk's over.
	fn next_work(&self) -> Option<Work> {
		let mut message = self.receive();
		loop {
			if self.pool.quit_now.load(Ordering::SeqCst) {
				return None;
			}
			match message {
				Some(Message::Work(work)) => return Some(work),
				Some(Message::Quit) => {
					self.send_quit();
					return None;
				}
				None => {
					// Only when every worker's own stack is empty at once is there no work left at all.
					if self.pool.active.fetch_sub(1, Ordering::Acquire) == 1 {
						self.send_quit();
						return None;
					}
					message = Some(self.wait()?);
				}
			}
		}
	}

	/// Sleep until there's a message, and become active again. `None` means a worker panicked.
	fn wait(&self) -> Option<Message> {
		let pool = self.pool;
		loop {
			if pool.quit_now.load(Ordering::SeqCst) {
				return None;
			}
			if let Some(message) = self.receive() {
				pool.active.fetch_add(1, Ordering::Release);
				return Some(message);
			}
			pool.sleepers.fetch_add(1, Ordering::SeqCst);
			// Work sent after the look above, and before this thread slept, is what the timeout's for.
			drop(
				pool.wake
					.wait_timeout(pool.lock.lock().unwrap_or_else(PoisonError::into_inner), SLEEP)
					.unwrap_or_else(PoisonError::into_inner),
			);
			pool.sleepers.fetch_sub(1, Ordering::SeqCst);
		}
	}

	/// Leave a quit message for the next worker to find, which it passes on in turn.
	fn send_quit(&self) {
		self.stack.deque.push(Message::Quit);
		drop(self.pool.lock.lock().unwrap_or_else(PoisonError::into_inner));
		self.pool.wake.notify_all();
	}

	fn receive(&self) -> Option<Message> {
		self.stack.deque.pop().or_else(|| {
			let (before, after) = self.pool.stealers.split_at(self.stack.index);
			after[1..].iter().chain(before).find_map(|stealer| stealer.steal_batch_and_pop(&self.stack.deque).success())
		})
	}
}

impl<V, F> Drop for Worker<'_, '_, '_, V, F> {
	fn drop(&mut self) {
		if thread::panicking() {
			self.pool.quit_now.store(true, Ordering::SeqCst);
		}
	}
}

/// A directory entry's type, and on Windows, whether its attributes hide it. Both come with the directory listing, so they cost no `stat`.
fn entry_parts(entry: &fs::DirEntry, path: PathBuf) -> (PathBuf, io::Result<FileType>, bool) {
	#[cfg(windows)]
	let attribute = entry.metadata().is_ok_and(|metadata| has_hidden_attribute(&metadata));
	#[cfg(not(windows))]
	let attribute = false;
	(path, entry.file_type(), attribute)
}

/// An error if a directory a link leads to is one of the directories above the link.
fn check_loop(rules: &Rules, child: &Path) -> Result<(), Error> {
	let handle = Handle::from_path(child).map_err(|err| io_error(err, child.to_path_buf()))?;
	for ancestor in rules.ancestors() {
		let ancestor_handle = Handle::from_path(ancestor).map_err(|err| io_error(err, child.to_path_buf()))?;
		if handle == ancestor_handle {
			return Err(Error::Loop { ancestor: ancestor.to_path_buf(), child: child.to_path_buf() });
		}
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use std::{collections::BTreeSet, sync::atomic::AtomicU64};

	use super::*;

	static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

	/// A directory holding `files`, as `(path, contents)`, that's removed afterward.
	struct Tree(PathBuf);

	impl Tree {
		fn new(files: &[(&str, &str)]) -> Self {
			let unique = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
			let root = std::env::temp_dir().join(format!("codestats_walk_test_{}_{unique}", std::process::id()));
			for (path, contents) in files {
				let path = root.join(path);
				fs::create_dir_all(path.parent().unwrap()).unwrap();
				fs::write(path, contents).unwrap();
			}
			fs::create_dir_all(&root).unwrap();
			Self(root)
		}

		fn walked(&self, options: WalkOptions) -> BTreeSet<String> {
			let found = Mutex::new(BTreeSet::new());
			walk(&self.0, options, &|| {
				|entry: Result<Entry, Error>| {
					let entry = entry.unwrap();
					let relative = entry.path().strip_prefix(&self.0).unwrap().to_string_lossy().replace('\\', "/");
					found.lock().unwrap().insert(relative);
				}
			});
			found.into_inner().unwrap()
		}
	}

	impl Drop for Tree {
		fn drop(&mut self) {
			fs::remove_dir_all(&self.0).unwrap();
		}
	}

	fn defaults() -> WalkOptions {
		WalkOptions {
			follow_links: false,
			skip_hidden: true,
			max_depth: None,
			respect_ignores: true,
			overrides: None,
			threads: 4,
		}
	}

	fn names(names: &[&str]) -> BTreeSet<String> {
		names.iter().map(ToString::to_string).collect()
	}

	#[test]
	fn nested_ignore_files_apply_below_where_they_are() {
		let tree = Tree::new(&[
			(".gitignore", "*.log\nbuilt/\n"),
			("a.rs", ""),
			("a.log", ""),
			("built/b.rs", ""),
			("sub/.ignore", "secret.rs\n"),
			("sub/c.rs", ""),
			("sub/secret.rs", ""),
			("secret.rs", ""),
			("sub/deep/.gitignore", "!keep.log\n"),
			("sub/deep/keep.log", ""),
			("sub/deep/drop.log.txt", ""),
		]);
		assert_eq!(
			tree.walked(defaults()),
			names(&["a.rs", "secret.rs", "sub/c.rs", "sub/deep/drop.log.txt", "sub/deep/keep.log"])
		);
	}

	#[test]
	fn ignore_files_can_be_turned_off() {
		let tree = Tree::new(&[(".gitignore", "*.log\n"), ("a.log", ""), (".hidden", ""), (".git/config", "")]);
		let options = WalkOptions { respect_ignores: false, ..defaults() };
		assert_eq!(tree.walked(options), names(&["a.log"]));
		let options = WalkOptions { respect_ignores: false, skip_hidden: false, ..defaults() };
		assert_eq!(tree.walked(options), names(&[".gitignore", ".hidden", "a.log"]));
	}

	#[test]
	fn git_exclude_applies() {
		let tree = Tree::new(&[(".git/info/exclude", "skipped.rs\n"), ("skipped.rs", ""), ("kept.rs", "")]);
		assert_eq!(tree.walked(defaults()), names(&["kept.rs"]));
	}

	#[test]
	fn ignore_files_above_the_root_apply() {
		let tree = Tree::new(&[
			(".gitignore", "*.log\nsub/skipped.rs\n"),
			("sub/a.log", ""),
			("sub/skipped.rs", ""),
			("sub/b.rs", ""),
		]);
		let found = Mutex::new(BTreeSet::new());
		walk(&tree.0.join("sub"), defaults(), &|| {
			|entry: Result<Entry, Error>| {
				found.lock().unwrap().insert(entry.unwrap().path().file_name().unwrap().to_string_lossy().into_owned());
			}
		});
		assert_eq!(found.into_inner().unwrap(), names(&["b.rs"]));
	}

	#[test]
	fn depth_limits_and_overrides_apply() {
		let tree = Tree::new(&[("a.rs", ""), ("sub/b.rs", ""), ("sub/deep/c.rs", ""), ("sub/d.txt", "")]);
		assert_eq!(tree.walked(WalkOptions { max_depth: Some(1), ..defaults() }), names(&["a.rs"]));
		assert_eq!(
			tree.walked(WalkOptions { max_depth: Some(2), ..defaults() }),
			names(&["a.rs", "sub/b.rs", "sub/d.txt"])
		);
		assert_eq!(tree.walked(WalkOptions { max_depth: Some(0), ..defaults() }), names(&[]));
		let mut overrides = ignore::overrides::OverrideBuilder::new(&tree.0);
		overrides.add("!*.txt").unwrap();
		let options = WalkOptions { overrides: Some(overrides.build().unwrap()), ..defaults() };
		assert_eq!(tree.walked(options), names(&["a.rs", "sub/b.rs", "sub/deep/c.rs"]));
	}

	#[test]
	fn every_file_of_a_large_tree_is_visited_once() {
		let paths: Vec<String> = (0..2000).map(|i| format!("{}/{}/{i}.rs", i % 7, i % 31)).collect();
		let files: Vec<(&str, &str)> = paths.iter().map(|path| (path.as_str(), "")).collect();
		let tree = Tree::new(&files);
		let visits = AtomicUsize::new(0);
		for threads in [1, 2, 16] {
			visits.store(0, Ordering::Relaxed);
			let options = WalkOptions { threads, ..defaults() };
			walk(&tree.0, options, &|| {
				|entry: Result<Entry, Error>| {
					entry.unwrap();
					visits.fetch_add(1, Ordering::Relaxed);
				}
			});
			assert_eq!(visits.load(Ordering::Relaxed), 2000);
		}
		assert_eq!(tree.walked(defaults()).len(), 2000);
	}

	#[test]
	fn a_file_as_the_root_is_visited_even_if_ignored() {
		let tree = Tree::new(&[(".gitignore", "*.rs\n"), ("a.rs", "")]);
		let visits = AtomicUsize::new(0);
		walk(&tree.0.join("a.rs"), defaults(), &|| {
			|entry: Result<Entry, Error>| {
				entry.unwrap();
				visits.fetch_add(1, Ordering::Relaxed);
			}
		});
		assert_eq!(visits.load(Ordering::Relaxed), 1);
	}

	#[test]
	fn a_missing_root_is_an_error() {
		let errors = AtomicUsize::new(0);
		walk(Path::new("/nonexistent/codestats/walk/test"), defaults(), &|| {
			|entry: Result<Entry, Error>| {
				assert!(entry.is_err());
				errors.fetch_add(1, Ordering::Relaxed);
			}
		});
		assert_eq!(errors.load(Ordering::Relaxed), 1);
	}

	#[cfg(unix)]
	#[test]
	fn links_are_followed_only_when_asked_and_loops_end() {
		let tree = Tree::new(&[("real/a.rs", ""), ("target.rs", "")]);
		std::os::unix::fs::symlink(tree.0.join("real"), tree.0.join("link")).unwrap();
		std::os::unix::fs::symlink(tree.0.join("target.rs"), tree.0.join("file_link.rs")).unwrap();
		std::os::unix::fs::symlink(tree.0.join("real"), tree.0.join("real/up")).unwrap();
		assert_eq!(tree.walked(defaults()), names(&["real/a.rs", "target.rs"]));
		let found = Mutex::new(BTreeSet::new());
		let loops = AtomicUsize::new(0);
		let options = WalkOptions { follow_links: true, ..defaults() };
		walk(&tree.0, options, &|| {
			|entry: Result<Entry, Error>| match entry {
				Ok(entry) => {
					let relative = entry.path().strip_prefix(&tree.0).unwrap().to_string_lossy().into_owned();
					found.lock().unwrap().insert(relative);
				}
				Err(_) => {
					loops.fetch_add(1, Ordering::Relaxed);
				}
			}
		});
		assert_eq!(found.into_inner().unwrap(), names(&["file_link.rs", "link/a.rs", "real/a.rs", "target.rs"]));
		// `real/up` and `link/up` each lead back to a directory above them.
		assert_eq!(loops.load(Ordering::Relaxed), 2);
	}
}
