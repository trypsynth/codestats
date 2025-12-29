use askama::{Result, Values};

use super::FormatterContext;

#[askama::filter_fn]
#[expect(clippy::unnecessary_wraps, clippy::trivially_copy_pass_by_ref)]
pub fn fmt_number(value: &u64, _values: &dyn Values, ctx: &FormatterContext) -> Result<String> {
	Ok(ctx.number(*value))
}

#[askama::filter_fn]
#[expect(clippy::unnecessary_wraps, clippy::trivially_copy_pass_by_ref)]
pub fn fmt_percent(value: &f64, _values: &dyn Values, ctx: &FormatterContext) -> Result<String> {
	Ok(ctx.percent(*value))
}
