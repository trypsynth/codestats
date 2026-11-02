//! Counting the regions of a file written in another language (a `<script>` in HTML, a fenced code block in Markdown) by that language's rules. The lines still belong to the file's own language.

use std::{mem, sync::LazyLock};

use memchr::memchr_iter;

use super::{
	line_classifier::{CommentState, LineType, classify_line, is_blank},
	stats::Embedded,
};
use crate::langs::{Embedding, LANGUAGES, Language, detect_language, get_candidates};

/// Classifies a file's lines in order, following the regions its language embeds others in.
pub(super) struct Classifier<'a> {
	language: &'a Language,
	state: CommentState,
	region: Region,
	embedded: Embedded,
}

enum Region {
	Host,
	/// Inside a `<script` or `<style` tag whose `>` is on a later line, collecting its attributes.
	OpeningTag {
		tag: Tag,
		attributes: String,
	},
	/// After an `AsciiDoc` `[source,python]` line, which says what the listing block it's followed by is written in.
	SourceAttribute(Option<&'static Language>),
	/// A PHP file's HTML, outside its `<?php ... ?>` tags, read by a classifier of its own that follows HTML's elements, made once a line of HTML needs it.
	Markup(Option<Box<Classifier<'static>>>),
	/// Inside an embedded region. Its lines are classified as the host's when its language isn't known.
	Embedded {
		language: Option<&'static Language>,
		state: CommentState,
		end: End,
	},
}

#[derive(Clone, Copy)]
enum Tag {
	Script,
	Style,
}

impl Tag {
	const fn name(self) -> &'static [u8] {
		match self {
			Self::Script => b"script",
			Self::Style => b"style",
		}
	}
}

/// What ends an embedded region.
enum End {
	/// A line of just this (`---` or `+++`).
	Frontmatter(&'static str),
	/// The element's closing tag.
	Tag(Tag),
	/// A fence of at least `len` `marker`s and nothing else.
	Fence { marker: u8, len: usize },
	/// A line of exactly `len` `marker`s, like the `----` that opened an `AsciiDoc` listing.
	Delimiter { marker: u8, len: usize },
	/// Org's `#+END_SRC`.
	OrgSource,
	/// A line indented no deeper than a reStructuredText directive, which isn't part of the region but follows it. Until the code starts, `preamble` is set: the directive's options (`:linenos:`) and the blank line after them are the host's. `blanks` counts the blank lines since the last line of code, which only belong to the region if more code follows.
	Dedent { indent: usize, preamble: bool, blanks: u64 },
}

impl End {
	fn closes(&self, line: &str) -> bool {
		match *self {
			Self::Frontmatter(marker) => line.trim_end() == marker,
			Self::Tag(tag) => memchr_iter(b'<', line.as_bytes()).any(|pos| {
				line.as_bytes()[pos + 1..]
					.strip_prefix(b"/")
					.is_some_and(|rest| starts_with_ignore_case(rest, tag.name()))
			}),
			Self::Fence { marker, len } => fence(line)
				.is_some_and(|(found, found_len, info)| found == marker && found_len >= len && info.is_empty()),
			Self::Delimiter { marker, len } => {
				let line = line.trim_end();
				line.len() == len && line.bytes().all(|b| b == marker)
			}
			Self::OrgSource => starts_with_ignore_case(line.trim_start().as_bytes(), b"#+end_src"),
			Self::Dedent { indent, .. } => !is_blank(line.as_bytes()) && indentation(line) <= indent,
		}
	}
}

impl<'a> Classifier<'a> {
	pub(super) fn new(language: &'a Language) -> Self {
		// A PHP file is HTML until its first `<?php`.
		let region = if language.embedding == Some(Embedding::Php) { Region::Markup(None) } else { Region::Host };
		Self { language, state: CommentState::new(), region, embedded: Embedded::default() }
	}

	/// Whether the next line starts in the file's own language, outside any comment or string.
	pub(super) const fn is_clear(&self) -> bool {
		matches!(self.region, Region::Host) && self.state.is_clear()
	}

	pub(super) fn classify(&mut self, line: &str, is_first_line: bool) -> LineType {
		let Some(embedding) = self.language.embedding else {
			return classify_line(line, self.language, &mut self.state, is_first_line);
		};
		if embedding == Embedding::Php {
			return self.classify_php(line, is_first_line);
		}
		match &mut self.region {
			Region::Host | Region::Markup(_) => {}
			Region::OpeningTag { tag, attributes } => {
				let Some(end) = line.find('>') else {
					attributes.push_str(line);
					return if is_blank(line.as_bytes()) { LineType::Blank } else { LineType::Code };
				};
				attributes.push_str(&line[..end]);
				self.region = element_region(*tag, &mem::take(attributes), &line[end + 1..]);
				return LineType::Code;
			}
			Region::SourceAttribute(language) => {
				let language = *language;
				self.region = Region::Host;
				if let Some(end) = listing_delimiter(line) {
					self.region = Region::Embedded { language, state: CommentState::new(), end };
					return LineType::Code;
				}
			}
			Region::Embedded { language, state, end } => {
				if end.closes(line) {
					let follows = matches!(end, End::Dedent { .. });
					self.region = Region::Host;
					if !follows {
						return LineType::Code;
					}
				} else {
					if let End::Dedent { preamble: preamble @ true, .. } = end {
						if is_blank(line.as_bytes()) {
							return LineType::Blank;
						}
						if line.trim_start().starts_with(':') {
							return LineType::Code;
						}
						*preamble = false;
					}
					let Some(language) = *language else {
						return classify_line(line, self.language, &mut self.state, false);
					};
					let line_type = classify_line(line, language, state, false);
					if let End::Dedent { blanks, .. } = end {
						if line_type == LineType::Blank {
							*blanks += 1;
							return line_type;
						}
						for _ in 0..mem::take(blanks) {
							self.embedded.add(language, LineType::Blank);
						}
					}
					self.embedded.add(language, line_type);
					return line_type;
				}
			}
		}
		if is_first_line && let Some((marker, language)) = frontmatter(embedding, line) {
			self.region = Region::Embedded { language, state: CommentState::new(), end: End::Frontmatter(marker) };
			return LineType::Code;
		}
		if self.state.is_clear()
			&& let Some(region) = self.region_start(embedding, line)
		{
			self.region = region;
			return LineType::Code;
		}
		classify_line(line, self.language, &mut self.state, is_first_line)
	}

	/// The lines classified so far that were in a region of a known language.
	pub(super) fn into_embedded(mut self) -> Embedded {
		if let Region::Markup(Some(markup)) = mem::replace(&mut self.region, Region::Host) {
			self.embedded.merge(&markup.into_embedded());
		}
		self.embedded
	}

	/// A line of a PHP file: HTML until a `<?php` tag, whose line is PHP's, and PHP until a `?>` outside its strings and comments, whose line is too.
	fn classify_php(&mut self, line: &str, is_first_line: bool) -> LineType {
		let in_markup = matches!(self.region, Region::Markup(_));
		if let Region::Markup(markup) = &mut self.region
			&& let Some(html) = *HTML
		{
			if !opens_php(line) {
				let markup = markup.get_or_insert_with(|| Box::new(Classifier::new(html)));
				let before = markup.embedded.total();
				let line_type = markup.classify(line, is_first_line);
				// Lines of the HTML's own scripts and styles are already counted as theirs.
				if markup.embedded.total() == before {
					self.embedded.add(html, line_type);
				}
				return line_type;
			}
			if let Region::Markup(Some(markup)) = mem::replace(&mut self.region, Region::Host) {
				self.embedded.merge(&markup.into_embedded());
			}
		}
		let starts_clear = in_markup || self.state.is_clear();
		let line_type = classify_line(line, self.language, &mut self.state, is_first_line);
		// Only a line with a closing tag can leave PHP, and only where the tag isn't in a string or block comment.
		if starts_clear
			&& self.state.is_clear()
			&& contains_pair(line.as_bytes(), *b"?>")
			&& php_line_ends_in_markup(line.as_bytes(), in_markup)
		{
			self.region = Region::Markup(None);
		}
		line_type
	}

	/// The region a line of the file's own language starts, if it starts one.
	fn region_start(&self, embedding: Embedding, line: &str) -> Option<Region> {
		let trimmed = line.trim_start();
		let embedded = |language, end| Region::Embedded { language, state: CommentState::new(), end };
		match embedding {
			// PHP's regions are followed by `classify_php`.
			Embedding::Php => None,
			Embedding::Markdown | Embedding::AsciiDoc => {
				if let Some((marker, len, info)) = fence(line) {
					return Some(embedded(info_language(info), End::Fence { marker, len }));
				}
				(embedding == Embedding::AsciiDoc).then(|| source_attribute(line)).flatten()
			}
			Embedding::Org => starts_with_ignore_case(trimmed.as_bytes(), b"#+begin_src")
				.then(|| embedded(info_language(trimmed["#+begin_src".len()..].trim()), End::OrgSource)),
			Embedding::ReStructuredText => [".. code-block::", ".. code::", ".. sourcecode::"]
				.into_iter()
				.find_map(|directive| trimmed.strip_prefix(directive))
				.map(|info| {
					embedded(
						info_language(info.trim()),
						End::Dedent { indent: indentation(line), preamble: true, blanks: 0 },
					)
				}),
			Embedding::Html | Embedding::Astro => {
				let (tag, name_end) = self.find_element(line)?;
				let rest = &line[name_end..];
				let Some(end) = rest.find('>') else {
					return Some(Region::OpeningTag { tag, attributes: rest.to_string() });
				};
				// A self-closing or one-line element (`<script src="app.js"></script>`) is just markup.
				(!rest[..end].ends_with('/') && !End::Tag(tag).closes(&rest[end + 1..]))
					.then(|| element_region(tag, &rest[..end], &rest[end + 1..]))
			}
		}
	}

	/// The first `<script` or `<style` opening tag on `line` that isn't in a comment, and where its name ends.
	fn find_element(&self, line: &str) -> Option<(Tag, usize)> {
		let bytes = line.as_bytes();
		let (pos, tag) = memchr_iter(b'<', bytes).find_map(|pos| {
			[Tag::Script, Tag::Style].into_iter().find_map(|tag| {
				let rest = &bytes[pos + 1..];
				let name_ends =
					rest.get(tag.name().len()).is_none_or(|&b| b == b'>' || b == b'/' || b.is_ascii_whitespace());
				(starts_with_ignore_case(rest, tag.name()) && name_ends).then_some((pos, tag))
			})
		})?;
		let before = &line[..pos];
		let commented = self
			.language
			.block_comments
			.iter()
			.any(|(start, end, _)| before.rfind(start).is_some_and(|open| !before[open..].contains(end)));
		(!commented).then_some((tag, pos + 1 + tag.name().len()))
	}
}

/// The language of a PHP file's markup.
static HTML: LazyLock<Option<&'static Language>> = LazyLock::new(|| language_named("HTML"));

/// The PHP tags on `line` in order, each with whether it opens PHP (`<?php`, `<?=`, or a short `<?`) rather than closing it (`?>`).
fn php_tags(line: &str) -> impl Iterator<Item = bool> + '_ {
	let bytes = line.as_bytes();
	memchr::memchr2_iter(b'<', b'?', bytes).filter_map(move |pos| match bytes[pos] {
		b'<' => {
			let rest = bytes[pos + 1..].strip_prefix(b"?")?;
			let opens = starts_with_ignore_case(rest, b"php")
				|| rest.first().is_none_or(|&b| b == b'=' || b.is_ascii_whitespace());
			opens.then_some(true)
		}
		_ => (bytes.get(pos + 1) == Some(&b'>') && (pos == 0 || bytes[pos - 1] != b'<')).then_some(false),
	})
}

/// Whether `bytes` holds `pair` anywhere.
pub(super) fn contains_pair(bytes: &[u8], [first, second]: [u8; 2]) -> bool {
	memchr_iter(first, bytes).any(|pos| bytes.get(pos + 1) == Some(&second))
}

/// Whether `line` has a tag that opens PHP.
fn opens_php(line: &str) -> bool {
	php_tags(line).any(|opens| opens)
}

/// Whether a PHP file is in its HTML after `line`, which started in it if `in_markup` and otherwise outside any string or comment. A `?>` in a string or block comment is text, while one in a line comment ends PHP, as PHP reads it.
fn php_line_ends_in_markup(line: &[u8], in_markup: bool) -> bool {
	let mut markup = in_markup;
	let mut pos = 0;
	while pos < line.len() {
		let rest = &line[pos..];
		if markup {
			let Some(open) = memchr::memmem::find(rest, b"<?") else {
				return true;
			};
			let after = &rest[open + 2..];
			let opens = starts_with_ignore_case(after, b"php")
				|| after.first().is_none_or(|&b| b == b'=' || b.is_ascii_whitespace());
			markup = !opens;
			pos += open + 2;
			continue;
		}
		match rest {
			[b'?', b'>', ..] => {
				markup = true;
				pos += 2;
			}
			[quote @ (b'\'' | b'"'), ..] => {
				// To the matching quote, past escaped ones; a string left open carries PHP on to the next line.
				let mut end = pos + 1;
				loop {
					match line.get(end) {
						None => return false,
						Some(b'\\') => end += 2,
						Some(b) if b == quote => break,
						Some(_) => end += 1,
					}
				}
				pos = end + 1;
			}
			[b'/', b'*', ..] => match memchr::memmem::find(&rest[2..], b"*/") {
				Some(end) => pos += 2 + end + 2,
				None => return false,
			},
			// A line comment runs to the end of the line, or to a `?>` before it.
			[b'/', b'/', ..] | [b'#', ..] if !rest.starts_with(b"#[") => match memchr::memmem::find(rest, b"?>") {
				Some(close) => {
					markup = true;
					pos += close + 2;
				}
				None => return false,
			},
			_ => pos += 1,
		}
	}
	markup
}

/// Which lines of a file's own language might start a region, where that can be told from their bytes alone; other lines can be swept like any language's.
#[derive(Debug, Clone, Copy)]
pub(super) enum RegionStarts {
	/// Lines whose first byte after indentation is one of these.
	LineStart(&'static [u8]),
	/// Lines holding these two bytes together anywhere (PHP's `?>`).
	Anywhere([u8; 2]),
}

/// Which lines might start a region in `embedding`'s files, or `None` where every line has to be classified.
pub(super) const fn region_starts(embedding: Embedding) -> Option<RegionStarts> {
	match embedding {
		Embedding::Html | Embedding::Astro => None,
		Embedding::Markdown => Some(RegionStarts::LineStart(b"`~")),
		Embedding::AsciiDoc => Some(RegionStarts::LineStart(b"`~[")),
		Embedding::Org => Some(RegionStarts::LineStart(b"#")),
		Embedding::ReStructuredText => Some(RegionStarts::LineStart(b".")),
		Embedding::Php => Some(RegionStarts::Anywhere(*b"?>")),
	}
}

/// An element's region, starting after its opening tag and carrying on with whatever follows the tag on its line.
fn element_region(tag: Tag, attributes: &str, rest: &str) -> Region {
	let language = element_language(tag, attributes);
	let mut state = CommentState::new();
	if let Some(language) = language
		&& !is_blank(rest.as_bytes())
	{
		classify_line(rest, language, &mut state, false);
	}
	Region::Embedded { language, state, end: End::Tag(tag) }
}

/// The frontmatter a file's first line opens, as its closing marker and language.
fn frontmatter(embedding: Embedding, line: &str) -> Option<(&'static str, Option<&'static Language>)> {
	let (marker, name) = match (embedding, line.trim_end()) {
		(Embedding::Markdown, "---") => ("---", "YAML"),
		(Embedding::Markdown, "+++") => ("+++", "TOML"),
		(Embedding::Astro, "---") => ("---", "TypeScript"),
		_ => return None,
	};
	Some((marker, language_named(name)))
}

/// The region an `AsciiDoc` block attribute line naming source code (`[source,python]`, or `[,python]` for short) starts, which waits to see if a listing block follows.
fn source_attribute(line: &str) -> Option<Region> {
	let attributes = line.trim().strip_prefix('[')?.strip_suffix(']')?;
	let mut attributes = attributes.split(',').map(str::trim);
	let style = attributes.next()?;
	(style == "source" || style.is_empty()).then(|| Region::SourceAttribute(attributes.next().and_then(info_language)))
}

/// What ends an `AsciiDoc` listing block opened by `line`, if it opens one: `----` (or a longer run), or a fence.
fn listing_delimiter(line: &str) -> Option<End> {
	let trimmed = line.trim_end();
	if trimmed.len() >= 4 && trimmed.bytes().all(|b| b == b'-') {
		return Some(End::Delimiter { marker: b'-', len: trimmed.len() });
	}
	fence(line).map(|(marker, len, _)| End::Fence { marker, len })
}

/// How many spaces and tabs `line` starts with.
fn indentation(line: &str) -> usize {
	line.bytes().take_while(|&b| b == b' ' || b == b'\t').count()
}

/// A Markdown code fence: its marker, how many there are, and the info string after them.
fn fence(line: &str) -> Option<(u8, usize, &str)> {
	let trimmed = line.trim_start();
	let marker = *trimmed.as_bytes().first()?;
	if marker != b'`' && marker != b'~' {
		return None;
	}
	let len = trimmed.bytes().take_while(|&b| b == marker).count();
	let info = trimmed[len..].trim();
	// A backtick in the info string makes it inline code (```` ```x``` ````), not a fence.
	(len >= 3 && !(marker == b'`' && info.contains('`'))).then_some((marker, len, info))
}

/// The language a fence's info string names, by name (`rust`, `C++`) or extension (`py`, `sh`).
fn info_language(info: &str) -> Option<&'static Language> {
	// Skips past attributes: `rust,ignore`, R Markdown's `{r setup}` and Pandoc's `{.python}`.
	let name = info
		.trim_start_matches(['{', '.'])
		.split(|c: char| c.is_whitespace() || matches!(c, ',' | '{' | '}'))
		.next()?;
	if name.is_empty() {
		return None;
	}
	// Names in common use that are neither a language's name nor its extension.
	let lowercase = name.to_ascii_lowercase();
	let name = match lowercase.as_str() {
		"shell" => "bash",
		"python3" | "py3" => "python",
		"elisp" | "emacs-lisp" => "emacs lisp",
		"golang" => "go",
		"objc" | "objectivec" => "objective-c",
		"csharp" => "c#",
		"fsharp" => "f#",
		"latex" => "tex",
		"protobuf" => "proto",
		_ => name,
	};
	language_named(name).or_else(|| {
		// Only plain names make a filename; anything else could be read as a glob or path.
		name.bytes()
			.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'_'))
			.then(|| detect_language(&get_candidates(&format!("x.{name}")), None).language)
			.flatten()
	})
}

/// The language of a `<script>` or `<style>` element, from its `lang` or `type` attribute.
fn element_language(tag: Tag, attributes: &str) -> Option<&'static Language> {
	let lang = attribute(attributes, "lang").map(str::to_ascii_lowercase);
	let name = match (tag, lang.as_deref()) {
		(Tag::Script, Some("ts" | "tsx" | "typescript")) => "TypeScript",
		(Tag::Script, Some("js" | "jsx" | "javascript")) => "JavaScript",
		(Tag::Script, Some("coffee" | "coffeescript")) => "CoffeeScript",
		(Tag::Script, None) => match attribute(attributes, "type").map(str::to_ascii_lowercase).as_deref() {
			None
			| Some(
				""
				| "module"
				| "text/javascript"
				| "application/javascript"
				| "application/x-javascript"
				| "text/ecmascript"
				| "application/ecmascript"
				| "text/babel"
				| "text/jsx",
			) => "JavaScript",
			Some("text/typescript" | "application/typescript") => "TypeScript",
			Some("application/json" | "application/ld+json" | "importmap" | "speculationrules") => "JSON",
			// Templates (`text/x-template`), shaders and the like.
			Some(_) => return None,
		},
		(Tag::Style, None | Some("css")) => "CSS",
		(Tag::Style, Some("postcss")) => "PostCSS",
		(Tag::Style, Some("scss")) => "SCSS",
		(Tag::Style, Some("sass")) => "Sass",
		(Tag::Style, Some("less")) => "Less",
		(Tag::Style, Some("stylus" | "styl")) => "Stylus",
		_ => return None,
	};
	language_named(name)
}

/// The value of attribute `name` in an opening tag's attributes, without its quotes.
fn attribute<'a>(attributes: &'a str, name: &str) -> Option<&'a str> {
	attributes.split_ascii_whitespace().find_map(|attribute| {
		let (key, value) = attribute.split_once('=')?;
		key.eq_ignore_ascii_case(name).then(|| value.trim_end_matches('/').trim_matches(['"', '\'']))
	})
}

/// The language called `name`, ignoring case, found by binary search as `languages.json5` keeps them in order.
fn language_named(name: &str) -> Option<&'static Language> {
	let key = |text: &str| -> Vec<u8> { text.bytes().map(|b| b.to_ascii_lowercase()).collect() };
	let name = key(name);
	LANGUAGES.binary_search_by(|language| key(language.name).cmp(&name)).ok().map(|index| &LANGUAGES[index])
}

fn starts_with_ignore_case(bytes: &[u8], prefix: &[u8]) -> bool {
	bytes.get(..prefix.len()).is_some_and(|start| start.eq_ignore_ascii_case(prefix))
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;
	use crate::analysis::{
		LineType::{Blank, Code, Comment},
		encoding::FileEncoding,
		line_counter::count_lines,
		stats::LineCounts,
	};

	fn classify_all(language: &str, text: &str) -> Vec<LineType> {
		let language = language_named(language).unwrap();
		let mut classifier = Classifier::new(language);
		text.lines().enumerate().map(|(i, line)| classifier.classify(line, i == 0)).collect()
	}

	#[rstest]
	#[case::vue_script(
		"Vue",
		"<template>\n  <p>hi</p>\n</template>\n<script setup lang=\"ts\">\n// note\nconst x: number = 1;\n/* a\n b */\n</script>",
		&[Code, Code, Code, Code, Comment, Code, Comment, Comment, Code]
	)]
	#[case::html_style("HTML", "<style>\n/* note */\np { color: red; }\n\n</style>\n<!-- c -->", &[Code, Comment, Code, Blank, Code, Comment])]
	#[case::one_line_script("HTML", "<script src=\"a.js\"></script>\n// text", &[Code, Code])]
	#[case::script_in_comment("HTML", "<!-- <script>\n// x\n-->", &[Comment, Comment, Comment])]
	#[case::multiline_opening_tag("Svelte", "<script\n  lang=\"ts\"\n>\n// x\n</script>", &[Code, Code, Code, Comment, Code])]
	#[case::comment_after_opening_tag("HTML", "<script>/* a\nb */\n</script>", &[Code, Comment, Code])]
	#[case::template_script("HTML", "<script type=\"text/x-template\">\n<!-- c -->\n</script>", &[Code, Comment, Code])]
	#[case::astro_frontmatter("Astro", "---\n// note\nconst a = 1;\n---\n<p>{a}</p>", &[Code, Comment, Code, Code, Code])]
	#[case::markdown_fence(
		"Markdown",
		"# Title\n\n```rust\n// note\nfn main() {}\n```\nText",
		&[Code, Blank, Code, Comment, Code, Code, Code]
	)]
	#[case::markdown_fence_by_extension("Markdown", "~~~py\n# note\n~~~", &[Code, Comment, Code])]
	#[case::markdown_unknown_fence("Markdown", "```\n// not a comment\n```", &[Code, Code, Code])]
	#[case::markdown_longer_fence("Markdown", "````md\n```rust\n// x\n```\n````\n# heading", &[Code, Code, Code, Code, Code, Code])]
	#[case::markdown_frontmatter("Markdown", "---\ntitle: x # note\n# note\n---\n# heading", &[Code, Code, Comment, Code, Code])]
	#[case::rmarkdown_chunk("Markdown", "```{r setup, include=FALSE}\n# note\n```", &[Code, Comment, Code])]
	#[case::asciidoc_listing("AsciiDoc", "[source,python]\n----\n# note\nx = 1\n----\n// c", &[Code, Code, Comment, Code, Code, Comment])]
	#[case::asciidoc_attribute_without_block("AsciiDoc", "[source,python]\nx # y", &[Code, Code])]
	#[case::org_source("Org", "#+begin_src python :results output\n# note\n#+END_SRC\n# c", &[Code, Comment, Code, Comment])]
	#[case::rst_directive(
		"reStructuredText",
		".. code-block:: python\n   :linenos:\n\n   # note\n   x = 1\nText",
		&[Code, Code, Blank, Comment, Code, Code]
	)]
	#[case::php_markup("PHP", "<?php\n?>\n<p>hi</p>\n<!-- c -->\n<?php\necho \"hi\"; // c\n?>", &[Code, Code, Code, Comment, Code, Code, Code])]
	#[case::php_starts_in_markup("PHP", "<!-- c -->\n<?php // c\n# c\n", &[Comment, Code, Comment])]
	#[case::php_closer_in_a_string("PHP", "<?php\n$s = \"\n?>\n# not html\";\n// c", &[Code, Code, Code, Code, Comment])]
	#[case::php_closer_in_a_one_line_string("PHP", "<?php\n$h = \"<?x ?>\";\n// c\n/* ?> */\n# c", &[Code, Code, Comment, Comment, Comment])]
	#[case::php_closer_in_a_line_comment("PHP", "<?php\n// c ?>\n<!-- c -->", &[Code, Comment, Comment])]
	#[case::php_inline_tags("PHP", "<?php\n?>\n<p><?= $x ?></p>\n<!-- c -->", &[Code, Code, Code, Comment])]
	fn embedded_regions(#[case] language: &str, #[case] text: &str, #[case] expected: &[LineType]) {
		assert_eq!(classify_all(language, text), expected);
	}

	/// A PHP file's HTML counts as HTML, and the scripts in it as theirs, whatever language is innermost.
	#[test]
	fn php_markup_counts_as_html_and_its_scripts_as_theirs() {
		let php = language_named("PHP").unwrap();
		let mut classifier = Classifier::new(php);
		let text = "<?php\n?>\n<div>\n<script>\n// c\nvar x = 1;\n</script>\n</div>\n<?php\necho 1;\n?>";
		for (i, line) in text.split('\n').enumerate() {
			classifier.classify(line, i == 0);
		}
		let embedded = classifier.into_embedded().languages();
		let totals: Vec<_> = embedded.iter().map(|(language, counts)| (language.name, counts.total)).collect();
		assert_eq!(totals, [("HTML", 4), ("JavaScript", 2)]);
	}

	/// A xorshift generator of numbers below a bound, so a failure reproduces.
	fn rng(mut seed: u64) -> impl FnMut(usize) -> usize {
		move |bound| {
			seed ^= seed << 13;
			seed ^= seed >> 7;
			seed ^= seed << 17;
			usize::try_from(seed % bound as u64).unwrap()
		}
	}

	fn random_lines(pieces: &[&str], next: &mut impl FnMut(usize) -> usize) -> Vec<String> {
		let text: String = (0..next(40)).map(|_| pieces[next(pieces.len())]).collect();
		text.split('\n').map(str::to_string).collect()
	}

	/// A region of random text in `language`, appended to `lines` with the line types it should get.
	/// How a region is written: the lines that open it, the one that closes it (none where the next line of the host does), what the classifier must take as its end, and the indentation its lines need.
	struct Syntax {
		opening: Vec<String>,
		closing: Option<&'static str>,
		end: End,
		indent: &'static str,
	}

	/// A region of random text in `language`, appended to `lines` with the line types it should get.
	fn push_region(
		lines: &mut Vec<String>,
		expected: &mut Vec<LineType>,
		embedded: &mut Embedded,
		syntax: Syntax,
		language: &'static Language,
		next: &mut impl FnMut(usize) -> usize,
	) {
		for line in syntax.opening {
			lines.push(line);
			expected.push(Code);
		}
		let mut state = CommentState::new();
		// Blank lines at the start or end of an indented region belong to the host.
		let deferred = matches!(syntax.end, End::Dedent { .. });
		let mut started = !deferred;
		let mut blanks = 0;
		for line in random_lines(&markers(language), next) {
			let line = format!("{}{line}", syntax.indent);
			// A line that would end the region early isn't part of one.
			let line =
				if syntax.end.closes(&line) || fence(&line).is_some() { format!("{}x", syntax.indent) } else { line };
			let line_type = classify_line(&line, language, &mut state, false);
			expected.push(line_type);
			lines.push(line);
			if deferred && line_type == Blank {
				blanks += 1;
				continue;
			}
			for _ in 0..mem::take(&mut blanks) {
				if started {
					embedded.add(language, Blank);
				}
			}
			started = true;
			embedded.add(language, line_type);
		}
		if let Some(closing) = syntax.closing {
			lines.push(closing.to_string());
			expected.push(Code);
		}
	}

	fn markers(language: &Language) -> Vec<&'static str> {
		let mut pieces = vec!["a", "x = 1", " ", "\t", "\n", "\n", "\n", ";", "é", "(", "\\"];
		pieces.extend(language.line_comments);
		pieces.extend(language.block_comments.iter().flat_map(|&(start, end, _)| [start, end]));
		pieces.extend(language.strings.iter().flat_map(|delimiter| [delimiter.open, delimiter.close]));
		pieces.extend(language.column_one_comments);
		pieces
	}

	/// How a random region of a random language is written in `embedding`'s files.
	fn random_syntax(
		embedding: Embedding,
		next: &mut impl FnMut(usize) -> usize,
		named: &[&'static Language],
	) -> (Syntax, &'static Language) {
		let language = named[next(named.len())];
		let name = language.name;
		let syntax = match embedding {
			Embedding::Html | Embedding::Astro => {
				let (opening, closing, tag, name): (&[&str], _, _, _) = [
					(&["<script>"][..], "</script>", Tag::Script, "JavaScript"),
					(&["<script setup lang=\"ts\">"], "</script>", Tag::Script, "TypeScript"),
					(&["<script", "\ttype=\"module\"", ">"], "</SCRIPT>", Tag::Script, "JavaScript"),
					(&["<style>"], "</style>", Tag::Style, "CSS"),
					(&["<style scoped lang=\"scss\">"], "</style>", Tag::Style, "SCSS"),
				][next(5)];
				let opening = opening.iter().map(ToString::to_string).collect();
				let syntax = Syntax { opening, closing: Some(closing), end: End::Tag(tag), indent: "" };
				return (syntax, language_named(name).unwrap());
			}
			Embedding::Markdown => Syntax {
				opening: vec![format!("```{name}")],
				closing: Some("```"),
				end: End::Fence { marker: b'`', len: 3 },
				indent: "",
			},
			Embedding::AsciiDoc => Syntax {
				opening: vec![format!("[source,{name}]"), "----".to_string()],
				closing: Some("----"),
				end: End::Delimiter { marker: b'-', len: 4 },
				indent: "",
			},
			Embedding::Org => Syntax {
				opening: vec![format!("#+BEGIN_SRC {name} :results output")],
				closing: Some("#+end_src"),
				end: End::OrgSource,
				indent: "",
			},
			Embedding::ReStructuredText => Syntax {
				opening: vec![format!(".. code-block:: {name}"), "   :linenos:".to_string()],
				closing: None,
				end: End::Dedent { indent: 0, preamble: false, blanks: 0 },
				indent: "   ",
			},
			Embedding::Php => unreachable!("PHP files start in HTML, so they have tests of their own"),
		};
		(syntax, language)
	}

	/// The lines of an embedded region must count exactly as they would in a file of their own language, whatever comments or strings they leave open, and the host's lines as they would without the regions.
	#[test]
	fn regions_count_as_their_language_would_alone() {
		let mut next = rng(0x5EED_CAFE_F00D_0001);
		// Languages whose names can be written after a fence or in a directive.
		let named: Vec<&Language> = LANGUAGES
			.iter()
			.filter(|language| language.embedding.is_none() && info_language(language.name) == Some(*language))
			.collect();
		for host in
			LANGUAGES.iter().filter(|language| language.embedding.is_some_and(|embedding| embedding != Embedding::Php))
		{
			let embedding = host.embedding.unwrap();
			// Lines complete on their own, so they classify the same anywhere outside a region, and that can't be taken for one's start or end.
			let host_lines: &[&str] = match embedding {
				Embedding::Html | Embedding::Astro => {
					&["<p>text</p>", "<!-- note -->", "", "  <div>", "<script src=\"a.js\"></script>", "<br/>"]
				}
				Embedding::ReStructuredText => &["Some text", "Title", "=====", "- item"],
				_ => &["Some text", "<!-- note -->", "", "  - item", "<script>", "[NOTE]"],
			};
			for _ in 0..200 {
				let mut lines: Vec<String> = Vec::new();
				let mut expected = Vec::new();
				let mut embedded = Embedded::default();
				let frontmatter = match embedding {
					Embedding::Markdown => Some("YAML"),
					Embedding::Astro => Some("TypeScript"),
					_ => None,
				};
				if let Some(name) = frontmatter
					&& next(2) == 0
				{
					let syntax = Syntax {
						opening: vec!["---".to_string()],
						closing: Some("---"),
						end: End::Frontmatter("---"),
						indent: "",
					};
					let language = language_named(name).unwrap();
					push_region(&mut lines, &mut expected, &mut embedded, syntax, language, &mut next);
				}
				for _ in 0..next(6) {
					for _ in 0..next(3) {
						let line = host_lines[next(host_lines.len())];
						expected.push(classify_line(line, host, &mut CommentState::new(), false));
						lines.push(line.to_string());
					}
					let (syntax, language) = random_syntax(embedding, &mut next, &named);
					push_region(&mut lines, &mut expected, &mut embedded, syntax, language, &mut next);
				}
				let mut classifier = Classifier::new(host);
				let got: Vec<LineType> =
					lines.iter().enumerate().map(|(i, line)| classifier.classify(line, i == 0)).collect();
				assert_eq!(got, expected, "{}: {lines:#?}", host.name);
				assert_eq!(classifier.into_embedded().languages(), embedded.languages(), "{}: {lines:#?}", host.name);
			}
		}
	}

	/// Arbitrary fragments of markup and code mustn't panic, and counting a whole file must agree with classifying it line by line.
	#[test]
	fn fragments_dont_panic() {
		let mut next = rng(0x0BAD_F00D_1234_0002);
		let pieces = [
			"<script",
			"<style",
			"<SCRIPT",
			">",
			"/>",
			"</script>",
			"</STYLE>",
			" lang=\"ts\"",
			" lang=",
			" type='module'",
			" type=\"text/x-template\"",
			"```",
			"~~~",
			"```rust",
			"```{r}",
			"````",
			"[source,python]",
			"[,c]",
			"----",
			"#+BEGIN_SRC python",
			"#+end_src",
			".. code-block:: c",
			"   ",
			"\u{b}",
			"\u{a0}",
			"---",
			"+++",
			"<!--",
			"-->",
			"/*",
			"*/",
			"//",
			"#",
			"\"",
			"'",
			"`",
			"é",
			"x",
			" ",
			"\n",
			"\n",
			"\t",
			"\r\n",
		];
		for language in LANGUAGES.iter().filter(|language| language.embedding.is_some()) {
			for _ in 0..500 {
				let text: String = (0..next(80)).map(|_| pieces[next(pieces.len())]).collect();
				let mut classifier = Classifier::new(language);
				let mut expected = LineCounts::default();
				for (i, line) in text.split_inclusive('\n').enumerate() {
					expected.add(classifier.classify(line, i == 0));
				}
				let utf8 = FileEncoding { encoding: encoding_rs::UTF_8, bom_len: 0 };
				assert_eq!(count_lines(language, utf8, text.as_bytes()).lines, expected, "{}: {text:?}", language.name);
			}
		}
	}

	#[rstest]
	#[case::name("Rust", Some("Rust"))]
	#[case::extension("py", Some("Python"))]
	#[case::alias("golang", Some("Go"))]
	#[case::alias_to_extension("latex", Some("TeX/LaTeX"))]
	#[case::attributes("rust,ignore", Some("Rust"))]
	#[case::transcript("console", None)]
	fn info_strings(#[case] info: &str, #[case] expected: Option<&str>) {
		assert_eq!(info_language(info).map(|language| language.name), expected);
	}

	#[test]
	fn every_language_is_found_by_name() {
		for language in LANGUAGES {
			assert_eq!(language_named(&language.name.to_ascii_uppercase()), Some(language), "{}", language.name);
		}
	}

	#[test]
	fn embedded_languages_exist() {
		for name in [
			"JavaScript",
			"TypeScript",
			"CoffeeScript",
			"JSON",
			"CSS",
			"PostCSS",
			"SCSS",
			"Sass",
			"Less",
			"Stylus",
			"YAML",
			"TOML",
		] {
			assert!(language_named(name).is_some(), "{name}");
		}
	}
}
