// Adapted from mezura (https://github.com/subamanis/mezura): the block masks and `scan_block` from
// mezura-core/src/engine/masks.rs, and `BlockScanner::sweep` from `MaskedLines::book_plain_lines` in
// mezura-core/src/engine/file_parser.rs. Used under the MIT license.
// Copyright (c) 2021 Petros Papatheodorou

//! Sweeping plain lines a 64-byte block at a time. One pass over a block answers three questions for each of its bytes, as bitmasks: is it a newline, could it start a marker, and is it anything but whitespace. Lines without a marker are then counted off the masks, with no search per line.

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::{
	uint8x16_t, vandq_u8, vceqq_u8, vcleq_u8, vdupq_n_u8, vgetq_lane_u64, vld1q_u8, vorrq_u8, vpaddq_u8,
	vreinterpretq_u64_u8, vsubq_u8,
};
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::{
	__m256i, _mm256_cmpeq_epi8, _mm256_loadu_si256, _mm256_min_epu8, _mm256_movemask_epi8, _mm256_or_si256,
	_mm256_set1_epi8, _mm256_setzero_si256, _mm256_sub_epi8,
};

use super::{line_classifier::LineType, stats::LineCounts};

const BLOCK: usize = 64;
/// With more marker bytes than this, a comparison per byte costs more than the masks save.
const MOST_SEARCHED: usize = 8;

#[derive(Clone, Copy, Default)]
struct Masks {
	newlines: u64,
	candidates: u64,
	text: u64,
}

/// A scanner over one file's bytes, remembering its last block, as the sweep often comes back to it after the classifier has taken a line.
pub(super) struct BlockScanner<'a> {
	bytes: &'a [u8],
	/// The block `masks` describes, as its start and the bytes searched for in it, the start being `usize::MAX` before the first.
	loaded: (usize, &'a [u8]),
	masks: Masks,
}

impl<'a> BlockScanner<'a> {
	/// A scanner over `bytes`, or `None` where the machine doesn't suit it.
	pub(super) fn new(bytes: &'a [u8]) -> Option<Self> {
		#[cfg(target_arch = "x86_64")]
		let suited = is_x86_feature_detected!("avx2")
			&& is_x86_feature_detected!("popcnt")
			&& is_x86_feature_detected!("lzcnt")
			&& is_x86_feature_detected!("bmi1");
		// Every 64-bit Arm has NEON.
		#[cfg(target_arch = "aarch64")]
		let suited = true;
		#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
		let suited = false;
		suited.then_some(Self { bytes, loaded: (usize::MAX, &[]), masks: Masks::default() })
	}

	/// Whether a search for `searched` is worth making this way.
	pub(super) const fn suits(searched: &[u8]) -> bool {
		searched.len() <= MOST_SEARCHED
	}

	#[inline]
	fn load(&mut self, start: usize, searched: &'a [u8]) {
		if self.loaded.0 != start || !std::ptr::eq(self.loaded.1, searched) {
			self.masks = scan(self.bytes, start, searched);
			self.loaded = (start, searched);
		}
	}

	/// Count the lines from `pos` up to the first holding a `searched` byte, each as `with_text`, or blank if it's only whitespace. Returns where that line starts and `true`, or the end of the bytes and `false` if there's no such line.
	///
	/// A block's lines are counted together, off the masks, as which of them are blank is too irregular to branch on line by line.
	pub(super) fn sweep(
		&mut self,
		pos: usize,
		searched: &'a [u8],
		with_text: LineType,
		counts: &mut LineCounts,
	) -> (usize, bool) {
		// SAFETY: a scanner only exists once `BlockScanner::new` has found these features.
		#[cfg(target_arch = "x86_64")]
		return unsafe { self.sweep_avx2(pos, searched, with_text, counts) };
		#[cfg(not(target_arch = "x86_64"))]
		self.sweep_blocks(pos, searched, with_text, counts)
	}

	/// The sweep built with the features `scan_block` and the bit counts need, so that both are inlined into its loop rather than called once per block.
	#[cfg(target_arch = "x86_64")]
	#[target_feature(enable = "avx2,popcnt,lzcnt,bmi1")]
	fn sweep_avx2(
		&mut self,
		pos: usize,
		searched: &'a [u8],
		with_text: LineType,
		counts: &mut LineCounts,
	) -> (usize, bool) {
		self.sweep_blocks(pos, searched, with_text, counts)
	}

	#[expect(
		clippy::inline_always,
		reason = "it must take on the features of `sweep_avx2`, which only inlining gives it"
	)]
	#[inline(always)]
	fn sweep_blocks(
		&mut self,
		pos: usize,
		searched: &'a [u8],
		with_text: LineType,
		counts: &mut LineCounts,
	) -> (usize, bool) {
		let mut start = pos;
		let (mut lines, mut lines_with_text) = (0, 0);
		// Whether the line the block starts in has text in earlier blocks.
		let mut has_text = false;
		let mut block = pos / BLOCK * BLOCK;
		let mut from = !0u64 << (pos - block);
		let stopped = loop {
			self.load(block, searched);
			let Masks { newlines, candidates, text } = self.masks;
			let (newlines, candidates, text) = (newlines & from, candidates & from, text & from);
			from = !0;
			// The lines ending before the first candidate; the one holding it stops the sweep.
			let ended = newlines & candidates.wrapping_sub(1) & !candidates;
			if ended != 0 {
				lines += ended.count_ones();
				lines_with_text += (ended & with_text_before(newlines, text, has_text)).count_ones();
				let last = ended.ilog2();
				start = block + last as usize + 1;
				has_text = text & !(u64::MAX >> (63 - last)) != 0;
			} else {
				has_text |= text != 0;
			}
			if candidates != 0 {
				break (start, true);
			}
			block += BLOCK;
			if block >= self.bytes.len() {
				if start < self.bytes.len() {
					lines += 1;
					lines_with_text += u32::from(has_text);
				}
				break (self.bytes.len(), false);
			}
		};
		counts.add_many(with_text, u64::from(lines_with_text));
		counts.add_many(LineType::Blank, u64::from(lines - lines_with_text));
		stopped
	}
}

/// The newlines ending a line with text: one after a text byte, or the first after the block's start if `carried`, the line it starts in having text already.
///
/// The bytes between two newlines are a run of ones in `!newlines`, and adding the run's text bytes to it carries a one out of the run's top onto its newline exactly when there's at least one. A carry stops at the newline, which is a zero.
const fn with_text_before(newlines: u64, text: u64, carried: bool) -> u64 {
	(!newlines).wrapping_add(text).wrapping_add(carried as u64)
}

/// The masks of the block at `start`. A block running past the end of `bytes`, the last of a file, says nothing about the bytes it lacks.
#[inline]
fn scan(bytes: &[u8], start: usize, searched: &[u8]) -> Masks {
	if let Some(Ok(block)) = bytes.get(start..start + BLOCK).map(<&[u8; BLOCK]>::try_from) {
		// SAFETY: a scanner only exists once `BlockScanner::new` has found the features `scan_block` needs.
		return unsafe { scan_block(block, searched) };
	}
	let rest = &bytes[start..];
	let mut block = [0; BLOCK];
	block[..rest.len()].copy_from_slice(rest);
	// SAFETY: as above.
	let mut masks = unsafe { scan_block(&block, searched) };
	let wanted = (1u64 << rest.len()) - 1;
	masks.candidates &= wanted;
	masks.text &= wanted;
	masks
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
fn scan_block(block: &[u8; BLOCK], searched: &[u8]) -> Masks {
	let [low, high]: [__m256i; 2] = {
		let at = block.as_ptr();
		// SAFETY: both 32-byte loads are inside the 64-byte block, and these loads don't need alignment.
		unsafe { [_mm256_loadu_si256(at.cast()), _mm256_loadu_si256(at.add(32).cast())] }
	};
	let bits = |low: __m256i, high: __m256i| {
		u64::from(_mm256_movemask_epi8(low).cast_unsigned())
			| (u64::from(_mm256_movemask_epi8(high).cast_unsigned()) << 32)
	};
	let newline = _mm256_set1_epi8(b'\n'.cast_signed());
	let newlines = bits(_mm256_cmpeq_epi8(low, newline), _mm256_cmpeq_epi8(high, newline));
	let (mut low_hits, mut high_hits) = (_mm256_setzero_si256(), _mm256_setzero_si256());
	for &byte in searched {
		let wanted = _mm256_set1_epi8(byte.cast_signed());
		low_hits = _mm256_or_si256(low_hits, _mm256_cmpeq_epi8(low, wanted));
		high_hits = _mm256_or_si256(high_hits, _mm256_cmpeq_epi8(high, wanted));
	}
	Masks { newlines, candidates: bits(low_hits, high_hits), text: !bits(whitespace(low), whitespace(high)) }
}

/// Which bytes `is_blank` takes for whitespace: a space, or a tab through a carriage return.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
fn whitespace(bytes: __m256i) -> __m256i {
	let from_tab = _mm256_sub_epi8(bytes, _mm256_set1_epi8(b'\t'.cast_signed()));
	let up_to_return = _mm256_set1_epi8((b'\r' - b'\t').cast_signed());
	_mm256_or_si256(
		_mm256_cmpeq_epi8(_mm256_min_epu8(from_tab, up_to_return), from_tab),
		_mm256_cmpeq_epi8(bytes, _mm256_set1_epi8(b' '.cast_signed())),
	)
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
fn scan_block(block: &[u8; BLOCK], searched: &[u8]) -> Masks {
	let chunks: [uint8x16_t; 4] = {
		let at = block.as_ptr();
		// SAFETY: all four 16-byte loads are inside the 64-byte block, and these loads don't need alignment.
		unsafe { [vld1q_u8(at), vld1q_u8(at.add(16)), vld1q_u8(at.add(32)), vld1q_u8(at.add(48))] }
	};
	let newline = vdupq_n_u8(b'\n');
	let newlines = bits(chunks.map(|chunk| vceqq_u8(chunk, newline)));
	let mut hits = [vdupq_n_u8(0); 4];
	for &byte in searched {
		let wanted = vdupq_n_u8(byte);
		for (hit, chunk) in hits.iter_mut().zip(chunks) {
			*hit = vorrq_u8(*hit, vceqq_u8(chunk, wanted));
		}
	}
	let space = vdupq_n_u8(b' ');
	let (tab, up_to_return) = (vdupq_n_u8(b'\t'), vdupq_n_u8(b'\r' - b'\t'));
	let whitespace = chunks.map(|chunk| vorrq_u8(vcleq_u8(vsubq_u8(chunk, tab), up_to_return), vceqq_u8(chunk, space)));
	Masks { newlines, candidates: bits(hits), text: !bits(whitespace) }
}

/// Never called: other machines get no scanner.
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
unsafe fn scan_block(_: &[u8; BLOCK], _: &[u8]) -> Masks {
	unreachable!("`BlockScanner::new` makes no scanner on this machine")
}

/// One bit per byte of four comparison results, which NEON has no single instruction for: each byte keeps its own bit of its position, and three pairwise additions pack them together.
#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
fn bits(chunks: [uint8x16_t; 4]) -> u64 {
	const POSITIONS: [u8; 16] = [1, 2, 4, 8, 16, 32, 64, 128, 1, 2, 4, 8, 16, 32, 64, 128];
	// SAFETY: the array holds the 16 bytes loaded.
	let positions = unsafe { vld1q_u8(POSITIONS.as_ptr()) };
	let [a, b, c, d] = chunks.map(|chunk| vandq_u8(chunk, positions));
	let packed = vpaddq_u8(vpaddq_u8(a, b), vpaddq_u8(c, d));
	vgetq_lane_u64::<0>(vreinterpretq_u64_u8(vpaddq_u8(packed, packed)))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::analysis::line_classifier::is_blank;

	/// Lines of every length around the block size, with markers and whitespace on either side of a block's edge, count as a look at each line would.
	#[test]
	fn sweeps_match_lines_across_block_edges() {
		let mut seed: u64 = 0x1234_5678_9abc_def1;
		let mut next = |bound: usize| {
			seed ^= seed << 13;
			seed ^= seed >> 7;
			seed ^= seed << 17;
			usize::try_from(seed % bound as u64).unwrap()
		};
		let searched: &[u8] = b"#\"";
		for _ in 0..3000 {
			let mut text = Vec::new();
			for _ in 0..next(8) {
				let (len, fill) = (next(140), [b'x', b' ', b'\t', b'\r', 0x0B, 0x0C][next(6)]);
				text.extend(std::iter::repeat_n(fill, len));
				if next(6) == 0 {
					text.push(if next(2) == 0 { b'#' } else { b'"' });
				}
				text.extend(std::iter::repeat_n(b' ', next(3)));
				if next(8) != 0 {
					text.push(b'\n');
				}
			}
			let Some(mut scanner) = BlockScanner::new(&text) else { return };
			let start = next(text.len().max(1)).min(text.len().saturating_sub(1));
			let start = text[..start].iter().rposition(|&b| b == b'\n').map_or(0, |at| at + 1);
			if start >= text.len() {
				continue;
			}
			let mut counts = LineCounts::default();
			let (stopped, at_marker) = scanner.sweep(start, searched, LineType::Comment, &mut counts);
			let mut expected = LineCounts::default();
			let mut pos = start;
			let mut expected_marker = false;
			while pos < text.len() {
				let end = text[pos..].iter().position(|&b| b == b'\n').map_or(text.len(), |at| pos + at + 1);
				if text[pos..end].iter().any(|b| searched.contains(b)) {
					expected_marker = true;
					break;
				}
				expected.add(if is_blank(&text[pos..end]) { LineType::Blank } else { LineType::Comment });
				pos = end;
			}
			assert_eq!(
				(counts, stopped, at_marker),
				(expected, pos, expected_marker),
				"{:?} from {start}",
				String::from_utf8_lossy(&text)
			);
		}
	}
}
