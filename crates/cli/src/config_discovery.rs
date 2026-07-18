//! Locates a `.codestats.toml`/`config.toml` on the local filesystem.
//!
//! This is deliberately kept out of the `codestats` lib: "search the user's
//! home directory and XDG dirs for a dotfile" only makes sense for a desktop
//! CLI, not a library that might run in a browser or against an uploaded zip.

use std::{env, path::PathBuf};

use anyhow::Result;
use codestats::config::Config;

/// Get the user's home directory.
fn home_dir() -> Option<PathBuf> {
	env::var_os("HOME").map(PathBuf::from)
}

/// Get the platform-specific config directory.
fn config_dir() -> Option<PathBuf> {
	#[cfg(target_os = "linux")]
	{
		env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| home_dir().map(|h| h.join(".config")))
	}
	#[cfg(target_os = "macos")]
	{
		home_dir().map(|h| h.join("Library/Application Support"))
	}
	#[cfg(target_os = "windows")]
	{
		env::var_os("APPDATA").map(PathBuf::from)
	}
	#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
	{
		home_dir().map(|h| h.join(".config"))
	}
}

/// Search the current directory, XDG/platform config dir, and home directory for a config file.
#[must_use]
pub fn find_config_file() -> Option<PathBuf> {
	[PathBuf::from(".codestats.toml"), PathBuf::from("codestats.toml")]
		.into_iter()
		.chain(config_dir().map(|d| d.join("codestats").join("config.toml")))
		.chain(home_dir().map(|h| h.join(".codestats.toml")))
		.find(|path| path.is_file())
}

/// Load the config found by [`find_config_file`], or `Config::default()` if none exists.
///
/// # Errors
///
/// Returns an error if a config file is found but cannot be read or parsed.
pub fn load_default() -> Result<Config> {
	find_config_file().map_or_else(|| Ok(Config::default()), Config::from_file)
}
