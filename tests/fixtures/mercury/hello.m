% expect: lang="Mercury" total=11 code=7 comment=2 blank=2 shebang=0
% Hello world in Mercury.
:- module hello.
:- interface.
:- import_module io.
:- pred main(io::di, io::uo) is det.

:- implementation.

main(!IO) :-
    io.write_string("Hello, world!\n", !IO).
