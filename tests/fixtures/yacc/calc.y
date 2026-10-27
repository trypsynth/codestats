// expect: lang="Yacc" total=19 code=14 comment=2 blank=3 shebang=0
%{
#include <stdio.h>
int yylex(void);
void yyerror(const char *s);
%}

%token NUMBER
%left '+' '-'

%%
/* A sum of numbers. */
expr: expr '+' expr { $$ = $1 + $3; }
    | expr '-' expr { $$ = $1 - $3; }
    | NUMBER
    ;
%%

int main(void) { return yyparse(); }
