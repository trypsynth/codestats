// expect: lang="TADS" total=15 code=11 comment=2 blank=2 shebang=0
// Room definitions for the library.
modify Room
    roomDaylight = true
;

replace VerbRule(Look)
    'look' | 'l'
    : LookAction
    verbPhrase = 'look/looking'
;

DefineAction(Ping)
    execAction() { "Pong. "; }
;
