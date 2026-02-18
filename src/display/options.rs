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

/// Indentation style for output formatting.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum IndentStyle {
	#[default]
	Tab,
	Spaces(u8),
}

impl std::str::FromStr for IndentStyle {
	type Err = String;

	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		if s.eq_ignore_ascii_case("tab") {
			return Ok(Self::Tab);
		}
		match s.parse::<u8>() {
			Ok(n @ 1..=8) => Ok(Self::Spaces(n)),
			_ => Err(format!("indent must be 'tab' or a number 1-8, got '{s}'")),
		}
	}
}

impl std::fmt::Display for IndentStyle {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Tab => f.write_str("tab"),
			Self::Spaces(n) => write!(f, "{n}"),
		}
	}
}

impl Serialize for IndentStyle {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
		serializer.serialize_str(&self.to_string())
	}
}

impl<'de> Deserialize<'de> for IndentStyle {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
		let s = String::deserialize(deserializer)?;
		s.parse().map_err(serde::de::Error::custom)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewOptions {
	pub number_style: NumberStyle,
	pub size_style: SizeStyle,
	pub percent_precision: u8,
	pub language_sort_key: LanguageSortKey,
	pub sort_direction: SortDirection,
	pub indent_style: IndentStyle,
}

impl Default for ViewOptions {
	fn default() -> Self {
		Self {
			number_style: NumberStyle::Plain,
			size_style: SizeStyle::Binary,
			percent_precision: 1,
			language_sort_key: LanguageSortKey::Lines,
			sort_direction: SortDirection::Desc,
			indent_style: IndentStyle::Tab,
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
		assert_eq!(opts.indent_style, IndentStyle::Tab);
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

	#[rstest]
	#[case::tab("tab", IndentStyle::Tab)]
	#[case::one("1", IndentStyle::Spaces(1))]
	#[case::two("2", IndentStyle::Spaces(2))]
	#[case::four("4", IndentStyle::Spaces(4))]
	#[case::eight("8", IndentStyle::Spaces(8))]
	fn parse_indent_style(#[case] input: &str, #[case] expected: IndentStyle) {
		assert_eq!(input.parse::<IndentStyle>().unwrap(), expected);
	}

	#[rstest]
	#[case::zero("0")]
	#[case::nine("9")]
	#[case::garbage("foo")]
	#[case::negative("-1")]
	fn parse_indent_style_rejects_invalid(#[case] input: &str) {
		assert!(input.parse::<IndentStyle>().is_err());
	}

	#[test]
	fn indent_style_display_roundtrip() {
		assert_eq!(IndentStyle::Tab.to_string(), "tab");
		assert_eq!(IndentStyle::Spaces(4).to_string(), "4");
	}

	#[rstest]
	#[case::tab(IndentStyle::Tab, "\"tab\"")]
	#[case::spaces(IndentStyle::Spaces(4), "\"4\"")]
	fn indent_style_serde_roundtrip(#[case] variant: IndentStyle, #[case] expected_json: &str) {
		let json = serde_json::to_string(&variant).unwrap();
		assert_eq!(json, expected_json);
		let deserialized: IndentStyle = serde_json::from_str(&json).unwrap();
		assert_eq!(deserialized, variant);
	}
}
