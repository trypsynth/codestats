pub const fn pluralize<'a>(count: u64, singular: &'a str, plural: &'a str) -> &'a str {
	if count == 1 { singular } else { plural }
}

/// Calculate percentage of part relative to total
///
/// # Note
/// This function performs u64 to f64 conversion which may lose precision
/// for very large values, but this is acceptable for percentage calculations
/// in this context.
#[allow(clippy::cast_precision_loss)]
pub fn percentage(part: u64, total: u64) -> f64 {
	if total == 0 { 0.0 } else { (part as f64 / total as f64) * 100.0 }
}

/// Safely convert u64 to f64 for display purposes
/// This function explicitly handles the precision loss that can occur
/// when converting large u64 values to f64
#[allow(clippy::cast_precision_loss)]
pub const fn size_to_f64(size: u64) -> f64 {
	size as f64
}
