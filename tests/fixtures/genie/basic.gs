// expect: lang="Genie" total=17 code=11 comment=2 blank=4 shebang=0
// A small Genie program.
uses
	GLib

init
	var greeter = new Greeter("world")
	greeter.greet()

class Greeter : Object
	prop name : string

	construct (name : string)
		self.name = name

	def greet()
		print "Hello, %s!", name
