/// Glob patterns for well-known generated files excluded by default.
/// Pass `--include-generated` to count these files.
pub const PATTERNS: &[&str] = &[
	// Lockfiles
	"Cargo.lock",
	"package-lock.json",
	"yarn.lock",
	"pnpm-lock.yaml",
	"bun.lockb",
	"deno.lock",
	"go.sum",
	"Gemfile.lock",
	"composer.lock",
	"poetry.lock",
	"Pipfile.lock",
	"mix.lock",
	"pubspec.lock",
	"flake.lock",
	"Podfile.lock",
	"packages.lock.json",
	// Minified assets
	"*.min.js",
	"*.min.mjs",
	"*.min.css",
];
