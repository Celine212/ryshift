#!/usr/bin/env bash
# Builds the GNOME Shell extension zip (the same layout `gnome-extensions pack`
# makes) into dist/. Install it with:
#   gnome-extensions install --force dist/ryshift@alexriderspy.github.io.shell-extension.zip
# then log out and back in (Wayland) and enable it in the Extensions app.
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
