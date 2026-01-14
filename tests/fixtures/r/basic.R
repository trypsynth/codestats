# expect: total=10 code=5 comment=3 blank=2 shebang=0
# simple R fixture

greet <- function(name) {
	paste("hi", name)
}

# call it
msg <- greet("world")
print(msg)
