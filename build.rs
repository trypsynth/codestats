//! Compiles `languages.json5` into a static Rust table of language definitions.

#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

#[path = "build/config.rs"]
mod config;
#[path = "build/render.rs"]
mod render;
#[path = "build/validate.rs"]
mod validate;

use std::{env, error::Error, fs, path::Path, result};

use indexmap::IndexMap;

use crate::{
	config::LanguageConfig,
	render::render_languages,
	validate::{normalize_languages, validate_languages},
};

type Result<T> = result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
	let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
	let json_path = Path::new(&manifest_dir).join("languages.json5");
	println!("cargo:rerun-if-changed={}", json_path.display());
	// Declaring any rerun-if-changed replaces cargo's default of watching the whole package, so the build script's own modules have to be listed too.
	println!("cargo:rerun-if-changed=build.rs");
	println!("cargo:rerun-if-changed=build");
	let json_content = fs::read_to_string(&json_path)?;
	let language_map: IndexMap<String, LanguageConfig> =
		json5::from_str(&json_content).map_err(|e| format!("Failed to parse languages.json5: {e}"))?;
	let languages = normalize_languages(language_map)?;
	validate_languages(&languages)?;
	let out_dir = env::var("OUT_DIR")?;
	fs::write(Path::new(&out_dir).join("languages.rs"), render_languages(&languages))?;
	Ok(())
}
