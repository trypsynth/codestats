 ; expect: lang="MUMPS" total=9 code=7 comment=2 blank=0 shebang=0
HELLO ; Print a greeting and count to five.
 ; Entry point.
 SET NAME="World"
 WRITE "Hello, ",NAME,"!",!
 FOR I=1:1:5 DO
 . WRITE I,!
 SET ^COUNT(NAME)=$GET(^COUNT(NAME))+1
 QUIT
