// expect: total=11 code=4 comment=5 blank=2 shebang=0
// jsdoc-style block comment fixture

/**
 * Add two numbers.
 */
function add(a, b) { return a + b; }

const x = add(3, 4);
const y = add(x, 1);
console.log(x + y);
