#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 3 ]]; then
    printf 'usage: %s <version> <changelog> <output>\n' "$0" >&2
    exit 2
fi

version=$1
changelog_path=$2
output_path=$3
heading="## [$version]"

if [[ ! -f "$changelog_path" ]]; then
    printf 'error: changelog file not found: %s\n' "$changelog_path" >&2
    exit 1
fi

temp_dir=${TMPDIR:-/tmp}
body_path=$(mktemp "$temp_dir/changelog-section.XXXXXX")
trimmed_path=$(mktemp "$temp_dir/changelog-section.XXXXXX")
trap 'rm -f "$body_path" "$trimmed_path"' EXIT

is_fence=false
found=false

while IFS= read -r line || [[ -n "$line" ]]; do
    trimmed=${line#"${line%%[![:space:]]*}"}
    if [[ "$trimmed" == '```'* ]]; then
        if [[ "$is_fence" == true ]]; then
            is_fence=false
        else
            is_fence=true
        fi
        if [[ "$found" == true ]]; then
            printf '%s\n' "$line" >> "$body_path"
        fi
        continue
    fi

    if [[ "$found" == false ]]; then
        if [[ "$line" == "$heading" ]]; then
            found=true
        elif [[ "$line" == "$heading - "* && -n "${line#"$heading - "}" ]]; then
            found=true
        fi
        continue
    fi

    if [[ "$is_fence" == false && "$line" == '## ['* ]]; then
        break
    fi
    printf '%s\n' "$line" >> "$body_path"
done < "$changelog_path"

if [[ "$found" == false ]]; then
    printf "error: changelog heading missing: %s\n" "$heading" >&2
    exit 1
fi

if ! grep -q '[^[:space:]]' "$body_path"; then
    printf "error: changelog section is whitespace-only: %s\n" "$heading" >&2
    exit 1
fi

if grep -Fq '[[GAL-RELEASE-DRAFT:' "$body_path"; then
    printf 'error: changelog section contains draft sentinel: [[GAL-RELEASE-DRAFT:\n' >&2
    exit 1
fi

awk '
    /^[[:space:]]*$/ && !started { next }
    { started = 1; lines[++count] = $0 }
    END {
        while (count > 0 && lines[count] ~ /^[[:space:]]*$/) count--
        for (i = 1; i <= count; i++) print lines[i]
    }
' "$body_path" > "$trimmed_path"

if ! cat "$trimmed_path" > "$output_path"; then
    printf 'error: cannot write output: %s\n' "$output_path" >&2
    exit 1
fi
