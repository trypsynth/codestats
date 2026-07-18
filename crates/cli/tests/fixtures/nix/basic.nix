# expect: total=11 code=5 comment=4 blank=2 shebang=0
# simple nix fixture

/* nix expression */
# configuration
{
	pkgs ? import <nixpkgs> {}
}: {

	packages = [ pkgs.git pkgs.vim ];
}
