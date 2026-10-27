-- expect: lang="Happy" total=24 code=17 comment=2 blank=5 shebang=0
{
module Parser where
import Lexer
}

%name parseExpr
%tokentype { Token }
%error { parseError }

%token
  int { TInt $$ }
  '+' { TPlus }

%%

-- Expressions are sums of integers.
Expr : Expr '+' int { $1 + $3 }
     | int          { $1 }

{
parseError :: [Token] -> a
parseError _ = error "Parse error"
}
