#!/usr/bin/env python3
"""Banc des modèles de mise au propre (mode Simple, cran Léger).

Rejoue sur un `llama-server` local le VRAI prompt de Scriibe (`scriibe messages-mise-au-propre`, donc `cleaner::messages`) pour
chaque dictée d'un corpus, puis mesure les écarts mot à mot avec la dictée. Rien ne sort de l'ordinateur ; les corpus et les
résultats (texte personnel) restent hors du dépôt, dans `~/Library/Application Support/Scriibe/bancs/`.

  préparer                 construit le corpus (écarts + échantillon sans écart) depuis les historiques
  lancer MODELE.gguf NOM   lance un llama-server (port 8099), rejoue le corpus, écrit résultats-NOM.json, arrête le serveur
  rapport                  chiffres de tous les résultats-*.json (sans texte personnel)
"""
import difflib, glob, json, os, random, re, subprocess, sys, time, unicodedata, urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.expanduser("~/Library/Application Support/Scriibe")
BANCS = os.path.join(DATA, "bancs")
BIN = os.path.join(ROOT, "target", "debug", "scriibe")
PORT = 8099
# Signes dictés : leur mise en forme change légitimement les mots.
SIGNS = {"ouvrez", "ouvre", "fermez", "ferme", "guillemets", "guillemet", "parenthèse", "parenthèses", "crochet", "crochets",
         "virgule", "point", "interrogation", "exclamation", "ligne", "à", "la", "deux", "points", "tiret", "barre", "slash"}
# Cas témoins : (dictée avec accords à corriger, attendu). Phrases neutres écrites pour le banc.
AGREEMENT = [
    ("les fiches est prêt pour demain", "Les fiches sont prêtes pour demain."),
    ("elle est parti hier soir", "Elle est partie hier soir."),
    ("j'ai reçu les document que tu m'a envoyé", "J'ai reçu les documents que tu m'as envoyés."),
    ("ces dictées est bien transcrite", "Ces dictées sont bien transcrites."),
    ("la liste des modèle sont à jour", "La liste des modèles est à jour."),
    ("mes collègues sont content du résultat", "Mes collègues sont contents du résultat."),
    ("la version que j'ai publié hier", "La version que j'ai publiée hier."),
    ("nous sommes arrivé tard", "Nous sommes arrivés tard."),
    ("les réglages que tu a changé sont bon", "Les réglages que tu as changés sont bons."),
    ("cette application est très rapides", "Cette application est très rapide."),
    ("les test sont passé", "Les tests sont passés."),
    ("une nouvelle fonctionnalité est ajouté", "Une nouvelle fonctionnalité est ajoutée."),
]


def norm(w):
    w = unicodedata.normalize("NFD", w.lower())
    return "".join(c for c in w if not unicodedata.combining(c))


def tokens(text):
    return [t for t in re.split(r"[^\w]+", text.replace("'", " ").replace("’", " ")) if t]


def diffs(raw, out, dictionary):
    """Écarts réels : remplacements, suppressions, ajouts, hors signes dictés et mots du dictionnaire. Casse, accents et ponctuation ignorés."""
    keep = {norm(t) for w in dictionary for t in tokens(w)}
    a, b = tokens(raw), tokens(out)
    sm = difflib.SequenceMatcher(None, [norm(t) for t in a], [norm(t) for t in b], autojunk=False)
    found = []
    for op, i1, i2, j1, j2 in sm.get_opcodes():
        if op == "equal":
            continue
        old, new = a[i1:i2], b[j1:j2]
        if all(norm(t) in SIGNS or norm(t) in keep for t in old + new):
            continue
        found.append(("remplacement" if old and new else "suppression" if old else "ajout", " ".join(old), " ".join(new)))
    return found


def chat(messages):
    body = json.dumps({"model": "banc", "temperature": 0, "max_tokens": 1200, "messages": messages}).encode()
    req = urllib.request.Request(f"http://127.0.0.1:{PORT}/v1/chat/completions", body, {"Content-Type": "application/json"})
    t = time.time()
    with urllib.request.urlopen(req, timeout=120) as r:
        data = json.load(r)
    return data["choices"][0]["message"]["content"], time.time() - t


def strip(s):
    """Version réduite de `cleaner::strip` (balises, guillemets autour du tout) : le banc mesure le texte, pas le nettoyage."""
    s = re.sub(r"</?dictation>|</?dictée>", "", s).strip()
    if len(s) > 2 and s[0] in '"«' and s[-1] in '"»' and s[1:-1].count('"') == 0:
        s = s[1:-1].strip()
    return s


def prepare():
    os.makedirs(BANCS, exist_ok=True)
    div = json.load(open(os.path.join(BANCS, "divergences-simple.json")))
    seen = {d["raw"] for d in div}
    dictionary = []
    for p in glob.glob(f"{DATA}/utilisateurs/*/dictionary.json"):
        dictionary += json.load(open(p)).get("words", [])
    clean = []
    for p in glob.glob(f"{DATA}/utilisateurs/*/history.json"):
        for e in json.load(open(p)):
            raw, fin = e.get("rawText", ""), e.get("finalText", "")
            if e.get("mode") == "Simple" and raw not in seen and len(raw.split()) >= 8 and not diffs(raw, fin, dictionary):
                clean.append(raw)
    random.Random(7).shuffle(clean)
    corpus = [{"raw": d["raw"], "groupe": "ecart"} for d in div] + [{"raw": r, "groupe": "sans-ecart"} for r in clean[:60]]
    corpus += [{"raw": r, "groupe": "accord", "attendu": a} for r, a in AGREEMENT]
    json.dump(corpus, open(os.path.join(BANCS, "corpus-banc.json"), "w"), ensure_ascii=False)
    json.dump({"words": sorted(set(dictionary))}, open(os.path.join(BANCS, "dictionnaire-banc.json"), "w"), ensure_ascii=False)
    print(f"corpus : {len(div)} avec écart, {min(60, len(clean))} sans écart (sur {len(clean)}), {len(AGREEMENT)} accords")


def run(model, name):
    corpus = json.load(open(os.path.join(BANCS, "corpus-banc.json")))
    dpath = os.path.join(BANCS, "dictionnaire-banc.json")
    dictionary = json.load(open(dpath))["words"]
    tmp = os.path.join(BANCS, "textes-banc.json")
    json.dump([c["raw"] for c in corpus], open(tmp, "w"), ensure_ascii=False)
    messages = json.loads(subprocess.run([BIN, "messages-mise-au-propre", tmp, dpath], capture_output=True, text=True, check=True).stdout)
    args = ["llama-server", "-m", model, "--host", "127.0.0.1", "--port", str(PORT), "-c", "8192", "-ngl", "99", "-np", "2", "-kvu", "-fa", "on", "--jinja"]
    server = subprocess.Popen(args, stdout=subprocess.DEVNULL, stderr=open(os.path.join(BANCS, f"serveur-{name}.log"), "w"))
    try:
        for _ in range(300):
            try:
                urllib.request.urlopen(f"http://127.0.0.1:{PORT}/health", timeout=2)
                break
            except Exception:
                time.sleep(2)
        else:
            sys.exit("serveur non prêt")
        chat(messages[0])  # chauffe, hors mesure
        results = []
        for c, m in zip(corpus, messages):
            try:
                out, secs = chat(m)
                out = strip(out)
            except Exception as e:
                out, secs = "", -1
            r = {"groupe": c["groupe"], "raw": c["raw"], "out": out, "secs": round(secs, 2), "diffs": diffs(c["raw"], out, dictionary)}
            if "attendu" in c:
                r["attendu"] = c["attendu"]
            results.append(r)
        json.dump(results, open(os.path.join(BANCS, f"resultats-{name}.json"), "w"), ensure_ascii=False)
        print()
    finally:
        server.terminate()
        server.wait(timeout=30)


def report():
    rows = []
    for p in sorted(glob.glob(os.path.join(BANCS, "resultats-*.json"))):
        res = json.load(open(p))
        row = {"modele": os.path.basename(p)[10:-5]}
        for g in ("ecart", "sans-ecart"):
            sub = [r for r in res if r["groupe"] == g]
            bad = [r for r in sub if r["diffs"]]
            row[g] = (len(bad), len(sub))
            row[g + "_ops"] = {k: sum(1 for r in bad for d in r["diffs"] if d[0] == k) for k in ("remplacement", "suppression", "ajout")}
        acc = [r for r in res if r["groupe"] == "accord"]
        row["accord_ok"] = (sum(1 for r in acc if norm(" ".join(tokens(r["out"]))) == norm(" ".join(tokens(r["attendu"])))), len(acc))
        row["accord_intact"] = sum(1 for r in acc if not r["diffs"])
        times = sorted(r["secs"] for r in res if r["secs"] >= 0)
        row["median_s"] = times[len(times) // 2] if times else None
        row["echecs"] = sum(1 for r in res if r["secs"] < 0 or not r["out"])
        rows.append(row)
    print(json.dumps(rows, ensure_ascii=False, indent=1))


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    if cmd == "preparer":
        prepare()
    elif cmd == "lancer" and len(sys.argv) == 4:
        run(sys.argv[2], sys.argv[3])
    elif cmd == "rapport":
        report()
    else:
        sys.exit(__doc__)
