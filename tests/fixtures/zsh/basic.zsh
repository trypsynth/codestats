#!/bin/zsh
# expect: total=9 code=5 comment=2 blank=1 shebang=1
# simple zsh fixture

function greet() {
	echo "Hello, $1!"
}
greet "world"
echo "done"
