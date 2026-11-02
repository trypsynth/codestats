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
		// Generated into memory and then written, since clap_complete panics when a write fails, as it does when the reader closes the pipe (`cs completions bash | head`).
		let mut script = Vec::new();
		match self {
			Self::Bash => generate(shells::Bash, &mut cmd, bin_name, &mut script),
			Self::Zsh => generate(shells::Zsh, &mut cmd, bin_name, &mut script),
			Self::Fish => generate(shells::Fish, &mut cmd, bin_name, &mut script),
			Self::Powershell => generate(shells::PowerShell, &mut cmd, bin_name, &mut script),
			Self::Elvish => generate(shells::Elvish, &mut cmd, bin_name, &mut script),
		}
		let mut stdout = io::stdout();
		stdout.write_all(&script)?;
		stdout.flush()?;
		Ok(())
	}
}
