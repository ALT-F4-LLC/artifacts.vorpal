#!/usr/bin/env bash
#
# detect-changed-artifacts.sh
#
# Dynamically discovers artifacts from src/artifact/*.rs files and detects
# which artifacts need to be built based on changed files between two commits.
#
# Usage:
#   ./detect-changed-artifacts.sh <base_sha> <head_sha>  # Compare commits
#   ./detect-changed-artifacts.sh --all                   # List all artifacts (JSON)
#   ./detect-changed-artifacts.sh --list                  # List all artifacts (plain text)
#   ./detect-changed-artifacts.sh --help                  # Show usage

set -euo pipefail

# Script location (for relative paths)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Utility files to exclude from artifact discovery
EXCLUDED_FILES=("file.rs")

# Convert filename to artifact name (underscore -> hyphen)
filename_to_artifact() {
    local filename="$1"
    # Remove .rs extension and convert underscores to hyphens
    echo "${filename%.rs}" | tr '_' '-'
}

# Discover all artifacts by scanning src/artifact/*.rs
discover_artifacts() {
    local artifacts=()

    for file in "$REPO_ROOT"/src/artifact/*.rs; do
        local basename
        basename="$(basename "$file")"

        # Skip excluded files
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

    # Sort artifacts for consistent output
    printf '%s\n' "${artifacts[@]}" | sort
}

# Get all artifacts as JSON array
get_all_artifacts_json() {
    local artifacts
    artifacts="$(discover_artifacts)"

    # Convert to JSON array
    echo "$artifacts" | jq -R -s -c 'split("\n") | map(select(length > 0))'
}

# Get all artifacts as plain text (one per line)
get_all_artifacts_list() {
    discover_artifacts
}

# Get changed artifacts between two commits
get_changed_artifacts() {
    local base_sha="$1"
    local head_sha="$2"

    # Get list of changed files
    local changed_files
    changed_files="$(git -C "$REPO_ROOT" diff --name-only "$base_sha" "$head_sha" 2>/dev/null || echo "")"

    if [[ -z "$changed_files" ]]; then
        echo "[]"
        return
    fi

    # Build set of directly changed artifacts
    declare -A artifacts_to_build

    while IFS= read -r file; do
        # Check if it's an artifact file
        if [[ "$file" =~ ^src/artifact/(.+)\.rs$ ]]; then
            local basename="${BASH_REMATCH[1]}.rs"

            # Skip excluded files
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
                artifacts_to_build["$artifact"]=1
            fi
        fi
    done <<< "$changed_files"

    # Convert to sorted JSON array
    if [[ ${#artifacts_to_build[@]} -eq 0 ]]; then
        echo "[]"
    else
        printf '%s\n' "${!artifacts_to_build[@]}" | sort | jq -R -s -c 'split("\n") | map(select(length > 0))'
    fi
}

# Show usage
show_help() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS] [BASE_SHA HEAD_SHA]

Dynamically discovers artifacts from src/artifact/*.rs files and detects
which artifacts need to be built based on changed files.

Options:
  --all       Output all artifacts as a JSON array
  --list      Output all artifacts as plain text (one per line)
  --help      Show this help message

Arguments:
  BASE_SHA    Base commit SHA for comparison
  HEAD_SHA    Head commit SHA for comparison

Examples:
  $(basename "$0") --list                    # List all artifacts
  $(basename "$0") --all                     # All artifacts as JSON
  $(basename "$0") HEAD~3 HEAD               # Changes in last 3 commits
  $(basename "$0") abc123 def456             # Compare two commits
EOF
}

# Main entry point
main() {
    if [[ $# -eq 0 ]]; then
        show_help
        exit 1
    fi

    case "${1:-}" in
        --help|-h)
            show_help
            exit 0
            ;;
        --all)
            get_all_artifacts_json
            ;;
        --list)
            get_all_artifacts_list
            ;;
        *)
            if [[ $# -lt 2 ]]; then
                echo "Error: Two commit SHAs required for comparison" >&2
                echo "Run '$(basename "$0") --help' for usage" >&2
                exit 1
            fi
            get_changed_artifacts "$1" "$2"
            ;;
    esac
}

main "$@"
