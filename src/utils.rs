#[inline]
fn u64_to_f64(value: u64) -> f64 {
	const SHIFT: f64 = 4294967296.0; // 2^32
	let high = u32::try_from(value >> 32).expect("shift keeps value within u32");
	let low = u32::try_from(value & u64::from(u32::MAX)).expect("mask keeps value within u32");
	f64::from(high) * SHIFT + f64::from(low)
}

#[inline]
pub(crate) const fn pluralize<'a>(count: u64, singular: &'a str, plural: &'a str) -> &'a str {
	if count == 1 { singular } else { plural }
}

#[inline]
pub(crate) fn percentage(part: u64, total: u64) -> f64 {
	if total == 0 { 0.0 } else { (u64_to_f64(part) / u64_to_f64(total)) * 100.0 }
}

#[must_use]
pub(crate) fn human_size(size: u64) -> String {
	const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
	let mut value = u64_to_f64(size);
	let mut unit_index = 0;
	while value >= 1024.0 && unit_index < UNITS.len() - 1 {
		value /= 1024.0;
		unit_index += 1;
	}
	if unit_index == 0 {
		format!("{size} B")
	} else if value < 10.0 {
		format!("{value:.2} {}", UNITS[unit_index])
	} else if value < 100.0 {
		format!("{value:.1} {}", UNITS[unit_index])
	} else {
		format!("{value:.0} {}", UNITS[unit_index])
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
	fn test_percentage_basic() {
		assert_eq!(percentage(0, 100), 0.0);
		assert_eq!(percentage(50, 100), 50.0);
		assert_eq!(percentage(25, 100), 25.0);
		assert_eq!(percentage(100, 100), 100.0);
		assert_eq!(percentage(10, 0), 0.0);
	}

	#[test]
	fn test_percentage_large_values() {
		let part = u64::MAX / 2;
		let total = u64::MAX;
		let pct = percentage(part, total);
		assert!((pct - 50.0).abs() < 0.000000_1);
	}

	#[test]
	fn test_human_size_bytes() {
		assert_eq!(human_size(0), "0 B");
		assert_eq!(human_size(1), "1 B");
		assert_eq!(human_size(512), "512 B");
	}

	#[test]
	fn test_human_size_kib() {
		assert_eq!(human_size(1024), "1.00 KiB");
		assert_eq!(human_size(1536), "1.50 KiB");
		assert_eq!(human_size(10 * 1024), "10.0 KiB");
		assert_eq!(human_size(99 * 1024), "99.0 KiB");
	}

	#[test]
	fn test_human_size_mib() {
		let one_mib = 1024 * 1024;
		assert_eq!(human_size(one_mib), "1.00 MiB");
		assert_eq!(human_size(one_mib * 20), "20.0 MiB");
		assert_eq!(human_size(one_mib * 200), "200 MiB");
	}

	#[test]
	fn test_human_size_gib_and_up() {
		let gib = 1024_u64.pow(3);
		let tib = 1024_u64.pow(4);
		let pib = 1024_u64.pow(5);
		assert!(human_size(gib).contains("GiB"));
		assert!(human_size(tib).contains("TiB"));
		assert!(human_size(pib).contains("PiB"));
	}
}
