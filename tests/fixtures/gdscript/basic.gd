# expect: total=10 code=6 comment=3 blank=1 shebang=0
# simple gdscript fixture

# greet function
extends Node
func greet(name):
	print("Hello, " + name + "!")
var x = 1 + 2
var y = x * 3
print(y)
