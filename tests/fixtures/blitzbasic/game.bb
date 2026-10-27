; expect: lang="BlitzBasic" total=19 code=14 comment=2 blank=3 shebang=0
; Bouncing ball demo.
Graphics 640, 480, 0, 2
SetBuffer BackBuffer()

Type Ball
	Field x#, y#, dx#, dy#
End Type

b.Ball = New Ball
b\x = 320 : b\y = 240 : b\dx = 2 : b\dy = 3

Repeat
	Cls
	b\x = b\x + b\dx
	Oval b\x, b\y, 16, 16
	Flip
Until KeyHit(1)
End
