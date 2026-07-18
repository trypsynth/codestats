// expect: total=14 code=6 comment=5 blank=3 shebang=0
/* rust supports block comments */

/* multi-line
   block comment
   continues here */

fn add(a: i32, b: i32) -> i32 { a + b }

fn main() {
	let x = add(3, 4);
	let y = add(x, 5);
	println!("{}", y);
}
