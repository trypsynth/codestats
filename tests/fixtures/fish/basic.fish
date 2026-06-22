#!/usr/bin/env fish
# expect: total=9 code=5 comment=3 blank=1 shebang=0
# simple fish fixture

function greet
	echo "Hello, $argv[1]!"
end
greet "world"
echo "done"
