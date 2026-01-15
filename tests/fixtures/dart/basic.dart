// expect: total=11 code=6 comment=3 blank=2 shebang=0
// simple dart fixture

String greet(String name) {
	return "hi $name";
}

// call it
void main() {
	print(greet("world"));
}
