# expect: lang="Limbo" total=12 code=8 comment=2 blank=2 shebang=0
# Module interface for the hello program.
Hello: module
{
	PATH: con "/dis/hello.dis";

	Point: adt {
		x, y: int;
	};

	init: fn(ctxt: ref Draw->Context, argv: list of string);
};
