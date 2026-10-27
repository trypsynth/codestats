// expect: lang="Bison" total=22 code=17 comment=2 blank=3 shebang=0
/* Calculator grammar using Bison extensions. */
%require "3.2"
%language "c++"
%define api.value.type variant
%define parse.error verbose

%code requires {
  #include <string>
}

%token <int> NUMBER
%token PLUS MINUS
%type <int> expr
%start expr

%%
expr: NUMBER
    | expr PLUS NUMBER  { $$ = $1 + $3; }
    | expr MINUS NUMBER { $$ = $1 - $3; }
    ;
%%
