// expect: lang="TADS" total=20 code=14 comment=2 blank=4 shebang=0
#charset "us-ascii"
#include <adv3.h>

// The starting location.
startRoom: Room 'Hallway'
    "A long hallway stretches north. "
;

+ lamp: Thing 'brass lamp' 'lamp'
    "A dented brass lamp. "
;

gameMain: GameMainDef
    initialPlayerChar = me
;

modify Thing
    dobjFor(Examine) { verify() { } }
;
