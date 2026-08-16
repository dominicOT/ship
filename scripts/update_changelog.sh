#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   ./scripts/update_changelog.sh          # regenerate Unreleased from commits since last tag
#   ./scripts/update_changelog.sh 1.2.0   # move Unreleased to a release header for version 1.2.0

version="${1:-}"
today=$(date +%F)
latest_tag=$(git describe --tags --abbrev=0 2>/dev/null || true)
range=""
if [[ -n "$latest_tag" ]]; then
  range="${latest_tag}..HEAD"
fi

commits=$(git log --pretty=format:"%s" ${range} || true)

tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT

printf "# Auto-generated changelog fragment\n\n" > "$tmp"
printf "Generated from git log: %s\n\n" "${range:-HEAD}" >> "$tmp"

features_file=$(mktemp)
fixes_file=$(mktemp)
others_file=$(mktemp)
trap 'rm -f "$tmp" "$features_file" "$fixes_file" "$others_file"' EXIT

while IFS= read -r line; do
  # parse conventional-commit style types
  if [[ $line =~ ^feat(\(.+\))?:[[:space:]]*(.*) ]]; then
    printf '%s\n' "- ${BASH_REMATCH[2]}" >> "$features_file"
  elif [[ $line =~ ^fix(\(.+\))?:[[:space:]]*(.*) ]]; then
    printf '%s\n' "- ${BASH_REMATCH[2]}" >> "$fixes_file"
  else
    printf '%s\n' "- $line" >> "$others_file"
  fi
done <<< "$commits"

if [[ -s "$features_file" ]]; then
  printf "### Features\n\n" >> "$tmp"
  cat "$features_file" >> "$tmp"
  printf "\n" >> "$tmp"
fi

if [[ -s "$fixes_file" ]]; then
  printf "### Fixes\n\n" >> "$tmp"
  cat "$fixes_file" >> "$tmp"
  printf "\n" >> "$tmp"
fi

if [[ -s "$others_file" ]]; then
  printf "### Others\n\n" >> "$tmp"
  cat "$others_file" >> "$tmp"
  printf "\n" >> "$tmp"
fi

frag=$(cat "$tmp")

changelog=CHANGELOG.md

if [[ ! -f "$changelog" ]]; then
  echo "No $changelog found; creating new one." >&2
  printf "%s\n\n" "# Changelog" > "$changelog"
fi

if [[ -n "$version" ]]; then
  # Move Unreleased -> versioned release and prepend a fresh Unreleased header
  release_header="## [$version] - $today"
  # If there is an Unreleased section, capture the current content (to move)
  perl -0777 -pe '
    if (/## \[Unreleased\](.*?)(?=^## \[|\z)/ms) {
      my $u = $1;
      s/## \[Unreleased\](.*?)(?=^## \[|\z)/## \[Unreleased\]\n\n/ms;
      s/\A/# Changelog\n\n/; # ensure header
      $_ = "$_\n";
    }
  ' "$changelog" > "$changelog.tmp" && mv "$changelog.tmp" "$changelog"

  # Prepend the new release section after Unreleased
  awk -v rel="$release_header" -v frag="$frag" '
    BEGIN{printed=0}
    { print }
    /^## \[Unreleased\]/{ if(!printed){ print "\n" rel "\n\n" frag; printed=1 } }
  ' "$changelog" > "$changelog.tmp" && mv "$changelog.tmp" "$changelog"

  echo "Moved Unreleased to $version and added changelog entries." >&2
  exit 0
fi

# Replace Unreleased section with generated fragment (or insert if missing)
awk -v frag="$frag" '
  BEGIN{in_unrel=0}
  /^## \[Unreleased\]/{ print "## [Unreleased]\n"; print frag; in_unrel=1; next }
  /^## \[.*\]/{ if(in_unrel){ in_unrel=0 } }
  { if(!in_unrel) print }
' "$changelog" > "$changelog.tmp" && mv "$changelog.tmp" "$changelog"

echo "Updated Unreleased section in $changelog" >&2
