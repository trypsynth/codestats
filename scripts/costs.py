#!/usr/bin/env python3
"""Measures how much work codestats does per language, to find slow languages and catch regressions.

Work is counted in user-space instructions (`perf stat -e instructions:u`), which barely vary between runs, unlike wall time.
A language's cost in a repository is the instructions for counting only that language (`-L`) minus those for a run that
counts nothing, which walks the same tree and detects the same languages.

    scripts/costs.py measure BINARY DIR... [--out FILE]   per-language costs, as a table and optionally JSON
    scripts/costs.py compare BASE CANDIDATE DIR...         where CANDIDATE does more work than BASE

Directories usually come from scripts/corpus.sh. Needs Linux perf.
"""

import argparse
import json
import subprocess
import sys
import tempfile
from collections import defaultdict
from pathlib import Path

# Languages are measured once they have this many bytes in a repository; smaller ones are lost in run-to-run variation.
MIN_BYTES = 256 * 1024
# Changes smaller than this, in instructions, are noise whatever their percentage.
NOISE = 3_000_000
# Languages that a run counting nothing can filter on, the first one absent from the repository being used.
ABSENT = ["Brainfuck", "Befunge", "Whitespace", "LOLCODE", "Malbolge"]


def run(binary, directory, *args):
	"""Runs codestats with JSON output and returns (instructions, report)."""
	with tempfile.NamedTemporaryFile("r", suffix=".perf") as stat:
		result = subprocess.run(
			["perf", "stat", "-x,", "-e", "instructions:u", "-o", stat.name, "--", binary, directory, "-o", "json-compact", *args],
			capture_output=True,
			text=True,
			check=True,
		)
		lines = [line for line in stat.read().splitlines() if "instructions" in line]
	return int(lines[0].split(",")[0]), json.loads(result.stdout)


def least(binary, directory, *args, runs=3):
	"""The fewest instructions over a few runs, since stray work (page faults, idle threads spinning) only ever adds."""
	results = [run(binary, directory, *args) for _ in range(runs)]
	return min(count for count, _ in results), results[0][1]


def measure(binary, directory, runs=3):
	"""Per-language costs in one repository: {language: {bytes, lines, files, instructions}}, plus the total and the walk."""
	total, report = least(binary, directory, runs=runs)
	present = {language["name"] for language in report["languages"]}
	absent = next(name for name in ABSENT if name not in present)
	walk, _ = least(binary, directory, "-L", absent, runs=runs)
	costs = {}
	for language in report["languages"]:
		if language["bytes"] < MIN_BYTES:
			continue
		count, _ = least(binary, directory, "-L", language["name"], runs=runs)
		costs[language["name"]] = {
			"bytes": language["bytes"],
			"lines": language["lines"]["total"],
			"files": language["file_count"],
			"instructions": max(count - walk, 0),
		}
	return {"total": total, "walk": walk, "files": report["totals"]["file_count"], "languages": costs}


def measure_all(binary, directories, runs=3):
	results = {}
	for directory in directories:
		print(f"measuring {Path(directory).name}", file=sys.stderr)
		results[Path(directory).name] = measure(binary, directory, runs)
	return results


def by_language(results):
	"""Costs pooled across repositories: {language: {bytes, lines, instructions, repos}}."""
	pooled = defaultdict(lambda: {"bytes": 0, "lines": 0, "instructions": 0, "repos": []})
	for repo, result in results.items():
		for name, cost in result["languages"].items():
			entry = pooled[name]
			entry["bytes"] += cost["bytes"]
			entry["lines"] += cost["lines"]
			entry["instructions"] += cost["instructions"]
			entry["repos"].append(repo)
	return pooled


def print_table(results):
	pooled = by_language(results)
	rows = sorted(pooled.items(), key=lambda item: -item[1]["instructions"] / item[1]["bytes"])
	per_byte = sorted(entry["instructions"] / entry["bytes"] for entry in pooled.values())
	median = per_byte[len(per_byte) // 2] if per_byte else 0
	print(f"{'language':<24} {'MB':>8} {'lines':>10} {'insn/byte':>9} {'insn/line':>9}  vs median  repos")
	for name, entry in rows:
		ratio = entry["instructions"] / entry["bytes"] / median if median else 0
		print(
			f"{name:<24} {entry['bytes'] / 1e6:8.1f} {entry['lines']:10} {entry['instructions'] / entry['bytes']:9.2f}"
			f" {entry['instructions'] / max(entry['lines'], 1):9.0f}  {ratio:8.1f}x  {', '.join(entry['repos'][:4])}"
		)
	print()
	print(f"{'repository':<24} {'files':>8} {'total insn':>14} {'walk share':>10}")
	for repo, result in results.items():
		print(f"{repo:<24} {result['files']:8} {result['total']:14} {result['walk'] / result['total']:9.0%}")


def compare(base, candidate, directories, threshold):
	"""Prints every repository and language where `candidate` does more than `threshold` more work than `base`; returns whether any did."""
	worse = False
	print(f"{'repository':<24} {'language':<24} {'base':>14} {'candidate':>14} {'change':>8}")
	for directory in directories:
		repo = Path(directory).name
		print(f"comparing {repo}", file=sys.stderr)
		old, new = measure(base, directory), measure(candidate, directory)
		pairs = [("(total)", old["total"], new["total"])]
		pairs += [(name, cost["instructions"], new["languages"].get(name, {}).get("instructions", 0)) for name, cost in old["languages"].items()]
		for name, before, after in pairs:
			change = (after - before) / before if before else 0
			flagged = change > threshold and after - before > NOISE
			worse |= flagged
			if flagged or name == "(total)":
				print(f"{repo:<24} {name:<24} {before:14} {after:14} {change:+7.1%}{'  <--' if flagged else ''}")
	return worse


def main():
	parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
	commands = parser.add_subparsers(dest="command", required=True)
	measure_parser = commands.add_parser("measure")
	measure_parser.add_argument("binary")
	measure_parser.add_argument("directories", nargs="+")
	measure_parser.add_argument("--out", help="also write the results as JSON")
	compare_parser = commands.add_parser("compare")
	compare_parser.add_argument("base")
	compare_parser.add_argument("candidate")
	compare_parser.add_argument("directories", nargs="+")
	compare_parser.add_argument("--threshold", type=float, default=0.02, help="flag changes above this fraction (default 0.02)")
	args = parser.parse_args()
	if args.command == "measure":
		results = measure_all(args.binary, args.directories)
		if args.out:
			Path(args.out).write_text(json.dumps(results, indent=1))
		print_table(results)
	else:
		sys.exit(1 if compare(args.base, args.candidate, args.directories, args.threshold) else 0)


if __name__ == "__main__":
	main()
