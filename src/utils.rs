#[inline]
pub const fn pluralize<'a>(count: u64, singular: &'a str, plural: &'a str) -> &'a str {
	if count == 1 { singular } else { plural }
}

/// Calculate percentage of part relative to total
///
/// # Note
/// This function performs u64 to f64 conversion which may lose precision
/// for very large values, but this is acceptable for percentage calculations
/// in this context.
#[inline]
#[allow(clippy::cast_precision_loss)]
pub fn percentage(part: u64, total: u64) -> f64 {
	if total == 0 { 0.0 } else { (part as f64 / total as f64) * 100.0 }
}

/// Convert size in bytes to human-readable format
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn human_size(size: u64) -> String {
	human_bytes::human_bytes(size as f64)
}
