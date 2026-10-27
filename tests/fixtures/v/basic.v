// expect: lang="V" total=27 code=21 comment=2 blank=4 shebang=0
module main

// A point in 2D space.
struct Point {
mut:
	x int
	y int
}

enum Color {
	red
	green
}

pub fn (p Point) str() string {
	return '(${p.x}, ${p.y})'
}

fn main() {
	mut p := Point{1, 2}
	p.x = match p.y {
		2 { 10 }
		else { 0 }
	}
	println(p)
}
