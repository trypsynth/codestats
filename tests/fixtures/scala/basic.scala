// expect: total=11 code=6 comment=3 blank=2 shebang=0
// simple scala fixture

object Hello {
	def greet(name: String): String = {
		s"hi $name"
	}

	// call it
	println(greet("world"))
}
