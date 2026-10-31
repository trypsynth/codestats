#!/usr/bin/env bash
# Benchmarks codestats against tokei, scc and gocloc on pinned commits of public repositories and prints the README tables.
# Needs git, cargo, hyperfine, jq, tokei, scc and gocloc. Usage: scripts/bench.sh [clone directory]

repos=(
	"BurntSushi/ripgrep 91f5d22ef48bde459d90a8c7dbeab416e97d0f2b"
	"cli/cli 9490d204a2ba95434153dedce99898ce9f281426"
	"nvaccess/nvda 0041ab653318c871dc132716215de4989e24c3fe"
	"wxWidgets/wxWidgets 73d90e3fd20f92c7b5c7cf04cc1e0567913e53ce"
	"python/cpython ef1fce0a69a19360643fb82ea3545ede21495c73"
	"kubernetes/kubernetes a972b672c7612c4892557f3d6d2a952c741c5d1f"
	"rust-lang/rust 92b5b8c87c61b20adc9001b41baf587634a74734"
	"torvalds/linux d89e1d186ec9271bf78e62771779e9fc03bf0fc3"
)
tools=(codestats tokei scc gocloc)
root=$(cd "$(dirname "$0")/.." && pwd)
dir=$(realpath -m "${1:-$root/target/bench-repos}")
mkdir -p "$dir/results"
cargo build --release --quiet --manifest-path "$root/Cargo.toml" || exit 1
cs=$root/target/release/cs

for entry in "${repos[@]}"; do
	read -r repo sha <<<"$entry"
	name=${repo#*/}
	if [[ ! -d $dir/$name ]]; then
		echo "Fetching $repo at ${sha:0:12}" >&2
		# Fetching just the pinned commit keeps the rust and linux checkouts to a fraction of a full clone.
		git init -q "$dir/$name" && git -C "$dir/$name" fetch -q --depth 1 "https://github.com/$repo" "$sha" &&
			git -C "$dir/$name" checkout -q FETCH_HEAD || exit 1
	fi
	runs=10
	[[ $name == linux || $name == rust ]] && runs=5
	echo "Benchmarking $name" >&2
	args=()
	for tool in "${tools[@]}"; do
		bin=$tool
		[[ $tool == codestats ]] && bin=$cs
		args+=(-n "$tool" "$bin $dir/$name")
	done
	hyperfine -N --warmup 2 --runs "$runs" --export-json "$dir/results/$name.json" "${args[@]}" >/dev/null || exit 1
	"$cs" "$dir/$name" -q -o json-compact | jq -c .summary >"$dir/results/$name.summary.json"
done

table() {
	local metric=$1 heading=$2
	echo "$heading"
	echo
	if [[ $metric == time ]]; then
		printf '| Repository | Files | Lines |'
	else
		printf '| Repository |'
	fi
	printf ' %s |' "${tools[@]}"
	echo
	printf '| --- |'
	[[ $metric == time ]] && printf ' ---: | ---: |'
	printf ' ---: |%.0s' "${tools[@]}"
	echo
	for entry in "${repos[@]}"; do
		read -r repo _ <<<"$entry"
		name=${repo#*/}
		jq -r --arg metric "$metric" --arg repo "$repo" --slurpfile summary "$dir/results/$name.summary.json" '
			def fixed($places): (. * pow(10; $places) | round) as $n | pow(10; $places) as $scale
				| if $places == 0 then "\($n)" else "\($n / $scale | floor).\("\($n % $scale)" | ("0" * ($places - length)) + .)" end;
			def seconds: if . >= 10 then "\(fixed(1)) s" elif . >= 1 then "\(fixed(2)) s"
				elif . >= 0.01 then "\(. * 1000 | fixed(0)) ms" else "\(. * 1000 | fixed(1)) ms" end;
			def megabytes: . / 1e6 | if . < 10 then "\(fixed(1)) MB" else "\(fixed(0)) MB" end;
			def lines: if . >= 1e7 then "\(. / 1e6 | fixed(0))M" elif . >= 1e6 then "\(. / 1e6 | fixed(1))M" else "\(. / 1e3 | fixed(0))K" end;
			def grouped: tostring | until(test("^\\d{4}"; "") | not; sub("(?<a>\\d)(?<b>\\d{3})(?<c>(,\\d{3})*)$"; "\(.a),\(.b)\(.c)"));
			[.results[] | [.measurements[] | if $metric == "time" then .time_wall_clock.value else .memory_peak_resident.value end] | add / length] as $values
			| ($values | min) as $best
			| [$values[] | (if $metric == "time" then seconds else megabytes end) as $cell | if . == $best then "**\($cell)**" else $cell end] as $cells
			| ($repo | split("/")[1]) as $name
			| "| [\($name)](https://github.com/\($repo)) |"
				+ (if $metric == "time" then " \($summary[0].total_files | grouped) | \($summary[0].total_lines | lines) |" else "" end)
				+ ($cells | map(" \(.) |") | join(""))
		' "$dir/results/$name.json"
	done
}

table time "Mean wall time:"
echo
table memory "Peak memory:"
