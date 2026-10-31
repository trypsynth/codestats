/// How a language embeds code in others, whose regions are counted by those languages' rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Embedding {
	/// `<script>` and `<style>` elements.
	Html,
	/// HTML's elements, and a TypeScript frontmatter between `---` lines.
	Astro,
	/// Fenced code blocks, and YAML (`---`) or TOML (`+++`) frontmatter.
	Markdown,
}

include!(concat!(env!("OUT_DIR"), "/languages.rs"));
