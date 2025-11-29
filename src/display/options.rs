use clap::ValueEnum;

/// Controls how integer counts are rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum NumberStyle {
	/// Emit plain digits without any grouping.
	Plain,
	/// Use `,` as the thousands separator (e.g. 12,345).
	Comma,
	/// Use `_` as the thousands separator (e.g. `12_345`).
	Underscore,
	/// Use a space as the thousands separator (e.g. 12 345).
	Space,
}

/// Controls whether human-readable sizes use decimal (KB) or binary (KiB) units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SizeStyle {
	/// Use base-1024 units (KiB, MiB, ...).
	Binary,
	/// Use base-1000 units (KB, MB, ...).
	Decimal,
}

/// Field used when ordering languages (and optionally files).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LanguageSortKey {
	Lines,
	Code,
	Comments,
	Blanks,
	Files,
	Size,
	Name,
}

/// Direction for applying a sort key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SortDirection {
	Asc,
	Desc,
}

/// Shared view configuration passed into all formatters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewOptions {
	pub number_style: NumberStyle,
	pub size_style: SizeStyle,
	pub percent_precision: u8,
	pub language_sort_key: LanguageSortKey,
	pub sort_direction: SortDirection,
}

impl Default for ViewOptions {
	fn default() -> Self {
		Self {
			number_style: NumberStyle::Plain,
			size_style: SizeStyle::Binary,
			percent_precision: 1,
			language_sort_key: LanguageSortKey::Lines,
			sort_direction: SortDirection::Desc,
		}
	}
}
