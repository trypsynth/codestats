//! Vendored code: other projects' code copied into a repository, which GitHub leaves out of its language statistics, and codestats does too unless `--include-vendored` is given.

use std::path::{Component, Path, PathBuf};

/// Files that make a `vendor` directory beside them hold copied dependencies: Go modules, Composer, Bundler, and `cargo vendor`. A `vendor` directory anywhere else, such as Linux's `arch/riscv/include/asm/vendor`, is the project's own code.
const MANIFESTS: [&[u8]; 4] = [b"go.mod", b"composer.json", b"Gemfile", b"Cargo.toml"];

/// Whether a file named `name` makes a `vendor` directory beside it vendored.
pub(super) fn is_manifest(name: &[u8]) -> bool {
	MANIFESTS.contains(&name)
}

/// Whether a directory named `name` holds vendored code. `beside_manifest` says whether its parent holds a manifest, and is only asked for a directory named `vendor`.
pub(super) fn is_vendored_dir(name: &[u8], beside_manifest: impl FnOnce() -> bool) -> bool {
	match name {
		b"node_modules" | b"bower_components" | b"Pods" | b"Carthage" => true,
		b"vendor" => beside_manifest(),
		_ => is_third_party(name),
	}
}

/// `third_party`, `third-party`, `ThirdParty`, `3rdparty`, and the other ways of writing them, as linguist matches them.
fn is_third_party(name: &[u8]) -> bool {
	let starts = |prefix: &[u8]| name.get(..prefix.len()).is_some_and(|start| start.eq_ignore_ascii_case(prefix));
	let rest = if starts(b"third") {
		&name[5..]
	} else if starts(b"3rd") {
		&name[3..]
	} else {
		return false;
	};
	let rest = rest.strip_prefix(b"_").or_else(|| rest.strip_prefix(b"-")).unwrap_or(rest);
	rest.eq_ignore_ascii_case(b"party")
}

/// Whether the file at `path` is inside a vendored directory, judging by the directories between the counted one and the file. `has_manifest` says whether a directory, as a path from the counted one, holds a manifest.
pub(super) fn is_inside_vendored(path: &Path, has_manifest: impl Fn(&Path) -> bool) -> bool {
	let Some(dir) = path.parent() else { return false };
	let mut parent = PathBuf::new();
	for component in dir.components() {
		if let Component::Normal(name) = component
			&& is_vendored_dir(name.as_encoded_bytes(), || has_manifest(&parent))
		{
			return true;
		}
		parent.push(component);
	}
	false
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;

	#[rstest]
	#[case("third_party", true)]
	#[case("third-party", true)]
	#[case("ThirdParty", true)]
	#[case("thirdparty", true)]
	#[case("3rdparty", true)]
	#[case("3rd_party", true)]
	#[case("3rdParty", true)]
	#[case("node_modules", true)]
	#[case("bower_components", true)]
	#[case("Pods", true)]
	#[case("Carthage", true)]
	#[case("third", false)]
	#[case("party", false)]
	#[case("third_party_tools", false)]
	#[case("vendors", false)]
	#[case("extern", false)]
	#[case("external", false)]
	#[case("deps", false)]
	#[case("pods", false)]
	fn vendored_directory_names(#[case] name: &str, #[case] vendored: bool) {
		assert_eq!(is_vendored_dir(name.as_bytes(), || unreachable!("only vendor asks")), vendored, "{name}");
	}

	#[test]
	fn vendor_needs_a_manifest_beside_it() {
		assert!(is_vendored_dir(b"vendor", || true));
		assert!(!is_vendored_dir(b"vendor", || false));
	}

	#[test]
	fn paths_inside_vendored_directories() {
		let manifests = [Path::new(""), Path::new("staging/api")];
		let has_manifest = |dir: &Path| manifests.contains(&dir);
		assert!(is_inside_vendored(Path::new("vendor/golang.org/x/net/http.go"), has_manifest));
		assert!(is_inside_vendored(Path::new("staging/api/vendor/lib.go"), has_manifest));
		assert!(is_inside_vendored(Path::new("lib/third_party/zlib/inflate.c"), has_manifest));
		assert!(!is_inside_vendored(Path::new("arch/riscv/include/asm/vendor/thead.h"), has_manifest));
		assert!(!is_inside_vendored(Path::new("vendor"), has_manifest));
		assert!(!is_inside_vendored(Path::new("cmd/main.go"), has_manifest));
	}
}
