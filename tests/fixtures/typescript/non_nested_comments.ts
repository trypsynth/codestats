// expect: lang="TypeScript" total=6 code=3 comment=2 blank=1 shebang=0
/* outer /* block comments do not nest */
const a: number = 1;

export function f(): number {
	return a; }
