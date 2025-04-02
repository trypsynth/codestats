mod analyzer;
mod cli;
mod langs;
use anyhow::{Result, bail};
use crate::analyzer::CodeAnalyzer;

fn main() -> Result<()> {
    let args = cli::parse_cli();
    if !args.path.exists() {
        bail!("Path `{}` not found", args.path.display());
    }
    let analyzer = CodeAnalyzer::new(&args);
    analyzer.analyze();
    Ok(())
}
