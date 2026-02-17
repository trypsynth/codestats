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

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;

	#[test]
	fn view_options_default() {
		let opts = ViewOptions::default();
		assert_eq!(opts.number_style, NumberStyle::Plain);
		assert_eq!(opts.size_style, SizeStyle::Binary);
		assert_eq!(opts.percent_precision, 1);
		assert_eq!(opts.language_sort_key, LanguageSortKey::Lines);
		assert_eq!(opts.sort_direction, SortDirection::Desc);
	}

	#[rstest]
	#[case::plain(NumberStyle::Plain, "\"plain\"")]
	#[case::comma(NumberStyle::Comma, "\"comma\"")]
	#[case::underscore(NumberStyle::Underscore, "\"underscore\"")]
	#[case::space(NumberStyle::Space, "\"space\"")]
	fn number_style_serde_roundtrip(#[case] variant: NumberStyle, #[case] expected_json: &str) {
		let json = serde_json::to_string(&variant).unwrap();
		assert_eq!(json, expected_json);
		let deserialized: NumberStyle = serde_json::from_str(&json).unwrap();
		assert_eq!(deserialized, variant);
	}

	#[rstest]
	#[case::binary(SizeStyle::Binary, "\"binary\"")]
	#[case::decimal(SizeStyle::Decimal, "\"decimal\"")]
	fn size_style_serde_roundtrip(#[case] variant: SizeStyle, #[case] expected_json: &str) {
		let json = serde_json::to_string(&variant).unwrap();
		assert_eq!(json, expected_json);
		let deserialized: SizeStyle = serde_json::from_str(&json).unwrap();
		assert_eq!(deserialized, variant);
	}

	#[rstest]
	#[case::lines(LanguageSortKey::Lines, "\"lines\"")]
	#[case::code(LanguageSortKey::Code, "\"code\"")]
	#[case::comments(LanguageSortKey::Comments, "\"comments\"")]
	#[case::blanks(LanguageSortKey::Blanks, "\"blanks\"")]
	#[case::files(LanguageSortKey::Files, "\"files\"")]
	#[case::size(LanguageSortKey::Size, "\"size\"")]
	#[case::name(LanguageSortKey::Name, "\"name\"")]
	fn language_sort_key_serde_roundtrip(#[case] variant: LanguageSortKey, #[case] expected_json: &str) {
		let json = serde_json::to_string(&variant).unwrap();
		assert_eq!(json, expected_json);
		let deserialized: LanguageSortKey = serde_json::from_str(&json).unwrap();
		assert_eq!(deserialized, variant);
	}

	#[rstest]
	#[case::asc(SortDirection::Asc, "\"asc\"")]
	#[case::desc(SortDirection::Desc, "\"desc\"")]
	fn sort_direction_serde_roundtrip(#[case] variant: SortDirection, #[case] expected_json: &str) {
		let json = serde_json::to_string(&variant).unwrap();
		assert_eq!(json, expected_json);
		let deserialized: SortDirection = serde_json::from_str(&json).unwrap();
		assert_eq!(deserialized, variant);
	}

	#[rstest]
	#[case::plain("\"plain\"", NumberStyle::Plain)]
	#[case::comma("\"comma\"", NumberStyle::Comma)]
	#[case::underscore("\"underscore\"", NumberStyle::Underscore)]
	#[case::space("\"space\"", NumberStyle::Space)]
	fn number_style_from_lowercase(#[case] input: &str, #[case] expected: NumberStyle) {
		let result: NumberStyle = serde_json::from_str(input).unwrap();
		assert_eq!(result, expected);
	}

	#[rstest]
	#[case::binary("\"binary\"", SizeStyle::Binary)]
	#[case::decimal("\"decimal\"", SizeStyle::Decimal)]
	fn size_style_from_lowercase(#[case] input: &str, #[case] expected: SizeStyle) {
		let result: SizeStyle = serde_json::from_str(input).unwrap();
		assert_eq!(result, expected);
	}

	#[rstest]
	#[case::lines("\"lines\"", LanguageSortKey::Lines)]
	#[case::code("\"code\"", LanguageSortKey::Code)]
	#[case::comments("\"comments\"", LanguageSortKey::Comments)]
	#[case::blanks("\"blanks\"", LanguageSortKey::Blanks)]
	#[case::files("\"files\"", LanguageSortKey::Files)]
	#[case::size("\"size\"", LanguageSortKey::Size)]
	#[case::name("\"name\"", LanguageSortKey::Name)]
	fn language_sort_key_from_lowercase(#[case] input: &str, #[case] expected: LanguageSortKey) {
		let result: LanguageSortKey = serde_json::from_str(input).unwrap();
		assert_eq!(result, expected);
	}

	#[rstest]
	#[case::asc("\"asc\"", SortDirection::Asc)]
	#[case::desc("\"desc\"", SortDirection::Desc)]
	fn sort_direction_from_lowercase(#[case] input: &str, #[case] expected: SortDirection) {
		let result: SortDirection = serde_json::from_str(input).unwrap();
		assert_eq!(result, expected);
	}
}
