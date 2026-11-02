#!/usr/bin/env bash
# Fetches the repositories codestats is checked against for speed and accuracy, each at a pinned commit.
# They're picked so that each is dominated by a different kind of file: a language, translations, documentation, data, notebooks, huge files or masses of tiny ones.
# Usage: scripts/corpus.sh [clone directory]. Prints the directory of each repository, one per line.

repos=(
	# The README benchmarks (scripts/bench.sh).
	"BurntSushi/ripgrep 91f5d22ef48bde459d90a8c7dbeab416e97d0f2b"
	"cli/cli 9490d204a2ba95434153dedce99898ce9f281426"
	"nvaccess/nvda 0041ab653318c871dc132716215de4989e24c3fe"
	"wxWidgets/wxWidgets 73d90e3fd20f92c7b5c7cf04cc1e0567913e53ce"
	"python/cpython ef1fce0a69a19360643fb82ea3545ede21495c73"
	"kubernetes/kubernetes a972b672c7612c4892557f3d6d2a952c741c5d1f"
	"rust-lang/rust 92b5b8c87c61b20adc9001b41baf587634a74734"
	"torvalds/linux d89e1d186ec9271bf78e62771779e9fc03bf0fc3"
	# Languages.
	"spring-projects/spring-framework ed4e0f896bc01279196a2c370fbfd15c7b201eed"
	"dotnet/roslyn eb07b178c241fad780d6fc947512f00bbdbd1dc4"
	"WordPress/WordPress f73030e78d1af6c13f70a8825d0484d49230197a"
	"rails/rails e77813d4e8d794c65d43956c11b20c733e9dec52"
	"ghc/ghc 341223c0be3eed72a5ff41593aa11e5051ea10df"
	"apple/swift-nio 2f58d8ef84438d6517fc78bcea2f8527e3c72936"
	"elixir-lang/elixir 9bdeaa5e3b3b15bd83cd3863ad3f9b12595470bb"
	"scala/scala 278cab10408742a464413c445957658bb226527b"
	"neovim/neovim 77443675ef1b4569cab1f383f3a87e97eaba30d9"
	"ocaml/ocaml a76fb1de949d320f35ce29d84d8181a7e2541f22"
	"erlang/otp 5555eb73b3a233a04da729e91f86780d033e5519"
	"vuejs/core 68f25a4a1957710de71731702bd42e05535f494b"
	"twbs/bootstrap e93ba5e5267ac2f74d5d35f1c11bb476957de94b"
	"flutter/flutter 091f11f0d976b4cb8f7c19ee5e2d3f110a8167ac"
	"Perl/perl5 ad000635d3a1cfde6a725553f3acb9cee7ee3455"
	"JuliaLang/julia 3303b17029cb7f9598285f230bbb30b7a6559650"
	"Reference-LAPACK/lapack 472b3a0fcbc5e90bf30a41b13ead24e40fff54ce"
	"tidyverse/ggplot2 b638d654c8189db82a62307fefa84b1da1819da5"
	"ziglang/zig 738d2be9d6b6ef3ff3559130c05159ef53336224"
	# Huge source files.
	"sqlite/sqlite 9b770d1d2e26781021cd7dc23d64361b693302b1"
	# Windows line endings and UTF-16.
	"microsoft/terminal 703ef7f969d90d3f7e132b9a5f51a8815e77d586"
	# Documentation, data and notebooks.
	"mdn/content 042ac54f5d3b4e26ec910cce0cc5916d492bd7e4"
	"github/rest-api-description f9ed74230d54847b5ca00a1343e8e349e858b8fb"
	"fastai/fastbook 971f0289c3d0bda81868d3c028a832c6ee380b58"
	# Masses of tiny files.
	"NixOS/nixpkgs 5c832fb79c4a3232a2ffae83ff915ea9355ad15a"
)
root=$(cd "$(dirname "$0")/.." && pwd)
dir=$(realpath -m "${1:-$root/target/bench-repos}")
mkdir -p "$dir"
for entry in "${repos[@]}"; do
	read -r repo sha <<<"$entry"
	name=${repo#*/}
	if [[ ! -d $dir/$name ]]; then
		echo "Fetching $repo at ${sha:0:12}" >&2
		git init -q "$dir/$name" && git -C "$dir/$name" fetch -q --depth 1 "https://github.com/$repo" "$sha" &&
			git -C "$dir/$name" checkout -q FETCH_HEAD || exit 1
	fi
	echo "$dir/$name"
done
