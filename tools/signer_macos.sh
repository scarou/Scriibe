#!/bin/sh
# Signe target/release/scriibe avec le certificat « Scribe Dev » (celui de Scribe, dans le trousseau de session) et un identifiant fixe.
# Sans cela, chaque compilation a une signature différente : macOS redemande l'accès au trousseau (une fois par clé) après chaque
# compilation. Avec une signature stable, « Toujours autoriser » tient d'une compilation à l'autre.
# Usage : après `cargo build --release`, `tools/signer_macos.sh`.
set -e
cd "$(dirname "$0")/.."
exe=target/release/scriibe
[ -f "$exe" ] || { echo "Compile d'abord : cargo build --release"; exit 1; }
codesign -f -s "Scribe Dev" --identifier fr.scriibe.cli "$exe"
codesign -d -r- "$exe" 2>&1 | tail -1
