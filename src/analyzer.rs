use crate::cli::Cli;
use ignore::WalkBuilder;
use std::collections::HashMap;

struct LangStats {
    files: u64,
    lines: u64,
    size: f64,
}

pub struct CodeAnalyzer<'a> {
    args: &'a Cli,
    total_files: u64,
    total_lines: u64,
    total_size: f64,
    lang_stats: HashMap<&'static str, LangStats>,
}

impl<'a> CodeAnalyzer<'a> {
    pub fn new(args: &'a Cli) -> Self {
        CodeAnalyzer {
            args,
            total_files: 0,
            total_lines: 0,
            total_size: 0.0,
            lang_stats: HashMap::new(),
        }
    }

    pub fn analyze(&self) {
        if self.args.verbose {
            println!("Analyzing directory {}", self.args.path.display());
        }
        let walker = WalkBuilder::new(&self.args.path)
            .follow_links(self.args.symlinks)
            .git_ignore(self.args.ignores)
            .hidden(self.args.no_hidden)
            .build();
    }
}
