; expect: total=12 code=6 comment=4 blank=2 shebang=0
; simple assembly fixture

section .data
	; message constant
	msg db "Hello", 0

section .text
	; entry point
	global _start
_start:
	mov eax, 4
