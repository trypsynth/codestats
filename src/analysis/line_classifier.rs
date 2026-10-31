mod strings;

use memchr::memchr2;

use self::strings::classify_with_strings;
use crate::langs::{
	Language, is_shebang_for,
	matchers::{BlockCommentMatchers, LanguageMatchers, language_matchers},
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

/// Tracks block comment state across lines: how deeply nested we are, and which delimiter pair opened the comment.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CommentState {
	block_comment_depth: usize,
	pair: usize,
	/// The string literal left open by the previous line, as an index into the language's `strings`.
	string: Option<usize>,
	/// Whether the open string is a docstring, whose contents count as comment.
	doc_string: bool,
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

	#[must_use]
	#[inline]
	const fn is_in_string(&self) -> bool {
		self.string.is_some()
	}

	/// Whether the next line starts outside any comment or string.
	#[must_use]
	#[inline]
	pub(crate) const fn is_clear(&self) -> bool {
		!self.is_in_comment() && !self.is_in_string()
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
			block_matchers.find_block_end_or_nested_start(line_remainder, comment_state.pair)
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
	line_comments
		.find_iter(line)
		.find(|matched| {
			is_valid_line_comment_match(line, matched.end(), lang.line_comments[matched.pattern().as_usize()])
		})
		.map(|matched| matched.start())
}

/// Classify a line as code, comment, blank, or shebang.
#[inline]
pub fn classify_line(line: &str, lang: &Language, comment_state: &mut CommentState, is_first_line: bool) -> LineType {
	let trimmed = trim_ascii(line);
	if trimmed.is_empty() {
		return LineType::Blank;
	}
	if is_first_line && is_shebang_for(trimmed, lang) {
		return LineType::Shebang;
	}
	// Markers like fixed-form Fortran's `C` only count in the first column, so check the untrimmed line.
	if !comment_state.is_in_comment()
		&& !comment_state.is_in_string()
		&& lang.column_one_comments.iter().any(|marker| line.starts_with(marker))
	{
		return LineType::Comment;
	}
	if lang.c_preprocessor
		&& !comment_state.is_in_comment()
		&& !comment_state.is_in_string()
		&& is_preprocessor_directive(trimmed)
	{
		return LineType::Code;
	}
	let matchers = language_matchers(lang);
	// Outside a comment or string, a line with no byte that could start one is code: every search below would come up empty.
	if !comment_state.is_in_comment() && !comment_state.is_in_string() && !matchers.openers.any_in(trimmed.as_bytes()) {
		return LineType::Code;
	}
	classify_markers(trimmed, lang, matchers, comment_state)
}

/// Classify a trimmed, non-blank line by the comment markers and string literals in it.
fn classify_markers(
	trimmed: &str,
	lang: &Language,
	matchers: &LanguageMatchers,
	comment_state: &mut CommentState,
) -> LineType {
	if let Some(strings) = matchers.strings.as_ref()
		&& (comment_state.is_in_string() || strings.may_contain_string(trimmed.as_bytes()))
	{
		return classify_with_strings(trimmed, lang, matchers, strings, comment_state);
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
						classify_markers(trimmed, lang, matchers, &mut state),
						LineType::Code,
						"{}: {line:?}",
						lang.name
					);
					assert_eq!(state, before, "{}: {line:?} changed the state", lang.name);
				} else {
					classify_markers(trimmed, lang, matchers, &mut state);
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
