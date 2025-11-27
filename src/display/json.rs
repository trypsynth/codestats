use std::{io::Write, path::Path, result};

use anyhow::Result;
use serde::ser::{Serialize, SerializeMap, SerializeSeq, SerializeStruct, Serializer};
use serde_json::{Serializer as JsonSerializer, ser::PrettyFormatter};

use super::OutputFormatter;
use crate::analysis::{AnalysisResults, FileStats, LanguageStats};

/// JSON formatter
pub struct JsonFormatter;

impl OutputFormatter for JsonFormatter {
	fn write_output(
		&self,
		results: &AnalysisResults,
		path: &Path,
		verbose: bool,
		writer: &mut dyn Write,
	) -> Result<()> {
		let formatter = PrettyFormatter::with_indent(b"\t");
		let mut serializer = JsonSerializer::with_formatter(writer, formatter);
		let mut map = serializer.serialize_map(Some(3))?;
		map.serialize_entry("analysis_path", &path.display().to_string())?;
		map.serialize_entry("summary", &Summary { results })?;
		map.serialize_entry("languages", &Languages { results, verbose })?;
		SerializeMap::end(map)?;
		Ok(())
	}
}

struct Summary<'a> {
	results: &'a AnalysisResults,
}

impl Serialize for Summary<'_> {
	fn serialize<S: Serializer>(&self, serializer: S) -> result::Result<S::Ok, S::Error> {
		let r = self.results;
		let mut state = serializer.serialize_struct("Summary", 12)?;
		state.serialize_field("total_files", &r.total_files())?;
		state.serialize_field("total_lines", &r.total_lines())?;
		state.serialize_field("total_code_lines", &r.total_code_lines())?;
		state.serialize_field("total_comment_lines", &r.total_comment_lines())?;
		state.serialize_field("total_blank_lines", &r.total_blank_lines())?;
		state.serialize_field("total_shebang_lines", &r.total_shebang_lines())?;
		state.serialize_field("total_size", &r.total_size())?;
		state.serialize_field("total_size_human", &r.total_size_human())?;
		state.serialize_field("code_percentage", &r.code_percentage())?;
		state.serialize_field("comment_percentage", &r.comment_percentage())?;
		state.serialize_field("blank_percentage", &r.blank_percentage())?;
		state.serialize_field("shebang_percentage", &r.shebang_percentage())?;
		state.end()
	}
}

struct Languages<'a> {
	results: &'a AnalysisResults,
	verbose: bool,
}

impl Serialize for Languages<'_> {
	fn serialize<S: Serializer>(&self, serializer: S) -> result::Result<S::Ok, S::Error> {
		let languages = self.results.languages_by_lines();
		let mut seq = serializer.serialize_seq(Some(languages.len()))?;
		for (lang_name, lang_stats) in languages {
			seq.serialize_element(&LanguageRecord { lang_name, lang_stats, verbose: self.verbose })?;
		}
		seq.end()
	}
}

struct LanguageRecord<'a> {
	lang_name: &'a str,
	lang_stats: &'a LanguageStats,
	verbose: bool,
}

impl Serialize for LanguageRecord<'_> {
	fn serialize<S: Serializer>(&self, serializer: S) -> result::Result<S::Ok, S::Error> {
		let mut state = serializer.serialize_map(None)?;
		state.serialize_entry("name", self.lang_name)?;
		state.serialize_entry("files", &self.lang_stats.files())?;
		state.serialize_entry("lines", &self.lang_stats.lines())?;
		state.serialize_entry("code_lines", &self.lang_stats.code_lines())?;
		state.serialize_entry("comment_lines", &self.lang_stats.comment_lines())?;
		state.serialize_entry("blank_lines", &self.lang_stats.blank_lines())?;
		state.serialize_entry("shebang_lines", &self.lang_stats.shebang_lines())?;
		state.serialize_entry("size", &self.lang_stats.size())?;
		state.serialize_entry("size_human", &self.lang_stats.size_human())?;
		state.serialize_entry("code_percentage", &self.lang_stats.code_percentage())?;
		state.serialize_entry("comment_percentage", &self.lang_stats.comment_percentage())?;
		state.serialize_entry("blank_percentage", &self.lang_stats.blank_percentage())?;
		state.serialize_entry("shebang_percentage", &self.lang_stats.shebang_percentage())?;
		if self.verbose {
			state.serialize_entry("files_detail", &FilesDetail { files: self.lang_stats.files_list() })?;
		}
		state.end()
	}
}

struct FilesDetail<'a> {
	files: &'a [FileStats],
}

impl Serialize for FilesDetail<'_> {
	fn serialize<S: Serializer>(&self, serializer: S) -> result::Result<S::Ok, S::Error> {
		let mut seq = serializer.serialize_seq(Some(self.files.len()))?;
		for file in self.files {
			seq.serialize_element(&FileRecord { file })?;
		}
		seq.end()
	}
}

struct FileRecord<'a> {
	file: &'a FileStats,
}

impl Serialize for FileRecord<'_> {
	fn serialize<S: Serializer>(&self, serializer: S) -> result::Result<S::Ok, S::Error> {
		let mut state = serializer.serialize_struct("FileRecord", 8)?;
		state.serialize_field("path", &self.file.path())?;
		state.serialize_field("total_lines", &self.file.total_lines())?;
		state.serialize_field("code_lines", &self.file.code_lines())?;
		state.serialize_field("comment_lines", &self.file.comment_lines())?;
		state.serialize_field("blank_lines", &self.file.blank_lines())?;
		state.serialize_field("shebang_lines", &self.file.shebang_lines())?;
		state.serialize_field("size", &self.file.size())?;
		state.serialize_field("size_human", &self.file.size_human())?;
		state.end()
	}
}
