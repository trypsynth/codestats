/* expect: lang="OpenEdge ABL" total=14 code=10 comment=2 blank=2 shebang=0 */
/* Customer business entity. */
USING Progress.Lang.*.

CLASS Customer:
    DEFINE PUBLIC PROPERTY Name AS CHARACTER NO-UNDO
        GET. SET.

    METHOD PUBLIC VOID Load(INPUT piId AS INTEGER):
        FIND FIRST CustTable WHERE CustTable.Id = piId NO-LOCK NO-ERROR.
        IF AVAILABLE CustTable THEN
            Name = CustTable.Name.
    END METHOD.
END CLASS.
