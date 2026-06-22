# expect: total=9 code=5 comment=3 blank=1 shebang=0
# simple crystal fixture

# greet function
def greet(name : String) : String
	"Hello, #{name}!"
end
puts greet("world")
x = 1 + 2
