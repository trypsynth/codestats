// expect: lang="C" total=12 code=8 comment=3 blank=1 shebang=0
#include <stdio.h>

/* it's a "test */
int main(void) {
	printf("/*\n");
	printf("// not a comment\n");
	char q = '"';
	puts("done */");
	// don't stop counting
	return 0;
}
