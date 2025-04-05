mod analyzer;
mod cli;
mod langs;
use crate::analyzer::CodeAnalyzer;
use anyhow::{Result, bail};

fn main() -> Result<()> {
    let args = cli::parse_cli();
    if !args.path.exists() {
        bail!("Path `{}` not found", args.path.display());
    }
    let mut analyzer = CodeAnalyzer::new(&args);
    analyzer.analyze();
    analyzer.print_stats();
    Ok(())
}
