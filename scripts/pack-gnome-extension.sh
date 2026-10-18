#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

uuid="ryshift@alexriderspy.github.io"
src="gnome-extension/$uuid"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

cp -R "$src/." "$tmp/"
glib-compile-schemas --strict "$tmp/schemas"
mkdir -p dist
rm -f "dist/$uuid.shell-extension.zip"
(cd "$tmp" && zip -qr - .) > "dist/$uuid.shell-extension.zip"
echo "dist/$uuid.shell-extension.zip"
