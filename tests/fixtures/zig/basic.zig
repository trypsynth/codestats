// expect: total=16 code=8 comment=4 blank=4 shebang=0
// Simple Zig fixture.

const std = @import("std");

/// Doc comment above a type.
const Thing = struct {
	//! Top-level comment inside a container.
	const A: f64 = 2.93821;
};

pub fn main() {
	const a = Thing.A;
	std.debug.print("{d}", a);
}
