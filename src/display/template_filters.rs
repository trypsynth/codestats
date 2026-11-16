//! Askama template filters used by Markdown and HTML reports.
#![allow(
	clippy::inline_always,
	clippy::missing_errors_doc,
	clippy::trivially_copy_pass_by_ref,
	clippy::unnecessary_wraps,
	reason = "the askama `#[filter_fn]` macro generates wrapper functions that trigger these"
)]

use askama::{Result, Values};

use super::FormatterContext;

/// # Errors
///
/// Never returns an error; signature required by the askama filter API.
#[askama::filter_fn]
pub fn fmt_number(value: &u64, _values: &dyn Values, ctx: &FormatterContext) -> Result<String> {
	Ok(ctx.number(*value))
}

/// # Errors
///
/// Never returns an error; signature required by the askama filter API.
#[askama::filter_fn]
pub fn fmt_size(value: &u64, _values: &dyn Values, ctx: &FormatterContext) -> Result<String> {
	Ok(ctx.size(*value))
}

/// # Errors
///
/// Never returns an error; signature required by the askama filter API.
#[askama::filter_fn]
pub fn fmt_float(value: &f64, _values: &dyn Values, precision: usize) -> Result<String> {
	Ok(format!("{value:.precision$}"))
}
