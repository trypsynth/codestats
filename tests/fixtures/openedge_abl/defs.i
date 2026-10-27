/* expect: lang="OpenEdge ABL" total=8 code=5 comment=2 blank=1 shebang=0 */
/* Shared variable definitions. */
DEFINE VARIABLE cName AS CHARACTER NO-UNDO.
DEFINE VARIABLE iCount AS INTEGER NO-UNDO INITIAL 0.

FUNCTION FormatName RETURNS CHARACTER (INPUT pcName AS CHARACTER):
    RETURN CAPS(pcName).
END FUNCTION.
