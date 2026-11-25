#!/bin/sh
set -eu
cd "$(dirname "$0")"
dir="${XDG_DATA_HOME:-$HOME/.local/share}/gnome-shell/extensions/chroma-panel@local"
install -d "$dir/schemas"
install -m 644 extension/metadata.json extension/extension.js "$dir/"
install -m 644 extension/schemas/*.xml "$dir/schemas/"
glib-compile-schemas --strict "$dir/schemas"
echo 'Installed. Log out and back in, then run gnome-extensions enable chroma-panel@local and reopen the panel.'
