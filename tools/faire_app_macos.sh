#!/bin/bash
# Assemble Scriibe.app (macOS) depuis target/release/scriibe, la signe avec « Scribe Dev » et l'installe dans ~/Applications (ou dans SCRIIBE_INSTALL=chemin/Scriibe.app).
# Usage : après `cd ui && npm run build` et `cargo build --release`, `tools/faire_app_macos.sh` (ajouter --lancer pour l'ouvrir ensuite).
#
# Les autorisations de macOS (Surveillance de l'entrée, Micro, Contrôle de l'appareil et accès aux données) vont à Scriibe.app lui-même, plus
# au Terminal qui l'a lancé : une signature stable (certificat « Scribe Dev », identifiant fr.scriibe.app) les garde d'une compilation à l'autre.
set -euo pipefail
cd "$(dirname "$0")/.."
exe=target/release/scriibe
[ -f "$exe" ] || { echo "Compile d'abord : cd ui && npm run build, puis cargo build --release"; exit 1; }

# Assemblé hors du dossier Documents : iCloud y remet des attributs Finder que codesign refuse.
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
APP="$STAGE/Scriibe.app"
INSTALL="${SCRIIBE_INSTALL:-$HOME/Applications/Scriibe.app}"

# Icône : le PNG 1024 vient de logo/scriibe-icone-macos.svg (voir logo/LISEZMOI.md).
ICONSET="$STAGE/AppIcon.iconset"
mkdir -p "$ICONSET"
for s in 16 32 128 256 512; do
  sips -z $s $s crates/cli/icons/icon-macos.png --out "$ICONSET/icon_${s}x${s}.png" >/dev/null
  sips -z $((s * 2)) $((s * 2)) crates/cli/icons/icon-macos.png --out "$ICONSET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$STAGE/AppIcon.icns"

mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$exe" "$APP/Contents/MacOS/Scriibe"
cp packaging/macos/Info.plist "$APP/Contents/Info.plist"
cp "$STAGE/AppIcon.icns" "$APP/Contents/Resources/AppIcon.icns"
# Publication d'une version (tools/publier_macos.sh) : le numéro affiché par macOS est celui de la version publiée.
if [ -n "${SCRIIBE_VERSION:-}" ]; then
  /usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $SCRIIBE_VERSION" "$APP/Contents/Info.plist"
fi
xattr -cr "$APP"

# Signature par l'empreinte du certificat (un nom en double rendrait codesign ambigu) ; sans lui, signature ad hoc (autorisations à refaire
# après chaque compilation).
CERT="$(security find-identity -p codesigning 2>/dev/null | awk '/"Scribe Dev"/ && !found { print $2; found = 1 }' || true)"
if [ -n "$CERT" ] && codesign --force --sign "$CERT" --identifier fr.scriibe.app "$APP"; then
  echo "Signé avec « Scribe Dev » : les autorisations sont gardées d'une compilation à l'autre."
else
  codesign --force --sign - --identifier fr.scriibe.app "$APP"
  echo "Signature ad hoc : les autorisations seront à redonner après chaque compilation."
fi

# Publication : l'appli signée est déposée dans ce dossier, sans toucher à l'installation en cours ni à Scriibe s'il tourne.
if [ -n "${SCRIIBE_APP_SORTIE:-}" ]; then
  mkdir -p "$SCRIIBE_APP_SORTIE"
  rm -rf "$SCRIIBE_APP_SORTIE/Scriibe.app"
  cp -R "$APP" "$SCRIIBE_APP_SORTIE/Scriibe.app"
  echo "Assemblé : $SCRIIBE_APP_SORTIE/Scriibe.app"
  exit 0
fi

# Remplace l'ancienne version, après l'avoir fermée (5 s au plus : sinon `open` réactiverait l'ancienne).
pgrep -x Scriibe | xargs -r kill 2>/dev/null || true
for _ in $(seq 1 25); do pgrep -x Scriibe >/dev/null || break; sleep 0.2; done
mkdir -p "$(dirname "$INSTALL")"
rm -rf "$INSTALL"
cp -R "$APP" "$INSTALL"
echo "Installé : $INSTALL"
if [ "${1:-}" = "--lancer" ]; then open "$INSTALL"; fi
