#!/usr/bin/env factor
! expect: total=9 code=4 comment=2 blank=2 shebang=1
! simple factor fixture

USING: io kernel ;
IN: sample

: greet ( name -- ) "hi " swap append print ;
"world" greet
