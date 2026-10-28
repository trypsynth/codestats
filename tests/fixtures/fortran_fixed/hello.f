*     expect: lang="Fortran (Fixed Form)" total=10 code=6 comment=3 blank=1 shebang=0
      PROGRAM HELLO
C     Print a greeting.
c     lowercase comment marker
      CHARACTER*5 NAME
      NAME = 'WORLD'
      CALL GREET(NAME)

      STOP
      END
