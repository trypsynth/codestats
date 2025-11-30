use askama::{Result, Values};

/// Escape Markdown table cells by escaping the pipe separator.
pub fn md_escape(value: &str, _values: &dyn Values) -> Result<String> {
	Ok(value.replace('|', "\\|"))
}
