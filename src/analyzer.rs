use std::path::PathBuf;

pub struct Analyzer {
    total_files: u64,
    total_lines: u64,
    total_size: f64,
    verbose: bool,
}

impl Analyzer {
    fn new(path: &PathBuf, verbose: bool) -> Self {
        Analyzer {
            total_files: 0,
            total_lines: 0,
            total_size: 0.0,
            verbose,
        }
    }
}
