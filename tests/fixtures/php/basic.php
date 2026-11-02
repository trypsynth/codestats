// expect: lang="PHP" total=12 code=7 comment=2 blank=3 shebang=0
<?php
// simple php fixture

/* doc comment */

$greeting = "hello";
function add($a, $b) {
	return $a + $b;
}

echo add(1, 2);
