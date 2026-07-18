use std::io::{self, Write};

use clap::{CommandFactory, ValueEnum};
use clap_complete::{generate, shells};

use crate::cli::Cli;

/// Shells supported for completion output.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Shell {
	Bash,
	Zsh,
	Fish,
	Powershell,
	Elvish,
}

impl Shell {
	/// Render completion scripts to stdout for the selected shell.
	pub fn generate_completions(self) -> io::Result<()> {
		let mut cmd = Cli::command();
		let bin_name = cmd.get_name().to_string();
		let mut stdout = io::stdout();
		match self {
			Self::Bash => generate(shells::Bash, &mut cmd, bin_name, &mut stdout),
			Self::Zsh => generate(shells::Zsh, &mut cmd, bin_name, &mut stdout),
			Self::Fish => generate(shells::Fish, &mut cmd, bin_name, &mut stdout),
			Self::Powershell => generate(shells::PowerShell, &mut cmd, bin_name, &mut stdout),
			Self::Elvish => generate(shells::Elvish, &mut cmd, bin_name, &mut stdout),
		}
		stdout.flush()?;
		Ok(())
	}
}
