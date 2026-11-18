//! Jupyter notebooks, which are JSON: code cells count as code in the kernel's language, and as that language's embedded code; Markdown and raw cells count as comment; outputs aren't counted.

use serde::Deserialize;

use super::{
	embedded::Classifier,
	line_classifier::{LineType, is_blank},
	stats::{Embedded, FileCounts, LineCounts},
};
use crate::langs::{Language, language_named};

/// A notebook: nbformat 4 keeps its cells at the top, and nbformat 3 in worksheets.
#[derive(Deserialize)]
struct Notebook {
	#[serde(default)]
	cells: Vec<Cell>,
	#[serde(default)]
	worksheets: Vec<Worksheet>,
	#[serde(default)]
	metadata: Metadata,
}

#[derive(Deserialize)]
struct Worksheet {
	#[serde(default)]
	cells: Vec<Cell>,
}

#[derive(Deserialize, Default)]
struct Metadata {
	kernelspec: Option<Kernelspec>,
	language_info: Option<Named>,
	/// nbformat 3's, which some tools wrote.
	language: Option<String>,
}

#[derive(Deserialize)]
struct Kernelspec {
	language: Option<String>,
	/// Such as `python3`, for a kernel that doesn't give its language.
	name: Option<String>,
}

#[derive(Deserialize)]
struct Named {
	name: Option<String>,
}

#[derive(Deserialize)]
struct Cell {
	#[serde(default, rename = "cell_type")]
	kind: String,
	#[serde(default)]
	source: Source,
	/// nbformat 3 keeps a code cell's source here, and its language in `language`.
	#[serde(default)]
	input: Source,
	language: Option<String>,
	#[serde(default)]
	metadata: CellMetadata,
}

#[derive(Deserialize, Default)]
struct CellMetadata {
	/// VS Code's notebooks give each cell a language, though a Polyglot notebook's is `polyglot-notebook` for all of them.
	vscode: Option<VsCode>,
	/// Polyglot and .NET Interactive notebooks name each cell's kernel.
	polyglot_notebook: Option<Polyglot>,
	dotnet_interactive: Option<DotnetInteractive>,
}

#[derive(Deserialize)]
struct Polyglot {
	#[serde(rename = "kernelName")]
	kernel_name: Option<String>,
}

#[derive(Deserialize)]
struct DotnetInteractive {
	language: Option<String>,
}

#[derive(Deserialize)]
struct VsCode {
	#[serde(rename = "languageId")]
	language_id: Option<String>,
}

/// A cell's text, as one string or as its lines, each ending with its newline.
#[derive(Deserialize)]
#[serde(untagged)]
enum Source {
	Text(String),
	Lines(Vec<String>),
}

impl Default for Source {
	fn default() -> Self {
		Self::Lines(Vec::new())
	}
}

impl Source {
	fn text(&self) -> String {
		match self {
			Self::Text(text) => text.clone(),
			Self::Lines(lines) => lines.concat(),
		}
	}
}

/// The language a kernel, cell, or cell magic names, by the names Jupyter uses.
fn language_for(name: &str) -> Option<&'static Language> {
	let name = name.trim();
	let canonical = match name.to_ascii_lowercase().as_str() {
		"python2" | "python3" | "ipython" | "ipython3" | "pyspark" => "Python",
		"c++" | "cpp" | "c++11" | "c++14" | "c++17" | "c++20" | "xcpp" => "C++",
		"csharp" | "c#" => "C#",
		"fsharp" | "f#" => "F#",
		"sh" | "shell" | "bash" | "zsh" => "Bash",
		"js" | "javascript" | "node" | "nodejs" => "JavaScript",
		"ts" | "typescript" => "TypeScript",
		"latex" | "tex" => "TeX/LaTeX",
		"wolfram language" | "wolfram" | "mathematica" => "Mathematica/Wolfram",
		"powershell" | "pwsh" => "PowerShell",
		"sage" | "sagemath" => "Python",
		"common-lisp" | "sbcl" | "lisp" => "Common Lisp",
		"sparql" => "SPARQL",
		_ => name,
	};
	language_named(canonical).filter(|language| language.embedding.is_none() || language.name == "HTML")
}

/// The language a kernel's name stands for, such as `python3`, `ir`, `julia-1.10`, or `xcpp17`, for a kernel that doesn't give its language.
fn language_for_kernel(name: &str) -> Option<&'static Language> {
	let name = name.split('-').next().unwrap_or(name).trim_end_matches(|c: char| c.is_ascii_digit() || c == '.');
	match name {
		"ir" => language_named("R"),
		"ijulia" => language_named("Julia"),
		_ => language_for(name),
	}
}

impl Notebook {
	/// The kernel's language, which code cells are written in unless they say otherwise.
	fn kernel_language(&self) -> Option<&'static Language> {
		let metadata = &self.metadata;
		let kernelspec = metadata.kernelspec.as_ref();
		[
			kernelspec.and_then(|kernel| kernel.language.as_deref()),
			metadata.language_info.as_ref().and_then(|info| info.name.as_deref()),
			metadata.language.as_deref(),
		]
		.into_iter()
		.flatten()
		.find_map(language_for)
		.or_else(|| kernelspec.and_then(|kernel| kernel.name.as_deref()).and_then(language_for_kernel))
		// A notebook that names no kernel runs in Jupyter's default, Python.
		.or_else(|| kernelspec.is_none().then(|| language_named("Python")).flatten())
	}
}

/// Count a notebook's cells, or `None` if `bytes` isn't a notebook's JSON.
pub(super) fn count(bytes: &[u8]) -> Option<FileCounts> {
	let notebook: Notebook = serde_json::from_slice(bytes).ok()?;
	let kernel = notebook.kernel_language();
	let mut lines = LineCounts::default();
	let mut embedded = Embedded::default();
	for cell in notebook.cells.iter().chain(notebook.worksheets.iter().flat_map(|worksheet| &worksheet.cells)) {
		if cell.kind == "code" {
			let source = if matches!(&cell.source, Source::Lines(found) if found.is_empty()) {
				&cell.input
			} else {
				&cell.source
			};
			let metadata = &cell.metadata;
			let cell_language = [
				cell.language.as_deref(),
				metadata.polyglot_notebook.as_ref().and_then(|polyglot| polyglot.kernel_name.as_deref()),
				metadata.dotnet_interactive.as_ref().and_then(|dotnet| dotnet.language.as_deref()),
				metadata.vscode.as_ref().and_then(|vscode| vscode.language_id.as_deref()),
			]
			.into_iter()
			.flatten()
			.find_map(language_for)
			.or(kernel);
			count_code(&source.text(), cell_language, &mut lines, &mut embedded);
		} else {
			// Markdown, raw, and nbformat 3's heading cells are the prose around the code.
			for line in cell.source.text().lines() {
				lines.add(if is_blank(line.as_bytes()) { LineType::Blank } else { LineType::Comment });
			}
		}
	}
	Some(FileCounts { lines, embedded })
}

/// Count a code cell written in `language`, or in the language its cell magic, such as `%%bash`, names.
fn count_code(text: &str, language: Option<&'static Language>, lines: &mut LineCounts, embedded: &mut Embedded) {
	let mut rest = text;
	let mut language = language;
	// Jupyter's `%%bash` and Polyglot's `#!fsharp` name the cell's language on its first line.
	if let Some(magic) = text.strip_prefix("%%").or_else(|| text.strip_prefix("#!")) {
		let (line, after) = magic.split_once('\n').unwrap_or((magic, ""));
		let name = line.split_whitespace().next().unwrap_or_default();
		if name == "markdown" {
			lines.add(LineType::Code);
			for line in after.lines() {
				lines.add(if is_blank(line.as_bytes()) { LineType::Blank } else { LineType::Comment });
			}
			return;
		}
		if let Some(named) = language_for(name) {
			lines.add(LineType::Code);
			rest = after;
			language = Some(named);
		}
	}
	let Some(language) = language else {
		for line in rest.lines() {
			lines.add(if is_blank(line.as_bytes()) { LineType::Blank } else { LineType::Code });
		}
		return;
	};
	// Each cell starts afresh: a comment or string left open doesn't run into the next one.
	let mut classifier = Classifier::new(language);
	for line in rest.lines() {
		let line_type = classifier.classify(line, false);
		lines.add(line_type);
		embedded.add(language, line_type);
	}
	embedded.merge(&classifier.into_embedded());
}

#[cfg(test)]
mod tests {
	use super::*;

	fn counted(json: &str) -> (LineCounts, Vec<(&'static str, LineCounts)>) {
		let counts = count(json.as_bytes()).unwrap();
		(
			counts.lines,
			counts.embedded.languages().into_iter().map(|(language, lines)| (language.name, lines)).collect(),
		)
	}

	const fn lines(code: u64, comment: u64, blank: u64) -> LineCounts {
		LineCounts { total: code + comment + blank, code, comment, blank, shebang: 0 }
	}

	#[test]
	fn code_cells_count_in_the_kernels_language_and_prose_as_comment() {
		let json = r##"{
			"metadata": {"kernelspec": {"language": "python", "name": "python3"}},
			"nbformat": 4,
			"cells": [
				{"cell_type": "markdown", "metadata": {}, "source": ["# Title\n", "\n", "Some prose."]},
				{"cell_type": "code", "metadata": {}, "outputs": [{"output_type": "stream", "text": ["a\n", "b\n"]}],
				 "source": ["# a comment\n", "x = 1\n", "\n", "print(x)"]},
				{"cell_type": "raw", "metadata": {}, "source": "\\LaTeX"}
			]
		}"##;
		let (all, embedded) = counted(json);
		assert_eq!(all, lines(2, 4, 2));
		assert_eq!(embedded, [("Python", lines(2, 1, 1))]);
	}

	#[test]
	fn an_unclosed_string_ends_with_its_cell() {
		let json = r##"{"metadata": {"kernelspec": {"language": "python"}}, "cells": [
			{"cell_type": "code", "source": "s = \"\"\"\nstill a string"},
			{"cell_type": "code", "source": "# a comment"}
		]}"##;
		assert_eq!(counted(json).0, lines(2, 1, 0));
	}

	#[test]
	fn a_cell_magic_names_its_cells_language() {
		let json = r##"{"metadata": {"kernelspec": {"language": "python"}}, "cells": [
			{"cell_type": "code", "source": ["%%bash\n", "# a shell comment\n", "ls"]},
			{"cell_type": "code", "source": ["%%timeit\n", "f()"]}
		]}"##;
		let (all, embedded) = counted(json);
		assert_eq!(all, lines(4, 1, 0));
		assert_eq!(embedded, [("Bash", lines(1, 1, 0)), ("Python", lines(2, 0, 0))]);
	}

	#[test]
	fn polyglot_cells_name_their_own_kernel() {
		let json = r##"{"metadata": {"kernelspec": {"language": "C#", "name": ".net-csharp"}}, "cells": [
			{"cell_type": "code", "metadata": {"polyglot_notebook": {"kernelName": "fsharp"}, "vscode": {"languageId": "polyglot-notebook"}},
			 "source": ["(* fsharp block\n", "comment *)\n", "let x = 1"]},
			{"cell_type": "code", "metadata": {}, "source": ["#!pwsh\n", "# a comment\n", "Get-Item ."]},
			{"cell_type": "code", "metadata": {}, "source": ["#!markdown\n", "Some prose."]}
		]}"##;
		let (all, embedded) = counted(json);
		assert_eq!(all, lines(4, 4, 0));
		assert_eq!(embedded, [("F#", lines(1, 2, 0)), ("PowerShell", lines(1, 1, 0))]);
	}

	#[test]
	fn a_notebook_without_a_kernel_is_python() {
		let json = r##"{"metadata": {}, "cells": [{"cell_type": "code", "source": "# c\nx = 1"}]}"##;
		assert_eq!(counted(json).1, [("Python", lines(1, 1, 0))]);
	}

	#[test]
	fn nbformat_3_keeps_cells_in_worksheets_and_code_in_input() {
		let json = r##"{"metadata": {}, "nbformat": 3, "worksheets": [{"cells": [
			{"cell_type": "heading", "level": 1, "source": ["Title"]},
			{"cell_type": "code", "language": "python", "input": ["# c\n", "x = 1"], "outputs": []}
		]}]}"##;
		let (all, embedded) = counted(json);
		assert_eq!(all, lines(1, 2, 0));
		assert_eq!(embedded, [("Python", lines(1, 1, 0))]);
	}

	#[test]
	fn kernels_are_known_by_their_jupyter_names() {
		for (name, language) in [("python3", "Python"), ("ir", "R"), ("julia-1.10", "Julia"), ("xcpp17", "C++")] {
			let json = format!(r#"{{"metadata": {{"kernelspec": {{"name": "{name}"}}}}, "cells": []}}"#);
			let found =
				serde_json::from_str::<Notebook>(&json).unwrap().kernel_language().map(|language| language.name);
			assert_eq!(found, Some(language), "{name}");
		}
		let unknown = r#"{"metadata": {"kernelspec": {"language": "no such language"}}, "cells": [
			{"cell_type": "code", "source": "a\n\nb"}
		]}"#;
		assert_eq!(counted(unknown), (lines(2, 0, 1), Vec::new()));
		assert!(count(b"not json").is_none());
	}
}
