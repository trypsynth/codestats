use std::{collections::HashMap, path::PathBuf};

struct LangStats {
    files: u64,
    lines: u64,
    size: f64,
}

pub struct Analyzer {
    total_files: u64,
    total_lines: u64,
    total_size: f64,
    verbose: bool,
    lang_stats: HashMap<&'static str, LangStats>,
}

impl Analyzer {
    fn new(path: &PathBuf, verbose: bool) -> Self {
        Analyzer {
            total_files: 0,
            total_lines: 0,
            total_size: 0.0,
            verbose,
            lang_stats: HashMap::new(),
        }
    }
}
