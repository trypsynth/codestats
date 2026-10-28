// expect: lang="Java" total=9 code=8 comment=1 blank=0 shebang=0
class Strings {
	String glob = "templates/**";
	char quote = '"';
	String block = """
		/* not a comment
		""";
	String after = "*/";
}
