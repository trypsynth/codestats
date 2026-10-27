/* expect: lang="OpenEdge ABL" total=15 code=10 comment=2 blank=3 shebang=0 */
/* Print a report of all customers. */
DEFINE VARIABLE iTotal AS INTEGER NO-UNDO.

FOR EACH Customer NO-LOCK:
    DISPLAY Customer.Name Customer.Balance.
    iTotal = iTotal + 1.
END.

RUN PrintSummary (INPUT iTotal).

PROCEDURE PrintSummary:
    DEFINE INPUT PARAMETER piCount AS INTEGER NO-UNDO.
    MESSAGE "Customers:" piCount.
END PROCEDURE.
