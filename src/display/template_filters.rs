//! Askama template filters used by Markdown and HTML reports.
#![expect(clippy::unnecessary_wraps)]
// The askama `#[filter_fn]` macro generates code that triggers these lints.
#![allow(clippy::inline_always, clippy::trivially_copy_pass_by_ref)]

use askama::{Result, Values};

use super::FormatterContext;

#[askama::filter_fn]
pub fn fmt_number(value: &u64, _values: &dyn Values, ctx: &FormatterContext) -> Result<String> {
	Ok(ctx.number(*value))
}

#[askama::filter_fn]
pub fn fmt_percent(value: &f64, _values: &dyn Values, ctx: &FormatterContext) -> Result<String> {
	Ok(ctx.percent(*value))
}

#[askama::filter_fn]
pub fn fmt_float(value: &f64, _values: &dyn Values, precision: usize) -> Result<String> {
	Ok(format!("{value:.precision$}"))
}
