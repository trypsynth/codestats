// expect: total=13 code=6 comment=3 blank=4 shebang=0
<?php
// simple php fixture

/* doc comment */

$greeting = "hello";
function add($a, $b) {
	return $a + $b;
}

echo add(1, 2);
