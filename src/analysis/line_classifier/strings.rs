//! Lexing string, char and regex literals, so comment markers inside them aren't taken for comments.

use super::{CommentState, LineType, contains_non_whitespace, is_valid_line_comment_match};
use crate::langs::{
	Language,
	matchers::{LanguageMatchers, StringMatchers, Token},
};

/// Classify a line in a language with string literals, scanning left to right so comment markers inside strings are ignored. String contents count as code.
pub(super) fn classify_with_strings(
	line: &str,
	lang: &Language,
	matchers: &LanguageMatchers,
	strings: &StringMatchers,
	state: &mut CommentState,
) -> LineType {
	let bytes = line.as_bytes();
	let mut pos = 0;
	let mut has_code = false;
	while pos < line.len() {
		if state.is_in_comment() {
			// Strings aren't lexed inside comments, so only the open comment's own end (or nested start) matters.
			let Some((found, len, nested)) = matchers
				.block_comments
				.as_ref()
				.and_then(|block| block.find_block_end_or_nested_start(&line[pos..], state.pair))
			else {
				break;
			};
			if nested {
				state.enter_nested_block();
			} else {
				state.exit_block();
			}
			pos += found + len;
		} else if let Some(index) = state.string {
			has_code |= !state.doc_string;
			let delimiter = &lang.strings[index];
			if let Some(end) = strings.find_close(bytes, pos, index, delimiter.escape) {
				state.string = None;
				pos = end;
			} else {
				// An unclosed single-line string ends with the line, unless the line ends in an escape (a continuation).
				let continues = delimiter.escape.is_some_and(|escape| bytes.last() == Some(&escape));
				if !delimiter.multiline && !continues {
					state.string = None;
				}
				break;
			}
		} else {
			let Some((start, end, token)) = strings.find_token(line, pos) else {
				has_code |= contains_non_whitespace(&line[pos..]);
				break;
			};
			has_code |= contains_non_whitespace(&line[pos..start]);
			pos = end;
			match token {
				Token::LineComment(i) => {
					if is_valid_line_comment_match(line, end, lang.line_comments[i]) {
						return if has_code { LineType::Code } else { LineType::Comment };
					}
					has_code = true;
				}
				Token::BlockComment(pair) => state.enter_first_block(pair),
				Token::Regex => {
					has_code = true;
					// Skip a whole regex literal so quotes, backticks and `/*` inside it don't open anything; otherwise it's division.
					if regex_can_start(bytes, start)
						&& let Some(regex_end) = find_regex_end(bytes, end)
					{
						pos = regex_end;
					}
				}
				Token::String(i) => {
					let delimiter = &lang.strings[i];
					// An escaped quote outside a string (in a regex or sigil like `~r[\"]`) doesn't open one.
					let escaped = start > 0 && delimiter.escape == Some(bytes[start - 1]);
					// A quote that doesn't look like a char literal (a Rust lifetime like `'a`) is just code.
					let char_mismatch = delimiter.char_literal && !looks_like_char(bytes, end);
					// A docstring is a string that starts its line; after code (`x = """`) it's an ordinary string.
					let doc_string = delimiter.doc && !has_code;
					has_code |= !doc_string;
					if !escaped && !char_mismatch {
						state.string = Some(i);
						state.doc_string = doc_string;
					}
				}
			}
		}
	}
	if has_code { LineType::Code } else { LineType::Comment }
}

/// Keywords after which a `/` starts a regex literal rather than dividing.
const REGEX_PRECEDING_KEYWORDS: &[&[u8]] = &[
	b"return",
	b"typeof",
	b"instanceof",
	b"in",
	b"of",
	b"new",
	b"delete",
	b"void",
	b"throw",
	b"case",
	b"do",
	b"else",
	b"yield",
	b"await",
];

/// Whether a `/` at `slash` can start a regex literal, judged by what precedes it on the line: the start of the line, an operator or opening punctuation, or a keyword like `return`. After an identifier, number or closing bracket it's division.
// Only JavaScript-like languages reach this; keeping it out of line keeps the hot scanning loop small.
#[inline(never)]
fn regex_can_start(line: &[u8], slash: usize) -> bool {
	let before = &line[..slash];
	let Some(last) = before.iter().rposition(|b| !b.is_ascii_whitespace()) else {
		return true;
	};
	match before[last] {
		// `a++ / b` and `a-- / b` divide.
		b'+' | b'-' => last == 0 || before[last - 1] != before[last],
		b'(' | b',' | b'=' | b':' | b'[' | b'!' | b'&' | b'|' | b'?' | b'{' | b'}' | b';' | b'*' | b'%' | b'<'
		| b'>' | b'~' | b'^' => true,
		b if b.is_ascii_alphanumeric() || b == b'_' || b == b'$' => {
			let word_start = before[..=last]
				.iter()
				.rposition(|b| !(b.is_ascii_alphanumeric() || *b == b'_' || *b == b'$'))
				.map_or(0, |i| i + 1);
			REGEX_PRECEDING_KEYWORDS.contains(&&before[word_start..=last])
		}
		_ => false,
	}
}

/// Find the end of a regex literal whose body starts at `start`, returning the position after its flags. Slashes inside `[...]` classes or escaped with `\` don't end it, and a regex can't span lines, so `None` means this `/` was division.
// Only JavaScript-like languages reach this; keeping it out of line keeps the hot scanning loop small.
#[inline(never)]
fn find_regex_end(line: &[u8], start: usize) -> Option<usize> {
	let mut pos = start;
	let mut in_class = false;
	while pos < line.len() {
		match line[pos] {
			b'\\' => pos += 1,
			b'[' => in_class = true,
			b']' => in_class = false,
			b'/' if !in_class => {
				let flags = line[pos + 1..].iter().take_while(|b| b.is_ascii_alphabetic()).count();
				return Some(pos + 1 + flags);
			}
			_ => {}
		}
		pos += 1;
	}
	None
}

/// Whether the text after an opening `'` looks like a char literal: one character or an escape, then a closing `'`.
fn looks_like_char(line: &[u8], start: usize) -> bool {
	match line.get(start) {
		None | Some(b'\'') => false,
		// Escapes like `'\n'` or `'\u{1F600}'`: the escaped character, then a closing quote within a few bytes.
		Some(b'\\') => line.get(start + 2..).is_some_and(|rest| rest.iter().take(10).any(|&b| b == b'\'')),
		Some(&lead) => {
			let len = match lead {
				0x00..=0x7F => 1,
				0xC0..=0xDF => 2,
				0xE0..=0xEF => 3,
				_ => 4,
			};
			line.get(start + len) == Some(&b'\'')
		}
	}
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;

	#[rstest]
	#[case::line_start(b"/re/", 0, true)]
	#[case::after_paren(b"f(/re/)", 2, true)]
	#[case::after_assign(b"x = /re/", 4, true)]
	#[case::after_return(b"return /re/", 7, true)]
	#[case::after_identifier(b"total / count", 6, false)]
	#[case::after_number(b"a / 2 / b", 6, false)]
	#[case::after_close_paren(b"(a) / b", 4, false)]
	#[case::after_increment(b"i++ / 2", 4, false)]
	#[case::identifier_ending_in_keyword(b"modulo / 2", 7, false)]
	fn test_regex_can_start(#[case] line: &[u8], #[case] slash: usize, #[case] expected: bool) {
		assert_eq!(regex_can_start(line, slash), expected);
	}

	#[rstest]
	#[case::simple(b"/abc/g;", Some(6))]
	#[case::escaped_slash(b"/a\\/b/;", Some(6))]
	#[case::slash_in_class(b"/[/*]/g", Some(7))]
	#[case::unterminated(b"/ 2", None)]
	fn test_find_regex_end(#[case] line: &[u8], #[case] expected: Option<usize>) {
		assert_eq!(find_regex_end(line, 1), expected);
	}

	#[rstest]
	#[case::plain(b"a'", 0, true)]
	#[case::escape(b"\\n'", 0, true)]
	#[case::unicode_escape(b"\\u{1F600}'", 0, true)]
	#[case::multibyte("é'".as_bytes(), 0, true)]
	#[case::lifetime(b"a>", 0, false)]
	#[case::empty(b"'", 0, false)]
	fn test_looks_like_char(#[case] line: &[u8], #[case] start: usize, #[case] expected: bool) {
		assert_eq!(looks_like_char(line, start), expected);
	}
}
