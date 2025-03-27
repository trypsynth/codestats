mod analyzer;
mod cli;
mod langs;
use anyhow::Result;

fn main() -> Result<()> {
    let args = cli::parse_cli();
    let path = args.path;
    if !path.exists() {
        return Err(anyhow::anyhow!("Path {} not found", path.display()));
    }
    let verbose = args.verbose;
    if verbose {
        println!("Analyzing directory {}", path.display());
    }
    Ok(())
}
