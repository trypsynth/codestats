pub const fn pluralize<'a>(count: u64, singular: &'a str, plural: &'a str) -> &'a str {
	if count == 1 { singular } else { plural }
}

pub fn percentage(part: u64, total: u64) -> f64 {
	if total == 0 { 0.0 } else { (part as f64 / total as f64) * 100.0 }
}
