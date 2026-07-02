#!/usr/bin/env bash
#
# check-artifact-registry.sh
#
# Guards against src/artifact/*.rs and the src/vorpal.rs build-call registry
# drifting apart (e.g. a file added but never wired into a `Xxx::new().build
# (context)` call, or a build call left behind after a file was deleted).
# rustc already enforces file <-> `pub mod` <-> struct-exists coherence; this
# guard covers the one invariant that compiles cleanly but breaks at runtime:
# the set script/list-artifacts.sh iterates over (source of truth for what
# CI builds) vs. the set src/vorpal.rs actually registers.
#
# Usage:
#   ./check-artifact-registry.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VORPAL_RS="$REPO_ROOT/src/vorpal.rs"

if [[ $# -gt 0 ]]; then
    echo "Error: $(basename "$0") takes no arguments" >&2
    exit 1
fi

missing_build_call=()
missing_source_file=()

# A: for every artifact list-artifacts.sh discovers, resolve its source file
# and struct name, then require a matching build call in src/vorpal.rs.
while IFS= read -r artifact; do
    file="$REPO_ROOT/src/artifact/$(tr '-' '_' <<<"$artifact").rs"

    if [[ ! -f "$file" ]]; then
        echo "Error: script/list-artifacts.sh listed '$artifact' but $file does not exist" >&2
        exit 1
    fi

    struct_name="$(grep -oE '^pub struct [A-Za-z0-9_]+' "$file" | head -n1 | awk '{print $3}')"

    if [[ -z "$struct_name" ]]; then
        echo "Error: no 'pub struct' declaration found in $file" >&2
        exit 1
    fi

    if ! grep -qE "^[[:space:]]*${struct_name}::new\(\)\.build\(context\)" "$VORPAL_RS"; then
        missing_build_call+=("$artifact -- struct ${struct_name} in ${file#"$REPO_ROOT"/} has no matching \"${struct_name}::new().build(context)\" call in src/vorpal.rs")
    fi
done < <("$SCRIPT_DIR/list-artifacts.sh")

# B: for every build call registered in src/vorpal.rs, require a source file
# under src/artifact/ that declares the corresponding struct.
while IFS= read -r struct_name; do
    if ! grep -lrE "^pub struct ${struct_name}(<|;| \{)" "$REPO_ROOT"/src/artifact/*.rs >/dev/null; then
        missing_source_file+=("${struct_name} -- registered via \"${struct_name}::new().build(context)\" in src/vorpal.rs but no src/artifact/*.rs file declares \"pub struct ${struct_name}\"")
    fi
done < <(grep -oE '^[[:space:]]*[A-Za-z0-9_]+::new\(\)\.build\(context\)' "$VORPAL_RS" | sed -E 's/^[[:space:]]*//; s/::new.*//' | sort -u)

if [[ ${#missing_build_call[@]} -gt 0 || ${#missing_source_file[@]} -gt 0 ]]; then
    echo "Artifact registry parity check FAILED" >&2
    echo >&2
    for entry in "${missing_build_call[@]:-}"; do
        [[ -n "$entry" ]] && echo "  - $entry" >&2
    done
    for entry in "${missing_source_file[@]:-}"; do
        [[ -n "$entry" ]] && echo "  - $entry" >&2
    done
    exit 1
fi

artifact_count="$("$SCRIPT_DIR/list-artifacts.sh" | wc -l | tr -d ' ')"
echo "Artifact registry parity check OK ($artifact_count artifacts)"
