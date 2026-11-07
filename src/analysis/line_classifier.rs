mod strings;

use memchr::memchr2;

use self::strings::classify_with_strings;
use crate::langs::{
	Language, is_shebang_for,
	matchers::{BlockCommentMatchers, ByteSet, LanguageMatchers, NestedSearch, language_matchers},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineType {
	Code,
	Comment,
	Blank,
	Shebang,
}

impl LineType {
	#[must_use]
	pub const fn singular_label(self) -> &'static str {
		match self {
			Self::Code => "code",
			Self::Comment => "comment",
			Self::Blank => "blank",
			Self::Shebang => "shebang",
		}
	}

	#[must_use]
	pub const fn plural_label(self) -> &'static str {
		match self {
			Self::Code => "code",
			Self::Comment => "comments",
			Self::Blank => "blanks",
			Self::Shebang => "shebangs",
		}
	}

	#[must_use]
	pub const fn title_label(self) -> &'static str {
		match self {
			Self::Code => "Code",
			Self::Comment => "Comments",
			Self::Blank => "Blanks",
			Self::Shebang => "Shebangs",
		}
	}
}

/// How an open heredoc ends.
#[derive(Debug, Clone, PartialEq, Eq)]
struct HeredocEnd {
	/// The word on the line that ends it.
	terminator: Box<str>,
	/// Whether that word may be indented (`<<~EOF` and `<<-EOF`, and any heredoc in PHP), or must start its line.
	indented: bool,
}

/// Tracks block comment state across lines: how deeply nested we are, and which delimiter pair opened the comment.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CommentState {
	block_comment_depth: usize,
	pair: usize,
	/// The string literal left open by the previous line, as an index into the language's `strings`.
	string: Option<usize>,
	/// Whether the open string is a docstring, whose contents count as comment.
	doc_string: bool,
	/// The interpolation holes the next line is in, innermost last, as in JavaScript's `` `a ${f(`b ${c}`)} d` ``. Inside one, code is lexed until the bracket that closes it.
	holes: Vec<Hole>,
	/// What closes the open string when its writer chose it (`)xy"` for C++'s `R"xy(`), rather than the language.
	string_close: Option<Box<[u8]>>,
	/// The heredoc whose body the next line is in, which counts as being in a string.
	heredoc: Option<HeredocEnd>,
	/// A comment the next line is in that's decided line by line, rather than by markers anywhere in it.
	carried: Carried,
}

/// An open interpolation hole: the string it's in, as an index into the language's strings, and how many of its brackets are open inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Hole {
	string: usize,
	depth: u32,
}

/// A comment carried from one line to the next that isn't an ordinary block comment.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Carried {
	#[default]
	Nothing,
	/// A line comment ending in a backslash, which in C carries it onto the next line.
	SplicedLineComment,
	/// One of the language's line block comments (Perl's POD), as an index into them.
	LineBlock(usize),
}

impl CommentState {
	#[must_use]
	#[inline]
	pub(crate) fn new() -> Self {
		Self::default()
	}

	#[inline]
	const fn enter_first_block(&mut self, pair: usize) {
		self.block_comment_depth = 1;
		self.pair = pair;
	}

	/// Close one level of comment. Non-nesting pairs never go deeper than one level, so this fully exits them.
	#[inline]
	const fn exit_block(&mut self) {
		self.block_comment_depth = self.block_comment_depth.saturating_sub(1);
	}

	#[inline]
	const fn enter_nested_block(&mut self) {
		self.block_comment_depth = self.block_comment_depth.saturating_add(1);
	}

	#[must_use]
	#[inline]
	const fn is_in_comment(&self) -> bool {
		self.block_comment_depth > 0
	}

	/// Note that a line comment runs to the end of `line`, which carries it on to the next line if the line ends with a backslash in a language with C's line splices.
	#[inline]
	fn end_line_comment(&mut self, line: &str, lang: &Language) {
		if lang.line_splices {
			self.carried = if line.ends_with('\\') { Carried::SplicedLineComment } else { Carried::Nothing };
		}
	}

	#[must_use]
	#[inline]
	const fn is_in_string(&self) -> bool {
		self.string.is_some() || self.heredoc.is_some() || !self.holes.is_empty()
	}

	/// End the holes in a one-line string at the end of its line, as the string itself ends there.
	fn end_one_line_holes(&mut self, lang: &Language) {
		if self.holes.first().is_some_and(|hole| !lang.strings[hole.string].multiline) {
			self.holes.clear();
			self.string = None;
		}
	}

	/// The bytes a line would need to end the block comment or multi-line string the next line starts in, and what a line without them counts as. `None` outside those, or where a line takes more than that to judge.
	#[inline]
	pub(crate) fn what_ends_this<'m>(&self, matchers: &'m LanguageMatchers) -> Option<(&'m ByteSet, LineType)> {
		if self.heredoc.is_some() || !self.holes.is_empty() || !matches!(self.carried, Carried::Nothing) {
			return None;
		}
		if self.is_in_comment() {
			return matchers.block_comments.as_ref().map(|block| (block.ending_bytes(self.pair), LineType::Comment));
		}
		let index = self.string?;
		if self.string_close.is_some() {
			return None;
		}
		let strings = matchers.strings.as_ref()?;
		strings
			.multiline_ending_bytes(index)
			.map(|bytes| (bytes, if self.doc_string { LineType::Comment } else { LineType::Code }))
	}

	/// Whether the next line starts outside any comment or string.
	#[must_use]
	#[inline]
	pub(crate) const fn is_clear(&self) -> bool {
		!self.is_in_comment() && !self.is_in_string() && matches!(self.carried, Carried::Nothing)
	}
}

/// Process block comments on a line, updating state and detecting code.
/// Returns: (`remaining_line_portion`, `has_code_outside_comments`)
#[inline]
fn handle_block_comments<'a>(
	line: &'a str,
	lang: &Language,
	matchers: &LanguageMatchers,
	block_matchers: &BlockCommentMatchers,
	comment_state: &mut CommentState,
) -> (&'a str, bool) {
	let mut line_remainder = line;
	let mut has_code = false;
	let mut search = NestedSearch::default();
	while !line_remainder.is_empty() {
		if !comment_state.is_in_comment() {
			if let Some((pos, start_len, pair)) = block_matchers.find_block_start(line_remainder) {
				// A block start after a line comment marker is part of that comment (`// see "src/*"`), so leave the line comment for the caller.
				if find_line_comment(line_remainder, lang, matchers).is_some_and(|line_pos| line_pos < pos) {
					break;
				}
				if pos > 0 && contains_non_whitespace(&line_remainder[..pos]) {
					has_code = true;
				}
				line_remainder = &line_remainder[pos + start_len..];
				comment_state.enter_first_block(pair);
			} else {
				break;
			}
		} else if let Some((pos, len, found_nested_start)) =
			block_matchers.find_block_end_or_nested_start(line_remainder, comment_state.pair, &mut search)
		{
			if found_nested_start {
				comment_state.enter_nested_block();
			} else {
				comment_state.exit_block();
			}
			line_remainder = &line_remainder[pos + len..];
		} else {
			break;
		}
	}
	(line_remainder, has_code)
}

/// Whether a line is a C preprocessor directive written with a space after the `#` (`# define X`), in a language where `#` also starts comments. Directives that read like English (`# if the value...`) only count when followed by a condition.
fn is_preprocessor_directive(line: &str) -> bool {
	let Some(rest) = line.strip_prefix('#') else {
		return false;
	};
	let rest = rest.trim_start();
	let word_end = rest.find(|c: char| !c.is_ascii_alphanumeric() && c != '_').unwrap_or(rest.len());
	let (word, after) = rest.split_at(word_end);
	match word {
		"define" | "undef" | "ifdef" | "ifndef" | "endif" | "include" | "pragma" => true,
		"else" => after.trim().is_empty(),
		"if" | "elif" => {
			let condition = after.trim_start();
			condition.starts_with("defined")
				|| condition.starts_with(|c: char| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '(' || c == '!')
		}
		_ => false,
	}
}

/// Find the position of the first valid line comment marker in `line`.
#[inline]
fn find_line_comment(line: &str, lang: &Language, matchers: &LanguageMatchers) -> Option<usize> {
	let line_comments = matchers.line_comments.as_ref()?;
	// Most comment lines start with their marker, so check for that without running the automaton.
	if starts_with_line_comment(line, lang) {
		return Some(0);
	}
	line_comments
		.find_iter(line)
		.find(|matched| {
			is_valid_line_comment_match(line, matched.end(), lang.line_comments[matched.pattern().as_usize()])
		})
		.map(|matched| matched.start())
}

/// Whether `line` starts with one of `lang`'s line comment markers. Leftmost-first matching picks the first listed marker that matches at the start, so this finds the same one the automaton would.
#[inline]
fn starts_with_line_comment(line: &str, lang: &Language) -> bool {
	lang.line_comments
		.iter()
		.find(|marker| starts_with(line, marker))
		.is_some_and(|marker| is_valid_line_comment_match(line, marker.len(), marker))
}

/// `str::starts_with` for markers: they're a few bytes long, so comparing them byte by byte beats a call to `memcmp` on every comment line.
#[inline]
fn starts_with(line: &str, marker: &str) -> bool {
	line.len() >= marker.len() && line.bytes().zip(marker.bytes()).all(|(a, b)| a == b)
}

/// Whether `line`, after the first and read from a clear state, is just a line comment, as `classify_line` would find, and leaves the state clear: it starts with a line comment marker, after indentation, that doesn't start a block comment there too (Lua's `--[[`), and doesn't carry on to the next line with a backslash. `false` means the line needs the classifier, not that it isn't a comment.
#[inline]
pub(super) fn is_plain_line_comment(line: &str, lang: &Language) -> bool {
	let trimmed = trim_ascii(line);
	starts_with_line_comment(trimmed, lang)
		&& !lang.block_comments.iter().any(|(start, _, _)| starts_with(trimmed, start))
		&& lang.line_block_comments.is_empty()
		&& !(lang.line_splices && trimmed.ends_with('\\'))
}

/// Classify a line as code, comment, blank, or shebang.
#[inline]
pub fn classify_line(line: &str, lang: &Language, comment_state: &mut CommentState, is_first_line: bool) -> LineType {
	let trimmed = trim_ascii(line);
	// Outside every comment and string, the common case, which the checks below need to know.
	let clear = comment_state.is_clear();
	if !clear {
		match comment_state.carried {
			Carried::Nothing => {}
			Carried::SplicedLineComment => {
				comment_state.end_line_comment(trimmed, lang);
				return if trimmed.is_empty() { LineType::Blank } else { LineType::Comment };
			}
			Carried::LineBlock(pair) => {
				if is_line_block_marker(line, trimmed, lang.line_block_comments[pair].1, lang) {
					comment_state.carried = Carried::Nothing;
				}
				return if trimmed.is_empty() { LineType::Blank } else { LineType::Comment };
			}
		}
	}
	if trimmed.is_empty() {
		// A one-line string goes on past a line ending in a backslash, but not past a line that doesn't, like this one.
		if !clear && comment_state.string.is_some_and(|i| !lang.strings[i].multiline) {
			comment_state.string = None;
		}
		if !clear {
			comment_state.end_one_line_holes(lang);
		}
		return LineType::Blank;
	}
	if is_first_line && is_shebang_for(trimmed, lang) {
		return LineType::Shebang;
	}
	// Markers like fixed-form Fortran's `C` only count in the first column, so check the untrimmed line.
	if clear && lang.column_one_comments.iter().any(|marker| line.starts_with(marker)) {
		return LineType::Comment;
	}
	if lang.c_preprocessor && clear && is_preprocessor_directive(trimmed) {
		return LineType::Code;
	}
	let matchers = language_matchers(lang);
	// Outside a comment or string, a line with no byte that could start one is code: every search below would come up empty.
	if clear && !matchers.openers.any_in(trimmed.as_bytes()) {
		return LineType::Code;
	}
	if clear
		&& let Some(pair) =
			lang.line_block_comments.iter().position(|&(open, _)| is_line_block_marker(line, trimmed, open, lang))
	{
		comment_state.carried = Carried::LineBlock(pair);
		return LineType::Comment;
	}
	classify_markers(trimmed, is_indented(line, trimmed), lang, matchers, comment_state)
}

/// Whether `line` holds `marker` of one of `lang`'s line block comments: at its very start, not followed by a word character (`=cut`, not `=cutoff`), or alone on it, spaces aside, where the language asks for that.
fn is_line_block_marker(line: &str, trimmed: &str, marker: &str, lang: &Language) -> bool {
	if lang.line_blocks_alone {
		trimmed == marker
	} else {
		line.strip_prefix(marker).is_some_and(|rest| !rest.as_bytes().first().is_some_and(|&b| is_word_char(b)))
	}
}

/// Whether `line` starts with whitespace that trimming took off, leaving `trimmed`.
fn is_indented(line: &str, trimmed: &str) -> bool {
	!std::ptr::eq(line.as_ptr(), trimmed.as_ptr())
}

/// Classify a trimmed, non-blank line by the comment markers and string literals in it.
fn classify_markers(
	trimmed: &str,
	indented: bool,
	lang: &Language,
	matchers: &LanguageMatchers,
	comment_state: &mut CommentState,
) -> LineType {
	if let Some(strings) = matchers.strings.as_ref()
		&& (comment_state.is_in_string() || strings.may_contain_string(trimmed.as_bytes()))
	{
		return classify_with_strings(trimmed, indented, lang, matchers, strings, comment_state);
	}
	let mut line_remainder = trimmed;
	#[expect(
		clippy::option_if_let_else,
		reason = "the two branches have different types of side effects (mutating line_remainder) that map_or_else cannot express cleanly"
	)]
	let mut has_code = if let Some(block_comments) = matchers.block_comments.as_ref() {
		let (remainder, found_code) = handle_block_comments(trimmed, lang, matchers, block_comments, comment_state);
		line_remainder = remainder;
		found_code
	} else {
		false
	};
	if comment_state.is_in_comment() {
		return if has_code { LineType::Code } else { LineType::Comment };
	}
	if let Some(pos) = find_line_comment(line_remainder, lang, matchers) {
		comment_state.end_line_comment(trimmed, lang);
		if pos > 0 && contains_non_whitespace(&line_remainder[..pos]) {
			has_code = true;
		}
		return if has_code { LineType::Code } else { LineType::Comment };
	}
	if contains_non_whitespace(line_remainder) {
		has_code = true;
	}
	if has_code { LineType::Code } else { LineType::Comment }
}

/// Trim ASCII whitespace, including a line's trailing `\n` or `\r\n`, from both ends. Bytes rather than `char`s, since only ASCII is trimmed; called for every line.
#[inline]
fn trim_ascii(line: &str) -> &str {
	let bytes = line.as_bytes();
	let mut start = 0;
	let mut end = bytes.len();
	// Indentation is mostly runs of spaces, so skip eight at a time.
	while let Some(chunk) = bytes.get(start..start + 8)
		&& chunk == b"        "
	{
		start += 8;
	}
	while start < end && is_ascii_ws(bytes[start]) {
		start += 1;
	}
	while end > start && is_ascii_ws(bytes[end - 1]) {
		end -= 1;
	}
	&line[start..end]
}

#[inline]
fn contains_non_whitespace(s: &str) -> bool {
	let bytes = s.as_bytes();
	let mut idx = 0;
	while idx < bytes.len() {
		let byte = bytes[idx];
		if !is_ascii_ws(byte) {
			return true;
		}
		if byte == b' ' || byte == b'\t' {
			// Skip runs of common whitespace quickly.
			if let Some(pos) = memchr2(b' ', b'\t', &bytes[idx..]) {
				idx += pos + 1;
			} else {
				idx = bytes.len();
			}
		} else {
			idx += 1;
		}
	}
	false
}

/// Whether a line is only whitespace, so [`classify_line`] would call it blank.
#[inline]
pub fn is_blank(line: &[u8]) -> bool {
	line.iter().all(|&b| is_ascii_ws(b))
}

#[inline]
const fn is_ascii_ws(b: u8) -> bool {
	matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}

#[inline]
const fn is_word_char(b: u8) -> bool {
	b.is_ascii_alphanumeric() || b == b'_'
}

#[inline]
const fn is_valid_line_comment_match(line: &str, end: usize, token: &str) -> bool {
	let Some(&first) = token.as_bytes().first() else {
		return false;
	};
	if is_word_char(first) {
		let bytes = line.as_bytes();
		if end < bytes.len() && is_word_char(bytes[end]) {
			return false;
		}
	}
	true
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;

	#[rstest]
	#[case::spaces("  hello  ", "hello")]
	#[case::tabs("\t\tworld\t\t", "world")]
	#[case::only_spaces("   ", "")]
	#[case::empty("", "")]
	#[case::trailing_lf("hello\n", "hello")]
	#[case::trailing_crlf("hello\r\n", "hello")]
	#[case::spaces_trailing_lf("  hello  \n", "hello")]
	#[case::spaces_trailing_crlf("  hello  \r\n", "hello")]
	fn test_trim_ascii(#[case] input: &str, #[case] expected: &str) {
		assert_eq!(trim_ascii(input), expected);
	}

	#[test]
	fn test_contains_non_whitespace() {
		assert!(contains_non_whitespace("hello"));
		assert!(contains_non_whitespace("  x  "));
		assert!(!contains_non_whitespace(""));
		assert!(!contains_non_whitespace("   "));
		assert!(!contains_non_whitespace("\t\t"));
		assert!(contains_non_whitespace("\u{000B}x"));
	}

	#[rstest]
	#[case::empty("")]
	#[case::spaces("   ")]
	#[case::tabs("\t\t")]
	fn test_classify_blank_lines(#[case] line: &str) {
		assert_eq!(classify_all("Rust", &[line]), [LineType::Blank]);
	}

	/// For every language, random lines built from its own markers: whenever the opener-byte fast path would call a line code, the full classification agrees and leaves the comment and string state alone.
	#[test]
	fn opener_fast_path_matches_full_classification() {
		use crate::langs::{LANGUAGES, matchers::language_matchers};
		let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
		let mut next = move |bound: usize| {
			seed ^= seed << 13;
			seed ^= seed >> 7;
			seed ^= seed << 17;
			usize::try_from(seed % bound as u64).unwrap()
		};
		let mut fast_lines = 0;
		for lang in LANGUAGES {
			let matchers = language_matchers(lang);
			let mut pieces: Vec<&str> = vec![
				"a", "x", "1", " ", "\t", "(", ")", ";", "=", "_", "*", "+", "-", "\\", "#", "/", "\"", "'", "`", "{",
				"}", "<", ">", "!", "%", "$", "@", ":", "[", "]", "é",
			];
			pieces.extend(lang.line_comments);
			pieces.extend(lang.block_comments.iter().flat_map(|&(start, end, _)| [start, end]));
			pieces.extend(lang.strings.iter().flat_map(|delimiter| [delimiter.open, delimiter.close]));
			let mut state = CommentState::new();
			for _ in 0..400 {
				let mut line = String::new();
				for _ in 0..=next(12) {
					line.push_str(pieces[next(pieces.len())]);
				}
				let trimmed = trim_ascii(&line);
				if trimmed.is_empty() {
					continue;
				}
				let fast =
					!state.is_in_comment() && !state.is_in_string() && !matchers.openers.any_in(trimmed.as_bytes());
				if fast {
					fast_lines += 1;
					let before = state.clone();
					assert_eq!(
						classify_markers(trimmed, is_indented(&line, trimmed), lang, matchers, &mut state),
						LineType::Code,
						"{}: {line:?}",
						lang.name
					);
					assert_eq!(state, before, "{}: {line:?} changed the state", lang.name);
				} else {
					classify_markers(trimmed, is_indented(&line, trimmed), lang, matchers, &mut state);
				}
			}
		}
		assert!(fast_lines > 10_000, "only {fast_lines} lines took the fast path");
	}

	#[test]
	fn test_comment_state_nesting() {
		let mut state = CommentState::new();
		assert!(!state.is_in_comment());
		state.enter_first_block(0);
		assert!(state.is_in_comment());
		state.enter_nested_block();
		assert!(state.is_in_comment());
		state.exit_block();
		assert!(state.is_in_comment());
		state.exit_block();
		assert!(!state.is_in_comment());
	}

	fn classify_all(lang_name: &str, lines: &[&str]) -> Vec<LineType> {
		let lang = crate::langs::LANGUAGES.iter().find(|l| l.name == lang_name).unwrap();
		let mut state = CommentState::new();
		lines.iter().enumerate().map(|(i, line)| classify_line(line, lang, &mut state, i == 0)).collect()
	}

	#[test]
	fn block_comment_only_closes_with_its_own_end() {
		// `*)` must not close a comment opened with `{`.
		let types = classify_all("Pascal", &["{ start", "still *) inside", "} x := 1;", "y := 2;"]);
		assert_eq!(types, [LineType::Comment, LineType::Comment, LineType::Code, LineType::Code]);
	}

	#[rstest]
	#[case::marker_in_string(&["let s = \"/* no\";", "x();"], &[LineType::Code, LineType::Code])]
	#[case::escaped_quote(&["let s = \"\\\"/*\";", "x();"], &[LineType::Code, LineType::Code])]
	#[case::escaped_escape(&["let s = \"\\\\\"; /* c */"], &[LineType::Code])]
	#[case::lifetime_then_comment(&["fn f<'a>(s: &'a str) -> char { '\"' }", "/* c */"], &[LineType::Code, LineType::Comment])]
	#[case::raw_string(&["let r = r#\"*/ \" /*\"#;", "x();"], &[LineType::Code, LineType::Code])]
	#[case::multiline_string(&["let s = \"a", "/* in string */\";", "/* c */"], &[LineType::Code, LineType::Code, LineType::Comment])]
	#[case::quote_in_comment(&["// don't", "x();"], &[LineType::Comment, LineType::Code])]
	fn rust_strings(#[case] lines: &[&str], #[case] expected: &[LineType]) {
		assert_eq!(classify_all("Rust", lines), expected);
	}

	#[rstest]
	#[case::doubled_quotes(&["var s = @\"value=\"\"\"\" />\";", "// c"], &[LineType::Code, LineType::Comment])]
	#[case::verbatim_backslash(&["var p = @\"C:\\temp\\\";", "// c"], &[LineType::Code, LineType::Comment])]
	fn csharp_verbatim_strings(#[case] lines: &[&str], #[case] expected: &[LineType]) {
		assert_eq!(classify_all("C#", lines), expected);
	}

	#[rstest]
	#[case::docstring(&["def f():", "    \"\"\"Docstring.", "", "    More.\"\"\"", "    return 1"], &[LineType::Code, LineType::Comment, LineType::Blank, LineType::Comment, LineType::Code])]
	#[case::one_line_docstring(&["    '''Short.'''", "x = 1"], &[LineType::Comment, LineType::Code])]
	#[case::string_argument(&["code = dedent('''", "    pass", "''')"], &[LineType::Code, LineType::Code, LineType::Code])]
	#[case::code_after_docstring(&["\"\"\"Doc.\"\"\"; x = 1"], &[LineType::Code])]
	fn python_docstrings(#[case] lines: &[&str], #[case] expected: &[LineType]) {
		assert_eq!(classify_all("Python", lines), expected);
	}

	#[rstest]
	#[case::closes_each_level(&["/* a /* b */ c */ x"], &[LineType::Code])]
	#[case::stays_open(&["/* a /* b */ c", "d */ e */ x"], &[LineType::Comment, LineType::Code])]
	#[case::reopens(&["/* /* */ */ /* /* */", "*/"], &[LineType::Comment, LineType::Comment])]
	#[case::shared_slash(&["/*/ /* */ */"], &[LineType::Comment])]
	fn nested_comments_on_one_line(#[case] lines: &[&str], #[case] expected: &[LineType]) {
		assert_eq!(classify_all("Rust", lines), expected);
	}

	#[test]
	fn nested_comments_on_one_line_take_linear_time() {
		let line = "/*x".repeat(1_000_000);
		let start = std::time::Instant::now();
		assert_eq!(classify_all("Rust", &[&line]), [LineType::Comment]);
		assert!(start.elapsed().as_secs() < 10, "took {:?}", start.elapsed());
	}

	#[test]
	fn empty_block_comment_closes() {
		let types = classify_all("Java", &["return /**/ null;", "x();"]);
		assert_eq!(types, [LineType::Code, LineType::Code]);
	}

	#[rstest]
	#[case::define("# define FOO 1", true)]
	#[case::indented_ifdef("#  ifdef CONFIG_X", true)]
	#[case::if_condition("# if KPMDS == 3", true)]
	#[case::if_defined("# if defined(CONFIG_PPC)", true)]
	#[case::bare_else("# else", true)]
	#[case::if_prose("# if the divisor is 0, take the slow path", false)]
	#[case::else_prose("# else we fall through", false)]
	#[case::comment("# save the frame pointer", false)]
	fn test_is_preprocessor_directive(#[case] line: &str, #[case] expected: bool) {
		assert_eq!(is_preprocessor_directive(line), expected);
	}

	#[rstest]
	#[case::toml_multiline("TOML", &["s = \"\"\"", "# not a comment", "\"\"\"", "# c"], &[LineType::Code, LineType::Code, LineType::Code, LineType::Comment])]
	#[case::toml_literal("TOML", &["p = '''", "# not a comment", "'''"], &[LineType::Code, LineType::Code, LineType::Code])]
	#[case::cmake_quoted("CMake", &["check_cxx_source_compiles(\"", "#include <x>", "int main() {}\" HAVE_X)", "# c"], &[LineType::Code, LineType::Code, LineType::Code, LineType::Comment])]
	#[case::cmake_bracket("CMake", &["set(s [=[", "# not a comment", "]=])", "#[[ c ]]"], &[LineType::Code, LineType::Code, LineType::Code, LineType::Comment])]
	#[case::cmake_bracket_comment_with_equals("CMake", &["#[==[ c", "]] still c", "]==]", "set(X 1)"], &[LineType::Comment, LineType::Comment, LineType::Comment, LineType::Code])]
	#[case::kotlin_raw_string_ending_in_a_quote("Kotlin", &["val s = \"\"\"a \"\"\"\" // c", "// c"], &[LineType::Code, LineType::Comment])]
	#[case::c_spliced_line_comment("C", &["int x; // c \\", "still c \\", "and this", "int y;"], &[LineType::Code, LineType::Comment, LineType::Comment, LineType::Code])]
	#[case::c_string_splice_ends_at_blank("C", &["char *s = \"abc \\", "", "// c\";", "int x;"], &[LineType::Code, LineType::Blank, LineType::Comment, LineType::Code])]
	#[case::csharp_hole_holding_a_quote("C#", &["var row = $@\"Name: {(quoted ? '\"' : ' ')}", "// still in the string", "\";", "// c"], &[LineType::Code, LineType::Code, LineType::Code, LineType::Comment])]
	#[case::csharp_doubled_brace_is_text("C#", &["var s = $\"{{ \\\" }}\"; // c"], &[LineType::Code])]
	#[case::csharp_raw_dollar_dollar("C#", &["var s = $$\"\"\"", "{x} // text", "\"\"\";", "// c"], &[LineType::Code, LineType::Code, LineType::Code, LineType::Comment])]
	#[case::js_nested_template("JavaScript", &["const s = `a ${f(`b ${c}`)} d", "// still in the template", "`;", "// c"], &[LineType::Code, LineType::Code, LineType::Code, LineType::Comment])]
	#[case::js_hole_over_lines("JavaScript", &["const s = `a ${", "  x // a comment in the hole", "} b`;", "// c"], &[LineType::Code, LineType::Code, LineType::Code, LineType::Comment])]
	#[case::js_braces_in_hole("JavaScript", &["const s = `${ {a: \"`\"}.a }`; // c", "// c"], &[LineType::Code, LineType::Comment])]
	#[case::ruby_hole_holding_a_quote("Ruby", &["s = \"#{h[\"k\"] + '\"'}\" # c", "# c"], &[LineType::Code, LineType::Comment])]
	#[case::swift_hole("Swift", &["let s = \"\\(a ? \"x\" : \"y\")\" // c", "// c"], &[LineType::Code, LineType::Comment])]
	#[case::kotlin_one_line_hole_ends_with_line("Kotlin", &["val s = \"${a", "// c"], &[LineType::Code, LineType::Comment])]
	#[case::cpp_raw_string("C++", &["auto s = R\"(", "/* not a comment", ")\";", "// c"], &[LineType::Code, LineType::Code, LineType::Code, LineType::Comment])]
	#[case::cpp_raw_string_with_delimiter("C++", &["auto s = R\"xy(a \")\" b", "/* not a comment", ")xy\"; /* c */"], &[LineType::Code, LineType::Code, LineType::Code])]
	#[case::cpp_raw_string_with_prefix("C++", &["auto s = u8R\"--(", "// not a comment", ")--\";", "// c"], &[LineType::Code, LineType::Code, LineType::Code, LineType::Comment])]
	#[case::cpp_macro_pasted_to_string("C++", &["f(DIR\"/*.txt\");", "// c"], &[LineType::Code, LineType::Comment])]
	#[case::cpp_r_quote_without_paren("C++", &["s = R\"/*\"; /*", "c */"], &[LineType::Code, LineType::Comment])]
	#[case::powershell_here_string("PowerShell", &["$s = @'", "don't", "# not a comment", "'@", "# c"], &[LineType::Code, LineType::Code, LineType::Code, LineType::Code, LineType::Comment])]
	#[case::pod_closes_only_unindented("Perl", &["=pod", "", "    =cut", "still pod", "=cut", "sub run { 1 }"], &[LineType::Comment, LineType::Blank, LineType::Comment, LineType::Comment, LineType::Comment, LineType::Code])]
	#[case::matlab_block_markers_alone("MATLAB", &["%{", "%} not the end", "z = x;", "%}", "%{ note", "z = x + 1;"], &[LineType::Comment, LineType::Comment, LineType::Comment, LineType::Comment, LineType::Comment, LineType::Code])]
	#[case::c_splice_ends_at_blank("C", &["// c \\", "", "int y;"], &[LineType::Comment, LineType::Blank, LineType::Code])]
	fn config_strings(#[case] language: &str, #[case] lines: &[&str], #[case] expected: &[LineType]) {
		assert_eq!(classify_all(language, lines), expected);
	}

	#[rstest]
	#[case::shell("Bash", &["cat <<EOF > x.c", "#include <stdio.h>", "", "EOF", "# c"], &[LineType::Code, LineType::Code, LineType::Blank, LineType::Code, LineType::Comment])]
	#[case::shell_indented_quoted("Bash", &["\tcat <<- 'END' # c", "\t# not a comment", "\tEND", "# c"], &[LineType::Code, LineType::Code, LineType::Code, LineType::Comment])]
	#[case::shell_spaced_lowercase_quoted("Bash", &["cat << 'eof'", "# x", "eof"], &[LineType::Code, LineType::Code, LineType::Code])]
	#[case::shell_here_string("Bash", &["grep x <<< \"$s\"", "# c"], &[LineType::Code, LineType::Comment])]
	#[case::shell_shift("Bash", &["echo $(( 1 << n ))", "# c"], &[LineType::Code, LineType::Comment])]
	#[case::perl("Perl", &["$code .= <<___;", "# asm comment", "___", "# c"], &[LineType::Code, LineType::Code, LineType::Code, LineType::Comment])]
	#[case::perl_spaced_bare_word("Perl", &["$x = $y << SHIFT;", "# c"], &[LineType::Code, LineType::Comment])]
	#[case::ruby_squiggly("Ruby", &["sql = <<~SQL.strip", "  # not a comment", "  SQL", "# c"], &[LineType::Code, LineType::Code, LineType::Code, LineType::Comment])]
	#[case::ruby_append("Ruby", &["out << \"EOF\"", "# c"], &[LineType::Code, LineType::Comment])]
	#[case::ruby_singleton_class("Ruby", &["class << self", "# c"], &[LineType::Code, LineType::Comment])]
	#[case::crystal_append_constant("Crystal", &["types << FFI::Type.pointer", "# c"], &[LineType::Code, LineType::Comment])]
	#[case::php("PHP", &["$s = <<<EOT", "# not a comment", "EOT;", "# c"], &[LineType::Code, LineType::Code, LineType::Code, LineType::Comment])]
	#[case::in_comment("Bash", &["# cat <<EOF", "# c"], &[LineType::Comment, LineType::Comment])]
	#[case::ruby_singleton_class_unspaced("Ruby", &["class <<self", "# c"], &[LineType::Code, LineType::Comment])]
	// A plain `<<` heredoc only closes at its word at the start of a line; `<<~` and `<<-` also behind spaces.
	#[case::ruby_plain_closes_unindented("Ruby", &["x = <<RUBY", "  RUBY", "# not a comment", "RUBY", "# c"], &[LineType::Code, LineType::Code, LineType::Code, LineType::Code, LineType::Comment])]
	#[case::ruby_squiggly_closes_indented("Ruby", &["x = <<~TEXT", "  a", "  TEXT", "# c"], &[LineType::Code, LineType::Code, LineType::Code, LineType::Comment])]
	#[case::shell_escaped_apostrophe("Bash", &["echo don\\'t", "# c"], &[LineType::Code, LineType::Comment])]
	#[case::powershell_escaped_apostrophe("PowerShell", &["Write-Host don`'t <# c #>", "# c"], &[LineType::Code, LineType::Comment])]
	fn heredocs(#[case] language: &str, #[case] lines: &[&str], #[case] expected: &[LineType]) {
		assert_eq!(classify_all(language, lines), expected);
	}

	#[rstest]
	#[case::backtick_in_regex(&["s = s.replace(/[`'\"]/g, \"\");", "// c"], &[LineType::Code, LineType::Comment])]
	#[case::block_start_in_regex(&["return /[/*]/g;", "x();"], &[LineType::Code, LineType::Code])]
	#[case::division_then_comment(&["const r = a / b; // c", "// c"], &[LineType::Code, LineType::Comment])]
	fn javascript_regex_literals(#[case] lines: &[&str], #[case] expected: &[LineType]) {
		assert_eq!(classify_all("JavaScript", lines), expected);
	}

	#[test]
	fn escaped_quote_outside_string_does_not_open_one() {
		let types = classify_all("Elixir", &["assert_raise E, ~r[key with \\\"=\\\"], fn ->", "# c"]);
		assert_eq!(types, [LineType::Code, LineType::Comment]);
	}

	#[test]
	fn unterminated_single_line_string_ends_with_line() {
		let types = classify_all("C", &["char *s = \"abc;", "/* c */"]);
		assert_eq!(types, [LineType::Code, LineType::Comment]);
	}

	#[test]
	fn block_start_inside_line_comment_is_ignored() {
		let types = classify_all("Go", &["// matches \"/healthz/*\" paths", "x := 1", "y := 2"]);
		assert_eq!(types, [LineType::Comment, LineType::Code, LineType::Code]);
	}

	#[rstest]
	#[case::nesting_pair(&["/+ outer", "/+ inner +/", "still comment", "+/ x = 1;"], &[LineType::Comment, LineType::Comment, LineType::Comment, LineType::Code])]
	#[case::non_nesting_pair(&["/* outer", "/* not nested */", "x = 1;"], &[LineType::Comment, LineType::Comment, LineType::Code])]
	fn d_nests_only_plus_comments(#[case] lines: &[&str], #[case] expected: &[LineType]) {
		assert_eq!(classify_all("D", lines), expected);
	}

	#[test]
	fn test_is_valid_line_comment_match() {
		// Word-char tokens need word boundary after
		assert!(is_valid_line_comment_match("REM hello", 3, "REM"));
		assert!(!is_valid_line_comment_match("REMEMBER", 3, "REM")); // no boundary
		// Symbol tokens don't need boundary
		assert!(is_valid_line_comment_match("// test", 2, "//"));
		assert!(is_valid_line_comment_match("//test", 2, "//"));
	}

	#[rstest]
	#[case(b' ', true)]
	#[case(b'\t', true)]
	#[case(b'\n', true)]
	#[case(b'\r', true)]
	#[case(0x0B, true)]
	#[case(0x0C, true)]
	#[case(b'a', false)]
	#[case(b'0', false)]
	fn test_is_ascii_ws(#[case] byte: u8, #[case] expected: bool) {
		assert_eq!(is_ascii_ws(byte), expected);
	}

	#[rstest]
	#[case(b'a', true)]
	#[case(b'Z', true)]
	#[case(b'5', true)]
	#[case(b'_', true)]
	#[case(b' ', false)]
	#[case(b'/', false)]
	#[case(b'-', false)]
	fn test_is_word_char(#[case] byte: u8, #[case] expected: bool) {
		assert_eq!(is_word_char(byte), expected);
	}

	#[rstest]
	#[case(LineType::Code, "code", "code", "Code")]
	#[case(LineType::Comment, "comment", "comments", "Comments")]
	#[case(LineType::Blank, "blank", "blanks", "Blanks")]
	#[case(LineType::Shebang, "shebang", "shebangs", "Shebangs")]
	fn test_line_type_labels(
		#[case] kind: LineType,
		#[case] singular: &str,
		#[case] plural: &str,
		#[case] title: &str,
	) {
		assert_eq!(kind.singular_label(), singular);
		assert_eq!(kind.plural_label(), plural);
		assert_eq!(kind.title_label(), title);
	}
}
