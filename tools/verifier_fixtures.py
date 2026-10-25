#!/usr/bin/env python3
"""Vérifie la structure des fichiers de fixtures/ : JSON valide, schéma 1, identifiants uniques et bien formés, champs obligatoires.

Usage : python3 tools/verifier_fixtures.py [dossier]   (défaut : fixtures)
"""
import glob, json, os, re, sys

folder = sys.argv[1] if len(sys.argv) > 1 else "fixtures"
errors, total, ids = [], 0, set()
for path in sorted(glob.glob(os.path.join(folder, "*.json"))):
    name = os.path.basename(path)[:-5]
    try:
        doc = json.load(open(path, encoding="utf-8"))
    except Exception as e:
        errors.append(f"{path}: JSON illisible ({e})")
        continue
    if doc.get("schema") != 1:
        errors.append(f"{path}: schema != 1")
    if doc.get("suite") != name:
        errors.append(f"{path}: suite '{doc.get('suite')}' != nom du fichier")
    for key in ("description", "source", "cases"):
        if key not in doc:
            errors.append(f"{path}: champ '{key}' manquant")
    for case in doc.get("cases", []):
        total += 1
        cid = case.get("id", "?")
        if not re.fullmatch(re.escape(name) + r"-\d{3}", cid):
            errors.append(f"{path}: identifiant mal formé '{cid}'")
        if cid in ids:
            errors.append(f"{path}: identifiant en double '{cid}'")
        ids.add(cid)
        for key in ("name", "line", "fn", "args", "expected"):
            if key not in case:
                errors.append(f"{cid}: champ '{key}' manquant")
        if "property" not in case and case.get("expected") is None and case.get("comparison") != "fields":
            errors.append(f"{cid}: expected vide sans propriété")
for e in errors:
    print("ERREUR", e)
print(f"{total} cas, {len(errors)} erreur(s).")
sys.exit(1 if errors else 0)
