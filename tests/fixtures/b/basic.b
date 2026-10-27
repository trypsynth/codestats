/* expect: lang="B" total=12 code=10 comment=2 blank=0 shebang=0 */
/* Print the numbers from 0 to 9. */
main() {
	extrn putchar;
	auto i;
	i = 0;
	while (i < 10) {
		putchar('0' + i);
		i++;
	}
	putchar('*n');
}
