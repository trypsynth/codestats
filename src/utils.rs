#[inline]
pub const fn pluralize<'a>(count: u64, singular: &'a str, plural: &'a str) -> &'a str {
	if count == 1 { singular } else { plural }
}

#[inline]
fn u64_to_f64(value: u64) -> f64 {
	const SHIFT: f64 = 4294967296.0; // 2^32
	let high = u32::try_from(value >> 32).expect("shift keeps value within u32");
	let low = u32::try_from(value & u64::from(u32::MAX)).expect("mask keeps value within u32");
	f64::from(high) * SHIFT + f64::from(low)
}

/// Calculate percentage of part relative to total
#[inline]
pub fn percentage(part: u64, total: u64) -> f64 {
	if total == 0 { 0.0 } else { (u64_to_f64(part) / u64_to_f64(total)) * 100.0 }
}

/// Convert size in bytes to a human-readable format.
#[must_use]
pub fn human_size(size: u64) -> String {
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
