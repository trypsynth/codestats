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
