use std::cmp::Ordering;

use num_format::{CustomFormat, Grouping, ToFormattedString};

use super::options::{IndentStyle, NumberStyle, SizeStyle, SortDirection, ViewOptions};

#[derive(Debug, Clone)]
pub struct FormatterContext {
	pub options: ViewOptions,
	number: NumberFormatter,
	size: SizeFormatter,
	percent: PercentFormatter,
}

impl FormatterContext {
	#[must_use]
	pub fn new(options: ViewOptions) -> Self {
		let number = NumberFormatter::new(options.number_style);
		let size = SizeFormatter::new(options.size_style, number.clone());
		let percent = PercentFormatter::new(options.percent_precision);
		Self { options, number, size, percent }
	}

	#[must_use]
	pub fn number(&self, value: u64) -> String {
		self.number.format(value)
	}

	#[must_use]
	pub fn size(&self, bytes: u64) -> String {
		self.size.format(bytes)
	}

	#[must_use]
	pub fn percent(&self, value: f64) -> String {
		self.percent.format(value)
	}

	#[must_use]
	pub fn indent(&self, level: usize) -> String {
		match self.options.indent_style {
			IndentStyle::Tab => "\t".repeat(level),
			IndentStyle::Spaces(n) => " ".repeat(usize::from(n) * level),
		}
	}
}

#[derive(Debug, Clone)]
pub enum NumberFormatter {
	Plain,
	// Box to keep enum variant sizes balanced (clippy::large_enum_variant).
	Formatted(Box<CustomFormat>),
}

impl NumberFormatter {
	#[must_use]
	pub fn new(style: NumberStyle) -> Self {
		match style {
			NumberStyle::Plain => Self::Plain,
			NumberStyle::Comma => {
				let format = CustomFormat::builder().grouping(Grouping::Standard).separator(",").build().unwrap();
				Self::Formatted(Box::new(format))
			}
			NumberStyle::Underscore => {
				let format = CustomFormat::builder().grouping(Grouping::Standard).separator("_").build().unwrap();
				Self::Formatted(Box::new(format))
			}
			NumberStyle::Space => {
				let format = CustomFormat::builder().grouping(Grouping::Standard).separator(" ").build().unwrap();
				Self::Formatted(Box::new(format))
			}
		}
	}

	#[must_use]
	pub fn format(&self, value: u64) -> String {
		match self {
			Self::Plain => value.to_string(),
			Self::Formatted(format) => value.to_formatted_string(format.as_ref()),
		}
	}
}

#[derive(Debug, Clone)]
pub struct SizeFormatter {
	style: SizeStyle,
	number: NumberFormatter,
}

impl SizeFormatter {
	#[must_use]
	pub const fn new(style: SizeStyle, number: NumberFormatter) -> Self {
		Self { style, number }
	}

	#[must_use]
	#[expect(clippy::cast_precision_loss)]
	pub fn format(&self, size: u64) -> String {
		let base: f64 = match self.style {
			SizeStyle::Binary => 1024.0,
			SizeStyle::Decimal => 1000.0,
		};
		let units: &[&str] = match self.style {
			SizeStyle::Binary => &["B", "KiB", "MiB", "GiB", "TiB", "PiB"],
			SizeStyle::Decimal => &["B", "KB", "MB", "GB", "TB", "PB"],
		};
		let mut value = size as f64;
		let mut unit_index = 0usize;
		while value >= base && unit_index < units.len() - 1 {
			value /= base;
			unit_index += 1;
		}
		match unit_index {
			0 => format!("{} B", self.number.format(size)),
			_ if value < 10.0 => format!("{value:.2} {}", units[unit_index]),
			_ if value < 100.0 => format!("{value:.1} {}", units[unit_index]),
			_ => format!("{value:.0} {}", units[unit_index]),
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub struct PercentFormatter {
	precision: u8,
}

impl PercentFormatter {
	#[must_use]
	pub const fn new(precision: u8) -> Self {
		Self { precision }
	}

	#[must_use]
	pub fn format(self, value: f64) -> String {
		let precision = usize::from(self.precision);
		format!("{value:.precision$}")
	}
}

/// Return `singular` when `count` equals 1, otherwise return `plural`.
#[inline]
pub const fn pluralize<'a>(count: u64, singular: &'a str, plural: &'a str) -> &'a str {
	if count == 1 { singular } else { plural }
}

pub fn apply_sort<T>(
	items: &mut [T],
	direction: SortDirection,
	mut metric: impl FnMut(&T) -> SortValue<'_>,
	mut tiebreaker: impl FnMut(&T, &T) -> Ordering,
) {
	items.sort_by(|a, b| {
		let lhs = metric(a);
		let rhs = metric(b);
		let mut ordering = match (lhs, rhs) {
			(SortValue::Num(l), SortValue::Num(r)) => l.cmp(&r),
			(SortValue::Text(l), SortValue::Text(r)) => l.cmp(r),
			(SortValue::Num(_), SortValue::Text(_)) | (SortValue::Text(_), SortValue::Num(_)) => Ordering::Equal,
		};
		if ordering == Ordering::Equal {
			ordering = tiebreaker(a, b);
		}
		if direction == SortDirection::Desc { ordering.reverse() } else { ordering }
	});
}

#[derive(Debug)]
pub enum SortValue<'a> {
	Num(u64),
	Text(&'a str),
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;

	#[rstest]
	#[case(NumberStyle::Plain, 0, "0")]
	#[case(NumberStyle::Plain, 1_234_567, "1234567")]
	#[case(NumberStyle::Comma, 0, "0")]
	#[case(NumberStyle::Comma, 999, "999")]
	#[case(NumberStyle::Comma, 1000, "1,000")]
	#[case(NumberStyle::Comma, 1_234_567, "1,234,567")]
	#[case(NumberStyle::Underscore, 1_234_567, "1_234_567")]
	#[case(NumberStyle::Space, 1_234_567, "1 234 567")]
	fn test_number_formatter(#[case] style: NumberStyle, #[case] value: u64, #[case] expected: &str) {
		let fmt = NumberFormatter::new(style);
		assert_eq!(fmt.format(value), expected);
	}

	#[rstest]
	#[case(SizeStyle::Binary, 0, "0 B")]
	#[case(SizeStyle::Binary, 512, "512 B")]
	#[case(SizeStyle::Binary, 1023, "1023 B")]
	#[case(SizeStyle::Binary, 1024, "1.00 KiB")]
	#[case(SizeStyle::Binary, 1536, "1.50 KiB")]
	#[case(SizeStyle::Binary, 10240, "10.0 KiB")]
	#[case(SizeStyle::Binary, 102_400, "100 KiB")]
	#[case(SizeStyle::Binary, 1024 * 1024, "1.00 MiB")]
	#[case(SizeStyle::Binary, 5 * 1024 * 1024, "5.00 MiB")]
	#[case(SizeStyle::Decimal, 1000, "1.00 KB")]
	#[case(SizeStyle::Decimal, 1500, "1.50 KB")]
	#[case(SizeStyle::Decimal, 1_000_000, "1.00 MB")]
	fn test_size_formatter(#[case] style: SizeStyle, #[case] bytes: u64, #[case] expected: &str) {
		let num = NumberFormatter::new(NumberStyle::Plain);
		let fmt = SizeFormatter::new(style, num);
		assert_eq!(fmt.format(bytes), expected);
	}

	#[rstest]
	#[case(2, 50.0, "50.00")]
	#[case(2, 33.333, "33.33")]
	#[case(0, 99.9, "100")]
	fn test_percent_formatter(#[case] precision: u8, #[case] value: f64, #[case] expected: &str) {
		let fmt = PercentFormatter::new(precision);
		assert_eq!(fmt.format(value), expected);
	}

	#[rstest]
	#[case(1, "file", "files", "file")]
	#[case(0, "file", "files", "files")]
	#[case(2, "file", "files", "files")]
	#[case(1, "child", "children", "child")]
	#[case(5, "child", "children", "children")]
	fn test_pluralize(#[case] count: u64, #[case] singular: &str, #[case] plural: &str, #[case] expected: &str) {
		assert_eq!(pluralize(count, singular, plural), expected);
	}

	#[test]
	fn test_apply_sort_ascending() {
		let mut items = vec![3u64, 1, 4, 1, 5];
		apply_sort(&mut items, SortDirection::Asc, |x| SortValue::Num(*x), |_, _| Ordering::Equal);
		assert_eq!(items, vec![1, 1, 3, 4, 5]);
	}

	#[test]
	fn test_apply_sort_descending() {
		let mut items = vec![3u64, 1, 4, 1, 5];
		apply_sort(&mut items, SortDirection::Desc, |x| SortValue::Num(*x), |_, _| Ordering::Equal);
		assert_eq!(items, vec![5, 4, 3, 1, 1]);
	}

	#[test]
	fn test_apply_sort_with_tiebreaker() {
		let mut items = vec![("b", 1u64), ("a", 1), ("c", 2)];
		apply_sort(&mut items, SortDirection::Asc, |(_, n)| SortValue::Num(*n), |(a, _), (b, _)| a.cmp(b));
		assert_eq!(items, vec![("a", 1), ("b", 1), ("c", 2)]);
	}

	#[test]
	fn test_apply_sort_text() {
		let mut items = vec!["banana", "apple", "cherry"];
		apply_sort(&mut items, SortDirection::Asc, |s| SortValue::Text(s), |_, _| Ordering::Equal);
		assert_eq!(items, vec!["apple", "banana", "cherry"]);
	}

	#[rstest]
	#[case(IndentStyle::Tab, 0, "")]
	#[case(IndentStyle::Tab, 1, "\t")]
	#[case(IndentStyle::Tab, 2, "\t\t")]
	#[case(IndentStyle::Spaces(2), 0, "")]
	#[case(IndentStyle::Spaces(2), 1, "  ")]
	#[case(IndentStyle::Spaces(2), 2, "    ")]
	#[case(IndentStyle::Spaces(4), 1, "    ")]
	#[case(IndentStyle::Spaces(4), 2, "        ")]
	fn test_indent(#[case] style: IndentStyle, #[case] level: usize, #[case] expected: &str) {
		let mut options = ViewOptions::default();
		options.indent_style = style;
		let ctx = FormatterContext::new(options);
		assert_eq!(ctx.indent(level), expected);
	}

	#[test]
	fn test_formatter_context() {
		let options = ViewOptions::default();
		let ctx = FormatterContext::new(options);
		// Basic sanity check that methods work
		assert!(!ctx.number(1000).is_empty());
		assert!(!ctx.size(1024).is_empty());
		assert!(!ctx.percent(50.0).is_empty());
	}
}
