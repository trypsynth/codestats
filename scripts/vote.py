#!/usr/bin/env python3
"""Finds files where codestats probably miscounts: other counters agree with each other, but not with codestats.

Each reference counter (tokei, scc and cloc, whichever are installed) counts every file. When at least two of them
agree on a file's code, comment and blank lines and codestats differs, the file is flagged. Two independent tools
agreeing makes a codestats bug likely, though they can share a mistake, so flagged files still need a look.

Differences that are codestats' conventions rather than mistakes are left out: shebang lines are folded into comments,
and whitespace-only lines inside comments and strings, which codestats counts as blank, are allowed for.

    scripts/vote.py CODESTATS DIR... [--cloc PATH] [--examples FILE]

Prints the flagged files per language, most first, and optionally writes examples as JSON.
"""

import argparse
import json
import os
import shutil
import subprocess
from collections import defaultdict


def codestats(binary, repo):
	report = json.loads(subprocess.run([binary, repo, "-v", "-o", "json-compact"], capture_output=True, check=True).stdout)
	files = {}
	for language in report["languages"]:
		for file in language.get("files") or []:
			lines = file["lines"]
			files[os.path.join(repo, file["path"])] = (language["name"], (lines["code"], lines["comment"] + lines["shebang"], lines["blank"]))
	return files


def tokei(repo):
	files = {}
	for language, data in json.loads(subprocess.run(["tokei", repo, "-o", "json"], capture_output=True, check=True).stdout).items():
		if language == "Total":
			continue
		for report in data["reports"]:
			stats = report["stats"]
			code, comment, blank = stats["code"], stats["comments"], stats["blanks"]
			# tokei reports embedded code separately; fold it back in.
			for child in stats.get("blobs", {}).values():
				code, comment, blank = code + child["code"], comment + child["comments"], blank + child["blanks"]
			files[os.path.realpath(report["name"])] = (code, comment, blank)
	return files


def scc(repo):
	output = subprocess.run(["scc", "--by-file", "-f", "json", "--no-min-gen", repo], capture_output=True, check=True).stdout
	return {os.path.realpath(file["Location"]): (file["Code"], file["Comment"], file["Blank"]) for language in json.loads(output) for file in language["Files"]}


def cloc(binary, repo):
	output = subprocess.run([binary, "--by-file", "--json", "--quiet", "--skip-uniqueness", repo], capture_output=True, check=True).stdout
	return {
		os.path.realpath(path): (counts["code"], counts["comment"], counts["blank"])
		for path, counts in json.loads(output).items()
		if path not in ("header", "SUM")
	}


def only_blank_convention(ours, theirs):
	"""Whether `ours` differs from `theirs` only by counting some of their comment or code lines as blank."""
	moved = ours[2] - theirs[2]
	return moved > 0 and ours[0] <= theirs[0] and ours[1] <= theirs[1] and (theirs[0] - ours[0]) + (theirs[1] - ours[1]) == moved


def main():
	parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
	parser.add_argument("codestats")
	parser.add_argument("directories", nargs="+")
	parser.add_argument("--cloc", default=shutil.which("cloc"), help="path to cloc, if it isn't on PATH")
	parser.add_argument("--examples", help="write up to 30 flagged files per language as JSON")
	args = parser.parse_args()
	references = {}
	if shutil.which("tokei"):
		references["tokei"] = tokei
	if shutil.which("scc"):
		references["scc"] = scc
	if args.cloc:
		references["cloc"] = lambda repo: cloc(args.cloc, repo)
	flagged = defaultdict(list)
	agreements = defaultdict(int)
	for directory in args.directories:
		repo = os.path.realpath(directory)
		ours = codestats(args.codestats, repo)
		theirs = {name: count(repo) for name, count in references.items()}
		for path, (language, counts) in ours.items():
			answers = {name: files[path] for name, files in theirs.items() if path in files}
			votes = defaultdict(list)
			for name, answer in answers.items():
				votes[answer].append(name)
			majority, voters = max(votes.items(), key=lambda item: len(item[1]), default=(None, []))
			if len(voters) < 2 or counts == majority or only_blank_convention(counts, majority):
				continue
			flagged[language].append({"path": path, "codestats": counts, "others": majority, "agreeing": voters})
			agreements["+".join(sorted(voters))] += 1
	print(f"{'language':<24} {'files':>6} {'code':>9} {'comment':>9} {'blank':>8}")
	for language, files in sorted(flagged.items(), key=lambda item: -len(item[1])):
		code, comment, blank = (sum(file["codestats"][i] - file["others"][i] for file in files) for i in range(3))
		print(f"{language:<24} {len(files):6} {code:+9} {comment:+9} {blank:+8}")
	print("\nagreeing references: " + ", ".join(f"{names} {count}" for names, count in sorted(agreements.items(), key=lambda item: -item[1])))
	if args.examples:
		with open(args.examples, "w") as out:
			json.dump({language: files[:30] for language, files in flagged.items()}, out, indent=1)


if __name__ == "__main__":
	main()
