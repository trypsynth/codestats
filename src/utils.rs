use std::borrow::Cow;

/// Return `singular` when `count` equals 1, otherwise return `plural`.
#[inline]
pub const fn pluralize<'a>(count: u64, singular: &'a str, plural: &'a str) -> &'a str {
	if count == 1 { singular } else { plural }
}

/// Calculate the percentage that `part` represents of `total`.
///
/// Returns `0.0` when `total` is `0` to avoid division-by-zero panics.
#[inline]
#[expect(clippy::cast_precision_loss)]
pub fn percentage(part: u64, total: u64) -> f64 {
	if total == 0 { 0.0 } else { (part as f64 / total as f64) * 100.0 }
}

/// Escape a CSV field, wrapping in quotes if necessary.
///
/// Returns a borrowed reference when no escaping is needed (zero-copy), or an owned String when quotes or escaping are required.
pub fn escape_csv_field(field: &str) -> Cow<'_, str> {
	let needs_quotes = field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r');
	if !needs_quotes {
		return Cow::Borrowed(field);
	}
	let escaped = field.replace('"', "\"\"");
	Cow::Owned(format!("\"{escaped}\""))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_pluralize() {
		assert_eq!(pluralize(1, "file", "files"), "file");
		assert_eq!(pluralize(0, "file", "files"), "files");
		assert_eq!(pluralize(2, "file", "files"), "files");
		assert_eq!(pluralize(1, "child", "children"), "child");
		assert_eq!(pluralize(5, "child", "children"), "children");
	}

	#[test]
	fn test_percentage() {
		assert_eq!(percentage(0, 100), 0.0);
		assert_eq!(percentage(50, 100), 50.0);
		assert_eq!(percentage(25, 100), 25.0);
		assert_eq!(percentage(100, 100), 100.0);
		assert_eq!(percentage(10, 0), 0.0);
		let part = u64::MAX / 2;
		let total = u64::MAX;
		let pct = percentage(part, total);
		assert!((pct - 50.0).abs() < 0.000000_1);
	}

	#[test]
	fn test_escape_csv_field_no_escaping() {
		let field = "simple";
		let result = escape_csv_field(field);
		assert!(matches!(result, Cow::Borrowed(_)));
		assert_eq!(result, "simple");
	}

	#[test]
	fn test_escape_csv_field_with_comma() {
		assert_eq!(escape_csv_field("hello,world"), "\"hello,world\"");
	}

	#[test]
	fn test_escape_csv_field_with_quotes() {
		assert_eq!(escape_csv_field("say \"hello\""), "\"say \"\"hello\"\"\"");
	}

	#[test]
	fn test_escape_csv_field_with_newline() {
		assert_eq!(escape_csv_field("line1\nline2"), "\"line1\nline2\"");
	}

	#[test]
	fn test_escape_csv_field_with_carriage_return() {
		assert_eq!(escape_csv_field("line1\rline2"), "\"line1\rline2\"");
	}
}
