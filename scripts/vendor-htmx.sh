#!/usr/bin/env sh
set -eu

repo_root=$(cd "$(dirname "$0")/.." && pwd)
manifest="$repo_root/static/vendor/htmx.version"
target="$repo_root/static/vendor/htmx.min.js"

version=$(sed -n 's/^HTMX_VERSION=//p' "$manifest" | tr -d '[:space:]' | sed 's/^v//')
if [ -z "$version" ]; then
  echo "error: HTMX_VERSION missing or empty in $manifest" >&2
  exit 1
fi

echo "Vendoring htmx ${version}"
curl -fsSL "https://unpkg.com/htmx.org@${version}/dist/htmx.min.js" -o "$target"

echo "sha256:"
sha256sum "$target"
