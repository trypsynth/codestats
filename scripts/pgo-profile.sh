#!/usr/bin/env bash
# Builds codestats instrumented, counts a mix of languages with it, and writes the profile that release builds optimize with.
# Needs git, cargo, and rustup's llvm-tools. Usage: scripts/pgo-profile.sh <output .profdata> [clone directory]
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
output=$(realpath -m "$1")
clones=$(realpath -m "${2:-$root/target/pgo-corpus}")
repos=(
	"python/cpython ef1fce0a69a19360643fb82ea3545ede21495c73"
	"BurntSushi/ripgrep 91f5d22ef48bde459d90a8c7dbeab416e97d0f2b"
	"cli/cli 9490d204a2ba95434153dedce99898ce9f281426"
	"rails/rails e77813d4e8d794c65d43956c11b20c733e9dec52"
	"vuejs/core 68f25a4a1957710de71731702bd42e05535f494b"
	"neovim/neovim 77443675ef1b4569cab1f383f3a87e97eaba30d9"
	"WordPress/WordPress f73030e78d1af6c13f70a8825d0484d49230197a"
)
data=$(mktemp -d)
trap 'rm -rf "$data"' EXIT
cd "$root"
RUSTC_WRAPPER="$root/scripts/pgo-rustc.sh" RUSTFLAGS="-Cprofile-generate=$data" cargo build --release --locked --target-dir target/pgo-generate
for entry in "${repos[@]}"; do
	read -r repo commit <<<"$entry"
	dir="$clones/${repo#*/}"
	if [[ $(git -C "$dir" rev-parse HEAD 2>/dev/null) != "$commit" ]]; then
		rm -rf "$dir"
		git init -q "$dir"
		git -C "$dir" fetch -q --depth 1 "https://github.com/$repo" "$commit"
		git -C "$dir" -c advice.detachedHead=false checkout -q FETCH_HEAD
	fi
	for flags in "" "--include-generated --no-gitignore --hidden --no-gitattributes"; do
		# Too many values to track is only a warning, and the profile is still useful.
		# shellcheck disable=SC2086
		target/pgo-generate/release/cs "$dir" -o json-compact $flags >/dev/null 2> >(grep -v 'LLVM Profile Warning' >&2 || true)
	done
done
target/pgo-generate/release/cs tests/fixtures -v -o json >/dev/null 2> >(grep -v 'LLVM Profile Warning' >&2 || true)
"$(find "$(rustc --print sysroot)" -name llvm-profdata -type f | head -1)" merge -o "$output" "$data"
echo "wrote $output"
