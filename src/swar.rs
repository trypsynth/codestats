//! Byte tests on eight bytes at once, packed little-endian into a `u64` ("SIMD within a register"): each result has a byte's high bit set where the test holds, so the first match is a `trailing_zeros` away instead of a branch per byte.

const ONES: u64 = 0x0101_0101_0101_0101;
/// Every byte's high bit.
const HIGH: u64 = 0x8080_8080_8080_8080;

/// Load up to eight bytes, padding with zeros.
#[inline]
pub fn load(bytes: &[u8]) -> u64 {
	let mut word = [0u8; 8];
	word[..bytes.len()].copy_from_slice(bytes);
	u64::from_le_bytes(word)
}

/// High bit set in each byte with `low <= byte < high`, for ASCII bounds. Bytes of 0x80 and up never match.
#[inline]
pub const fn in_range(x: u64, low: u8, high: u8) -> u64 {
	// Clearing the high bits first keeps the sums from carrying into the next byte.
	let seven = x & !HIGH;
	let at_least_low = seven + (128 - low as u64) * ONES;
	let below_high = seven + (128 - high as u64) * ONES;
	at_least_low & !below_high & !x & HIGH
}

/// High bit set in each byte equal to `b`, an ASCII byte.
#[inline]
pub const fn equal(x: u64, b: u8) -> u64 {
	let seven = x & !HIGH;
	!((seven ^ (b as u64 * ONES)) + !HIGH) & !x & HIGH
}

/// Gather the eight high bits into the low eight bits, byte `i` to bit `i`.
#[inline]
pub const fn bits(high: u64) -> u64 {
	(high >> 7).wrapping_mul(0x0102_0408_1020_4080) >> 56
}

/// High bit set in each identifier byte (ASCII letter, digit or `_`).
#[inline]
pub const fn word(x: u64) -> u64 {
	in_range(x, b'0', b'9' + 1) | in_range(x | 0x2020_2020_2020_2020, b'a', b'z' + 1) | equal(x, b'_')
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn tests_agree_with_std_for_every_byte_in_every_position() {
		for b in 0..=255u8 {
			for position in 0..8 {
				// Surround the byte with others so carries between bytes would show.
				let x = (u64::from(b) << (position * 8))
					| (0xFF_u64.rotate_left(((position + 1) % 8) * 8) & !(0xFF << (position * 8)));
				let get = |mask: u64| bits(mask) >> position & 1 == 1;
				assert_eq!(get(word(x)), b.is_ascii_alphanumeric() || b == b'_', "word {b:#04x} at {position}");
				assert_eq!(get(equal(x, b'_')), b == b'_', "equal {b:#04x} at {position}");
			}
		}
	}
}
