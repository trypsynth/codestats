# expect: total=9 code=4 comment=3 blank=2 shebang=0
# tab-indented python fixture

def greet(name):
	# return a greeting
	return f"Hello, {name}!"

result = greet("world")
print(result)
