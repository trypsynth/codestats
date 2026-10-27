// expect: lang="Lex" total=19 code=15 comment=2 blank=2 shebang=0
%{
#include <stdio.h>
int lines = 0;
%}

%%
\n          { lines++; }
[0-9]+      { printf("NUMBER %s\n", yytext); }
[a-zA-Z]+   { printf("WORD %s\n", yytext); }
.           { ECHO; }
%%

/* Count lines in the input. */
int main(void) {
	yylex();
	printf("%d lines\n", lines);
	return 0;
}
