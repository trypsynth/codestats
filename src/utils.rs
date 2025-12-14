/// Return `singular` when `count` equals 1, otherwise return `plural`.
#[inline]
pub const fn pluralize<'a>(count: u64, singular: &'a str, plural: &'a str) -> &'a str {
	if count == 1 { singular } else { plural }
}

/// Calculate the percentage that `part` represents of `total`.
///
/// Returns `0.0` when `total` is `0` to avoid division-by-zero panics.
#[inline]
#[expect(clippy::cast_precision_loss)]
pub fn percentage(part: u64, total: u64) -> f64 {
	if total == 0 { 0.0 } else { (part as f64 / total as f64) * 100.0 }
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_pluralize() {
		assert_eq!(pluralize(1, "file", "files"), "file");
		assert_eq!(pluralize(0, "file", "files"), "files");
		assert_eq!(pluralize(2, "file", "files"), "files");
		assert_eq!(pluralize(1, "child", "children"), "child");
		assert_eq!(pluralize(5, "child", "children"), "children");
	}

	#[test]
	fn test_percentage() {
		const EPSILON: f64 = f64::EPSILON;
		assert!((percentage(0, 100) - 0.0).abs() <= EPSILON);
		assert!((percentage(50, 100) - 50.0).abs() <= EPSILON);
		assert!((percentage(25, 100) - 25.0).abs() <= EPSILON);
		assert!((percentage(100, 100) - 100.0).abs() <= EPSILON);
		assert!((percentage(10, 0) - 0.0).abs() <= EPSILON);
		let part = u64::MAX / 2;
		let total = u64::MAX;
		let pct = percentage(part, total);
		assert!((pct - 50.0).abs() < 0.000_000_1);
	}
}
