// expect: lang="Harbour" total=22 code=16 comment=2 blank=4 shebang=0
// Harbour class example.
#include "hbclass.ch"

PROCEDURE Main()
   LOCAL oGreeter := Greeter():New( "world" )
   oGreeter:Greet()
   RETURN

CLASS Greeter
   DATA cName
   METHOD New( cName )
   METHOD Greet()
ENDCLASS

METHOD New( cName ) CLASS Greeter
   ::cName := cName
   RETURN Self

METHOD Greet() CLASS Greeter
   ? "Hello, " + ::cName
   RETURN NIL
