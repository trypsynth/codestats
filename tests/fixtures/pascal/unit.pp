// expect: lang="Pascal" total=21 code=12 comment=3 blank=6 shebang=0
unit MathUtils;

{$mode objfpc}

interface

// Return the larger of two integers.
function Max(A, B: Integer): Integer;

implementation

function Max(A, B: Integer): Integer;
begin
  if A > B then
    Result := A
  else
    Result := B;
end;

end.
