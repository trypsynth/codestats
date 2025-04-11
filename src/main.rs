mod analyzer;
mod cli;
mod langs;
use crate::analyzer::CodeAnalyzer;
use anyhow::{Result, ensure};

fn main() -> Result<()> {
    let args = cli::parse_cli();
    ensure!(args.path.exists(), "Path `{}` not found", args.path.display());
    let mut analyzer = CodeAnalyzer::new(&args);
    analyzer.analyze();
    analyzer.print_stats();
    Ok(())
}
