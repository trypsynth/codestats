// expect: lang="C#" total=9 code=8 comment=1 blank=0 shebang=0
class Paths {
	string glob = "src/**/*.cs";
	string verbatim = @"C:\temp\";
	string after = "/* not a comment";
	string raw = """
		/* raw */
		""";
}
