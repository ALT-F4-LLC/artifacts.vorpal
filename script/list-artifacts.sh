#!/usr/bin/env bash
#
# list-artifacts.sh
#
# Dynamically discovers artifacts from src/artifact/*.rs files and prints
# every artifact name, one per line.
#
# Usage:
#   ./list-artifacts.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Utility files to exclude from artifact discovery
EXCLUDED_FILES=("file.rs")

# Convert filename to artifact name (underscore -> hyphen)
filename_to_artifact() {
    local filename="$1"
    echo "${filename%.rs}" | tr '_' '-'
}

# Discover all artifacts by scanning src/artifact/*.rs
discover_artifacts() {
    local artifacts=()

    for file in "$REPO_ROOT"/src/artifact/*.rs; do
        local basename
        basename="$(basename "$file")"

        local skip=false
        for excluded in "${EXCLUDED_FILES[@]}"; do
            if [[ "$basename" == "$excluded" ]]; then
                skip=true
                break
            fi
        done

        if [[ "$skip" == "false" ]]; then
            local artifact
            artifact="$(filename_to_artifact "$basename")"
            artifacts+=("$artifact")
        fi
    done

    printf '%s\n' "${artifacts[@]}" | sort
}

if [[ $# -gt 0 ]]; then
    echo "Error: $(basename "$0") takes no arguments" >&2
    exit 1
fi

discover_artifacts
