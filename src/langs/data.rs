/// How a language embeds code in others, whose regions are counted by those languages' rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Embedding {
	/// `<script>` and `<style>` elements.
	Html,
	/// HTML's elements, and a TypeScript frontmatter between `---` lines.
	Astro,
	/// Fenced code blocks, and YAML (`---`) or TOML (`+++`) frontmatter.
	Markdown,
	/// `[source,python]` listing blocks, and fenced code blocks.
	AsciiDoc,
	/// `#+BEGIN_SRC python` blocks.
	Org,
	/// `.. code-block:: python` directives.
	ReStructuredText,
	/// HTML outside `<?php ... ?>` tags, with HTML's own elements inside it.
	Php,
}

/// A language's heredocs (`<<EOF`): string literals whose body runs from the next line to a line holding just the terminator. They differ in where a space may come between the opener and the terminator, which tells them apart from a shift or append (`x << y`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Heredoc {
	/// `<<EOF`, `<<-EOF`, `<< 'EOF'`: spaces allowed.
	Shell,
	/// `<<EOF`, `<<~EOF`, `<< "EOF"`: a space only before a quoted terminator.
	Perl,
	/// `<<EOF`, `<<~EOF`, `<<-'EOF'`: no space, as `x << y` appends.
	Ruby,
	/// `<<<EOT`, `<<< "EOT"`: spaces allowed, as `<<<` is nothing else.
	Php,
}

impl Heredoc {
	#[must_use]
	pub const fn opener(self) -> &'static str {
		match self {
			Self::Php => "<<<",
			Self::Shell | Self::Perl | Self::Ruby => "<<",
		}
	}
}

include!(concat!(env!("OUT_DIR"), "/languages.rs"));
