// expect: total=12 code=7 comment=3 blank=2 shebang=0
// simple rust fixture

fn greet(name: &str) -> String {
	format!("Hello, {name}!")
}

fn main() {
	// print the greeting
	let msg = greet("world");
	println!("{msg}");
}
