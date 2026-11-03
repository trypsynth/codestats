#!/usr/bin/env bash
# A rustc wrapper for profile-guided builds. Cargo mixes the target into each crate's `-C metadata`, which goes into its symbol names, and a profile is matched by those names, so a profile taken on one target would match nothing on another. This names each crate's metadata after the package alone.
rustc=$1
shift
args=()
while (($#)); do
	if [[ $1 == -C && $2 == metadata=* ]]; then
		args+=(-C "metadata=${CARGO_PKG_NAME:-unknown}-${CARGO_PKG_VERSION:-0}")
		shift 2
	elif [[ $1 == metadata=* ]]; then
		args+=("metadata=${CARGO_PKG_NAME:-unknown}-${CARGO_PKG_VERSION:-0}")
		shift
	else
		args+=("$1")
		shift
	fi
done
exec "$rustc" "${args[@]}"
