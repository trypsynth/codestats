// expect: lang="Structured Text" total=16 code=12 comment=3 blank=1 shebang=0
(* Motor control with start/stop latch. *)
PROGRAM MotorControl
VAR
    StartButton : BOOL;
    StopButton : BOOL;
    MotorOn : BOOL;
END_VAR

// Latch the motor on until stop is pressed.
IF StartButton AND NOT StopButton THEN
    MotorOn := TRUE;
ELSIF StopButton THEN
    MotorOn := FALSE;
END_IF;
END_PROGRAM
