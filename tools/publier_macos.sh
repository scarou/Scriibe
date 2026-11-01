#!/bin/bash
# Publie une version de Scriibe pour Mac : Scriibe.app signée (« Scribe Dev »), zippée, en pièce jointe d'une Release GitHub.
# Usage : après `cd ui && npm ci && npm run build` et `cargo build --release`, `tools/publier_macos.sh 0.2.0`.
# Montre l'aperçu (version, commit, taille, signature) et ne publie qu'après un « oui » (publication visible de tous ceux qui ont accès au dépôt).
set -euo pipefail
cd "$(dirname "$0")/.."
version="${1:-}"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "Usage : tools/publier_macos.sh x.y.z (par exemple 0.2.0)"; exit 1; }
[ -f target/release/scriibe ] || { echo "Compile d'abord : cd ui && npm run build, puis cargo build --release"; exit 1; }
[ -z "$(git status --porcelain)" ] || { echo "Des changements ne sont pas commités : commite et pousse d'abord."; exit 1; }
git fetch -q origin main
[ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] || { echo "Le commit courant n'est pas celui de origin/main : pousse (ou tire) d'abord."; exit 1; }
if [ "$(stat -f %m target/release/scriibe)" -lt "$(git log -1 --format=%ct)" ]; then
  echo "Le programme compilé est plus ancien que le dernier commit : relance cargo build --release."; exit 1
fi
tag="v$version"
gh release view "$tag" --repo scarou/Scriibe >/dev/null 2>&1 && { echo "La version $tag existe déjà sur GitHub."; exit 1; }

out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT
SCRIIBE_VERSION="$version" SCRIIBE_APP_SORTIE="$out" tools/faire_app_macos.sh
zip="$out/Scriibe-$version-macos.zip"
ditto -c -k --keepParent "$out/Scriibe.app" "$zip"

commit="$(git rev-parse --short=7 HEAD)"
echo
echo "── Aperçu ──"
echo "Version   : $version (étiquette $tag)"
echo "Commit    : $commit — $(git log -1 --format=%s)"
echo "Fichier   : $(basename "$zip") ($(du -h "$zip" | cut -f1))"
echo "Signature : $(codesign -dv "$out/Scriibe.app" 2>&1 | grep -E 'Authority|Signature' | head -n 2 | tr '\n' ' ')"
echo "Dépôt     : scarou/Scriibe (privé : seuls les comptes invités verront cette version)"
read -r -p "Publier cette version ? (oui/non) " answer
[ "$answer" = "oui" ] || { echo "Annulé : rien n'est publié."; exit 0; }

gh release create "$tag" "$zip" --repo scarou/Scriibe --target "$(git rev-parse HEAD)" --title "Scriibe $version" \
  --notes "Scriibe $version (commit $commit) pour Mac à puce Apple, macOS 14 ou plus récent. Installation pas à pas : docs/INSTALLATION_MACOS.md."
echo "Publié : $tag"
