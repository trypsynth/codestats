// expect: lang="Pascal" total=11 code=7 comment=2 blank=2 shebang=0
{ Classic Pascal hello world. }
program Hello;

var
  i: integer;

begin
  for i := 1 to 3 do
    writeln('Hello, world!');
end.
