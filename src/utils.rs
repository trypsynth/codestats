/// Return `singular` when `count` equals 1, otherwise return `plural`.
#[inline]
pub const fn pluralize<'a>(count: u64, singular: &'a str, plural: &'a str) -> &'a str {
	if count == 1 { singular } else { plural }
}

/// Calculate the percentage that `part` represents of `total`.
///
/// Returns `0.0` when `total` is `0` to avoid division-by-zero panics.
#[inline]
#[allow(clippy::cast_precision_loss)]
pub fn percentage(part: u64, total: u64) -> f64 {
	if total == 0 { 0.0 } else { (part as f64 / total as f64) * 100.0 }
}

/// Convert a byte count into a human-readable string using base-1024 units.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn human_size(size: u64) -> String {
	const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
	let mut value = size as f64;
	let mut unit_index = 0;
	while value >= 1024.0 && unit_index < UNITS.len() - 1 {
		value /= 1024.0;
		unit_index += 1;
	}
	match unit_index {
		0 => format!("{size} B"),
		_ if value < 10.0 => format!("{value:.2} {}", UNITS[unit_index]),
		_ if value < 100.0 => format!("{value:.1} {}", UNITS[unit_index]),
		_ => format!("{value:.0} {}", UNITS[unit_index]),
	}
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
		assert_eq!(percentage(0, 100), 0.0);
		assert_eq!(percentage(50, 100), 50.0);
		assert_eq!(percentage(25, 100), 25.0);
		assert_eq!(percentage(100, 100), 100.0);
		assert_eq!(percentage(10, 0), 0.0);
		let part = u64::MAX / 2;
		let total = u64::MAX;
		let pct = percentage(part, total);
		assert!((pct - 50.0).abs() < 0.000000_1);
	}

	#[test]
	fn test_human_size() {
		assert_eq!(human_size(0), "0 B");
		assert_eq!(human_size(1), "1 B");
		assert_eq!(human_size(512), "512 B");
		assert_eq!(human_size(1024), "1.00 KiB");
		assert_eq!(human_size(1536), "1.50 KiB");
		assert_eq!(human_size(10 * 1024), "10.0 KiB");
		assert_eq!(human_size(99 * 1024), "99.0 KiB");
		let one_mib = 1024 * 1024;
		assert_eq!(human_size(one_mib), "1.00 MiB");
		assert_eq!(human_size(one_mib * 20), "20.0 MiB");
		assert_eq!(human_size(one_mib * 200), "200 MiB");
		let gib = 1024_u64.pow(3);
		let tib = 1024_u64.pow(4);
		let pib = 1024_u64.pow(5);
		assert!(human_size(gib).contains("GiB"));
		assert!(human_size(tib).contains("TiB"));
		assert!(human_size(pib).contains("PiB"));
	}
}
