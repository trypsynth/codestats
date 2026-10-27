// expect: total=13 code=4 comment=5 blank=4 shebang=0
// kotlin nested block comment fixture

/* outer comment /* nested comment */ still outer */

/* another
   multi-line */

fun square(n: Int): Int {
	return n * n
}

fun main() { println(square(4)) }
