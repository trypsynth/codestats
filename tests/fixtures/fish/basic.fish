#!/usr/bin/env fish
# expect: lang="Fish" total=9 code=5 comment=2 blank=1 shebang=1
# simple fish fixture

function greet
	echo "Hello, $argv[1]!"
end
greet "world"
echo "done"
