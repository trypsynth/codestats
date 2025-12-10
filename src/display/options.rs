use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NumberStyle {
	Plain,
	Comma,
	Underscore,
	Space,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SizeStyle {
	Binary,
	Decimal,
}

/// Field used when ordering languages (and optionally files).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
	Asc,
	Desc,
}

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
