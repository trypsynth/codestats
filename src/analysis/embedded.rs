//! Counting the regions of a file written in another language (a `<script>` in HTML, a fenced code block in Markdown) by that language's rules. The lines still belong to the file's own language.

use std::mem;

use memchr::memchr_iter;

use super::line_classifier::{CommentState, LineType, classify_line, is_blank};
use crate::langs::{Embedding, LANGUAGES, Language, detect_language, get_candidates};

/// Classifies a file's lines in order, following the regions its language embeds others in.
pub(super) struct Classifier<'a> {
	language: &'a Language,
	state: CommentState,
	region: Region,
}

enum Region {
	Host,
	/// Inside a `<script` or `<style` tag whose `>` is on a later line, collecting its attributes.
	OpeningTag {
		tag: Tag,
		attributes: String,
	},
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
		}
	}
}

impl<'a> Classifier<'a> {
	pub(super) fn new(language: &'a Language) -> Self {
		Self { language, state: CommentState::new(), region: Region::Host }
	}

	/// Whether the next line starts in the file's own language, outside any comment or string.
	pub(super) const fn is_clear(&self) -> bool {
		matches!(self.region, Region::Host) && self.state.is_clear()
	}

	pub(super) fn classify(&mut self, line: &str, is_first_line: bool) -> LineType {
		let Some(embedding) = self.language.embedding else {
			return classify_line(line, self.language, &mut self.state, is_first_line);
		};
		match &mut self.region {
			Region::Host => {}
			Region::OpeningTag { tag, attributes } => {
				let Some(end) = line.find('>') else {
					attributes.push_str(line);
					return if is_blank(line.as_bytes()) { LineType::Blank } else { LineType::Code };
				};
				attributes.push_str(&line[..end]);
				let (tag, attributes) = (*tag, mem::take(attributes));
				self.enter_element(tag, &attributes, &line[end + 1..]);
				return LineType::Code;
			}
			Region::Embedded { language, state, end } => {
				if end.closes(line) {
					self.region = Region::Host;
					return LineType::Code;
				}
				return match language {
					Some(language) => classify_line(line, language, state, false),
					None => classify_line(line, self.language, &mut self.state, false),
				};
			}
		}
		if is_first_line && let Some((marker, language)) = frontmatter(embedding, line) {
			self.region = Region::Embedded { language, state: CommentState::new(), end: End::Frontmatter(marker) };
			return LineType::Code;
		}
		if self.state.is_clear() {
			match embedding {
				Embedding::Markdown => {
					if let Some((marker, len, info)) = fence(line) {
						self.region = Region::Embedded {
							language: info_language(info),
							state: CommentState::new(),
							end: End::Fence { marker, len },
						};
						return LineType::Code;
					}
				}
				Embedding::Html | Embedding::Astro => {
					if let Some((tag, name_end)) = self.find_element(line) {
						let rest = &line[name_end..];
						let Some(end) = rest.find('>') else {
							self.region = Region::OpeningTag { tag, attributes: rest.to_string() };
							return LineType::Code;
						};
						// A self-closing or one-line element (`<script src="app.js"></script>`) is just markup.
						if !rest[..end].ends_with('/') && !End::Tag(tag).closes(&rest[end + 1..]) {
							self.enter_element(tag, &rest[..end], &rest[end + 1..]);
							return LineType::Code;
						}
					}
				}
			}
		}
		classify_line(line, self.language, &mut self.state, is_first_line)
	}

	/// Start an element's region after its opening tag, carrying on with whatever follows the tag on its line.
	fn enter_element(&mut self, tag: Tag, attributes: &str, rest: &str) {
		let language = element_language(tag, attributes);
		let mut state = CommentState::new();
		if let Some(language) = language
			&& !is_blank(rest.as_bytes())
		{
			classify_line(rest, language, &mut state, false);
		}
		self.region = Region::Embedded { language, state, end: End::Tag(tag) };
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

fn language_named(name: &str) -> Option<&'static Language> {
	LANGUAGES.iter().find(|language| language.name.eq_ignore_ascii_case(name))
}

fn starts_with_ignore_case(bytes: &[u8], prefix: &[u8]) -> bool {
	bytes.get(..prefix.len()).is_some_and(|start| start.eq_ignore_ascii_case(prefix))
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;
	use crate::analysis::LineType::{Blank, Code, Comment};

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
	fn embedded_regions(#[case] language: &str, #[case] text: &str, #[case] expected: &[LineType]) {
		assert_eq!(classify_all(language, text), expected);
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
