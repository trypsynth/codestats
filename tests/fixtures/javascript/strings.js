// expect: lang="JavaScript" total=8 code=6 comment=1 blank=1 shebang=0
const patterns = ["src/**/*.js", 'lib/*'];
const url = "http://example.com";
const tpl = `multi-line
/* still a template */
// also inside the template`;

const done = true;
