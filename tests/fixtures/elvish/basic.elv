# expect: total=9 code=5 comment=2 blank=2 shebang=0
# simple elvish fixture

fn greet [name]{
	put "hi $name"
}

greet world
set x = 1
