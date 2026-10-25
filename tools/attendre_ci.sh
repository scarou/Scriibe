#!/bin/bash
# Attend la fin de l'intégration continue du dernier commit poussé, puis affiche le résultat de chaque système et, pour un échec,
# le détail publié en annotation par le workflow (les journaux complets ne sont pas téléchargeables depuis une session cloud :
# « gh run view --log-failed » répond « Forbidden »).
#
# Usage : tools/attendre_ci.sh          (depuis le dossier du dépôt, après `git push`)
set -u
repo=$(git config --get remote.origin.url | sed -E 's#(git@github.com:|https://github.com/)##; s#\.git$##')
sha=$(git rev-parse HEAD)
id=""
for _ in $(seq 1 120); do
  id=$(gh run list --repo "$repo" --commit "$sha" --limit 1 --json databaseId --jq '.[0].databaseId' 2>/dev/null)
  [ -n "$id" ] && break
  sleep 10
done
[ -z "$id" ] && { echo "Aucune exécution trouvée pour $sha"; exit 1; }
until [ "$(gh run view "$id" --repo "$repo" --json status --jq .status 2>/dev/null)" = "completed" ]; do sleep 15; done
gh run view "$id" --repo "$repo" --json conclusion,jobs --jq '{conclusion, jobs: [.jobs[] | {name, conclusion, id: .databaseId}]}'
for job in $(gh run view "$id" --repo "$repo" --json jobs --jq '.jobs[] | select(.conclusion=="failure") | .databaseId'); do
  echo "--- annotations du job $job"
  gh api "repos/$repo/check-runs/$job/annotations" --jq '.[] | .message' 2>&1 | head -80
done
