#!/usr/bin/env bash
# Benchmarks codestats against mezura, tokei, scc, and gocloc on pinned commits of public repositories and prints the README tables.
# Needs git, hyperfine, jq, and the other tools on PATH. Usage: CS=<release binary> scripts/bench.sh [clone directory]
# Without CS, it builds the binary with cargo, which leaves out the release's PGO and so isn't what users download.

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
tools=(codestats mezura tokei scc gocloc)
root=$(cd "$(dirname "$0")/.." && pwd)
dir=$(realpath -m "${1:-$root/target/bench-repos}")
mkdir -p "$dir/results"
cs=${CS:-}
if [[ -z $cs ]]; then
	cargo build --release --quiet --manifest-path "$root/Cargo.toml" || exit 1
	cs=$root/target/release/cs
fi

for entry in "${repos[@]}"; do
	read -r repo sha <<<"$entry"
	name=${repo#*/}
	if [[ ! -d $dir/$name ]]; then
		echo "Fetching $repo at ${sha:0:12}" >&2
		# Fetching just the pinned commit keeps the rust and linux checkouts to a fraction of a full clone.
		git init -q "$dir/$name" && git -C "$dir/$name" fetch -q --depth 1 "https://github.com/$repo" "$sha" &&
			git -C "$dir/$name" checkout -q FETCH_HEAD || exit 1
	fi
	rounds=10
	[[ $name == linux || $name == rust ]] && rounds=5
	echo "Benchmarking $name" >&2
	# A processor slows down as it heats up, which favors whichever tool runs first in a long block of runs. Short rounds of two runs per tool, in an order that flips each round, with a pause between rounds, spread that evenly.
	forward=()
	backward=()
	for tool in "${tools[@]}"; do
		bin=$tool
		[[ $tool == codestats ]] && bin=$cs
		forward+=(-n "$tool" "$bin $dir/$name")
		backward=(-n "$tool" "$bin $dir/$name" "${backward[@]}")
	done
	rm -f "$dir/results/$name".round-*.json
	for ((round = 0; round < rounds; round++)); do
		order=("${forward[@]}")
		((round % 2)) && order=("${backward[@]}")
		warmup=0
		((round == 0)) && warmup=4
		hyperfine -N --warmup "$warmup" --runs 2 --export-json "$dir/results/$name.round-$round.json" "${order[@]}" >/dev/null || exit 1
		sleep 2
	done
	jq -s --arg tools "${tools[*]}" '[.[].results[]] as $all | {results: [$tools | split(" ")[] as $tool | {measurements: [$all[] | select(.name == $tool) | .measurements[]]}]}' \
		"$dir/results/$name".round-*.json >"$dir/results/$name.json" || exit 1
	"$cs" "$dir/$name" -q -o json-compact | jq -c .totals >"$dir/results/$name.summary.json"
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
			def median: sort | length as $n | if $n % 2 == 1 then .[$n / 2 | floor] else (.[$n / 2 - 1] + .[$n / 2]) / 2 end;
			[.results[] | [.measurements[] | if $metric == "time" then .time_wall_clock.value else .memory_peak_resident.value end] | median] as $values
			| ($values | min) as $best
			| [$values[] | (if $metric == "time" then seconds else megabytes end) as $cell | if . == $best then "**\($cell)**" else $cell end] as $cells
			| ($repo | split("/")[1]) as $name
			| "| [\($name)](https://github.com/\($repo)) |"
				+ (if $metric == "time" then " \($summary[0].file_count | grouped) | \($summary[0].lines.total | lines) |" else "" end)
				+ ($cells | map(" \(.) |") | join(""))
		' "$dir/results/$name.json"
	done
}

table time "Median wall time:"
echo
table memory "Median peak memory:"
