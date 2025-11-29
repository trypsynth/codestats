// expect: total=12 code=5 comment=3 blank=4 shebang=0
// simple typescript fixture

/* block comment */

type Point = { x: number; y: number };
function add(a: number, b: number): number {
	return a + b;
}

const origin: Point = { x: 0, y: 0 };
