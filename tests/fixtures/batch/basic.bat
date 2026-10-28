:: expect: lang="Batch" total=8 code=3 comment=4 blank=1 shebang=0
@echo off
REM uppercase comment
rem lowercase comment
@REM hidden comment

set NAME=world
echo Hello %NAME%
