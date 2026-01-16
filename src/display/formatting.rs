use std::cmp::Ordering;

use num_format::{CustomFormat, Grouping, ToFormattedString};

use super::options::{NumberStyle, SizeStyle, SortDirection, ViewOptions};

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
	fn test_number_formatter_plain() {
		let fmt = NumberFormatter::new(NumberStyle::Plain);
		assert_eq!(fmt.format(0), "0");
		assert_eq!(fmt.format(1234567), "1234567");
	}

	#[test]
	fn test_number_formatter_comma() {
		let fmt = NumberFormatter::new(NumberStyle::Comma);
		assert_eq!(fmt.format(0), "0");
		assert_eq!(fmt.format(999), "999");
		assert_eq!(fmt.format(1000), "1,000");
		assert_eq!(fmt.format(1234567), "1,234,567");
	}

	#[test]
	fn test_number_formatter_underscore() {
		let fmt = NumberFormatter::new(NumberStyle::Underscore);
		assert_eq!(fmt.format(1234567), "1_234_567");
	}

	#[test]
	fn test_number_formatter_space() {
		let fmt = NumberFormatter::new(NumberStyle::Space);
		assert_eq!(fmt.format(1234567), "1 234 567");
	}

	#[test]
	fn test_size_formatter_binary_bytes() {
		let num = NumberFormatter::new(NumberStyle::Plain);
		let fmt = SizeFormatter::new(SizeStyle::Binary, num);
		assert_eq!(fmt.format(0), "0 B");
		assert_eq!(fmt.format(512), "512 B");
		assert_eq!(fmt.format(1023), "1023 B");
	}

	#[test]
	fn test_size_formatter_binary_kib() {
		let num = NumberFormatter::new(NumberStyle::Plain);
		let fmt = SizeFormatter::new(SizeStyle::Binary, num);
		assert_eq!(fmt.format(1024), "1.00 KiB");
		assert_eq!(fmt.format(1536), "1.50 KiB");
		assert_eq!(fmt.format(10240), "10.0 KiB");
		assert_eq!(fmt.format(102400), "100 KiB");
	}

	#[test]
	fn test_size_formatter_binary_mib() {
		let num = NumberFormatter::new(NumberStyle::Plain);
		let fmt = SizeFormatter::new(SizeStyle::Binary, num);
		assert_eq!(fmt.format(1024 * 1024), "1.00 MiB");
		assert_eq!(fmt.format(5 * 1024 * 1024), "5.00 MiB");
	}

	#[test]
	fn test_size_formatter_decimal() {
		let num = NumberFormatter::new(NumberStyle::Plain);
		let fmt = SizeFormatter::new(SizeStyle::Decimal, num);
		assert_eq!(fmt.format(1000), "1.00 KB");
		assert_eq!(fmt.format(1500), "1.50 KB");
		assert_eq!(fmt.format(1_000_000), "1.00 MB");
	}

	#[test]
	fn test_percent_formatter() {
		let fmt = PercentFormatter::new(2);
		assert_eq!(fmt.format(50.0), "50.00");
		assert_eq!(fmt.format(33.333), "33.33");
		let fmt0 = PercentFormatter::new(0);
		assert_eq!(fmt0.format(99.9), "100");
	}

	#[test]
	fn test_apply_sort_ascending() {
		let mut items = vec![3u64, 1, 4, 1, 5];
		apply_sort(
			&mut items,
			SortDirection::Asc,
			|x| SortValue::Num(*x),
			|_, _| Ordering::Equal,
		);
		assert_eq!(items, vec![1, 1, 3, 4, 5]);
	}

	#[test]
	fn test_apply_sort_descending() {
		let mut items = vec![3u64, 1, 4, 1, 5];
		apply_sort(
			&mut items,
			SortDirection::Desc,
			|x| SortValue::Num(*x),
			|_, _| Ordering::Equal,
		);
		assert_eq!(items, vec![5, 4, 3, 1, 1]);
	}

	#[test]
	fn test_apply_sort_with_tiebreaker() {
		let mut items = vec![("b", 1), ("a", 1), ("c", 2)];
		apply_sort(
			&mut items,
			SortDirection::Asc,
			|(_, n)| SortValue::Num(*n as u64),
			|(a, _), (b, _)| a.cmp(b),
		);
		assert_eq!(items, vec![("a", 1), ("b", 1), ("c", 2)]);
	}

	#[test]
	fn test_apply_sort_text() {
		let mut items = vec!["banana", "apple", "cherry"];
		apply_sort(
			&mut items,
			SortDirection::Asc,
			|s| SortValue::Text(s),
			|_, _| Ordering::Equal,
		);
		assert_eq!(items, vec!["apple", "banana", "cherry"]);
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
