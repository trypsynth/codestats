// expect: total=11 code=6 comment=3 blank=2 shebang=0
// simple groovy fixture

/* scripting fixture */
def add = { a, b -> a + b }
def result = add(3, 4)

class Greeter {
	String name
	def greet() { println "Hi, ${name}!" }
}
