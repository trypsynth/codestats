// expect: total=14 code=5 comment=5 blank=4 shebang=0
// simple swift fixture

/*
 multiple
 lines */

import Foundation

func greet(_ name: String) {
	print("Hi \\(name)")
}

greet("Swift")
