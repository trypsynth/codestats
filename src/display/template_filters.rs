use askama::{Result as AskamaResult, Values};

use super::FormatterContext;

#[expect(clippy::unnecessary_wraps, clippy::trivially_copy_pass_by_ref)]
pub fn fmt_number(value: &u64, _values: &dyn Values, ctx: &FormatterContext) -> AskamaResult<String> {
	Ok(ctx.number(*value))
}

#[expect(clippy::unnecessary_wraps, clippy::trivially_copy_pass_by_ref)]
pub fn fmt_percent(value: &f64, _values: &dyn Values, ctx: &FormatterContext) -> AskamaResult<String> {
	Ok(ctx.percent(*value))
}
