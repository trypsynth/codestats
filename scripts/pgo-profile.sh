#!/usr/bin/env bash
# Builds codestats instrumented, counts a mix of languages with it, and writes the profile that release builds optimize with.
# Needs git, cargo, and rustup's llvm-tools. Usage: scripts/pgo-profile.sh <output .profdata> [clone directory] [ARM]
# With "arm" as the last argument, the profile is taken from an ARM64 build run under qemu-aarch64, which also needs zig, as an x86-64 profile misses some of the hottest functions on ARM.
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
export RUSTC_WRAPPER="$root/scripts/pgo-rustc.sh" RUSTFLAGS="-Cprofile-generate=$data"
if [[ ${3:-} == arm ]]; then
	# zig's linker can't take the profiler runtime's flags, so zig only compiles the C code here.
	export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=rust-lld
	cat >"$data/cc" <<'CC'
#!/usr/bin/env bash
args=()
for arg in "$@"; do [[ $arg == --target=* ]] || args+=("$arg"); done
exec zig cc -target aarch64-linux-musl "${args[@]}"
CC
	chmod +x "$data/cc"
	export CC_aarch64_unknown_linux_musl="$data/cc" AR_aarch64_unknown_linux_musl="zig ar"
	cargo build --release --locked --target aarch64-unknown-linux-musl --target-dir target/pgo-generate-arm
	run=(qemu-aarch64 target/pgo-generate-arm/aarch64-unknown-linux-musl/release/cs)
else
	cargo build --release --locked --target-dir target/pgo-generate
	run=(target/pgo-generate/release/cs)
fi
unset RUSTC_WRAPPER RUSTFLAGS
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
		"${run[@]}" "$dir" -o json-compact $flags >/dev/null 2> >(grep -v 'LLVM Profile Warning' >&2 || true)
	done
done
"${run[@]}" tests/fixtures -v -o json >/dev/null 2> >(grep -v 'LLVM Profile Warning' >&2 || true)
"$(find "$(rustc --print sysroot)" -name llvm-profdata -type f | head -1)" merge -o "$output" "$data"/*.profraw
echo "wrote $output"
