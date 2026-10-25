#!/usr/bin/env python3
"""Vérifie que chaque texte d'entrée/sortie des fixtures existe tel quel comme littéral dans Checks.swift."""
import json, re, sys, glob, os

swift_path, fixtures_dir = sys.argv[1], sys.argv[2]
src = open(swift_path, encoding="utf-8").read()
NB = "\u00a0"

def decode(s):
    s = s.replace("\\(nbsp)", NB)
    s = re.sub(r"\\u\{([0-9A-Fa-f]+)\}", lambda m: chr(int(m.group(1), 16)), s)
    out, i = [], 0
    while i < len(s):
        c = s[i]
        if c == "\\" and i + 1 < len(s):
            n = s[i + 1]
            out.append({"n": "\n", "t": "\t", '"': '"', "\\": "\\", "'": "'"}.get(n, "\\" + n))
            i += 2
        else:
            out.append(c); i += 1
    return "".join(out)

lines = src.split("\n")
def lits_in(text):
    out = set()
    for m in re.finditer(r'(?<!")#"(.*?)"#', text, re.S): out.add(m.group(1))
    for m in re.finditer(r'"((?:[^"\\\n]|\\.)*)"', text): out.add(decode(m.group(1)))
    return out
window_cache = {}
def window(line):
    if line not in window_cache:
        window_cache[line] = lits_in("\n".join(lines[max(0, line - 2): line + 6]))
    return window_cache[line]
literals = set()
# chaînes brutes #"..."#
for m in re.finditer(r'(?<!")#"(.*?)"#', src, re.S):
    literals.add(m.group(1))
# chaînes ordinaires
for m in re.finditer(r'"((?:[^"\\\n]|\\.)*)"', src):
    literals.add(decode(m.group(1)))

TEXT_FNS = {"french_typography", "cleaner_strip", "apply_dictionary", "spoken_signs_apply"}
bad = 0
checked = 0
def check(case, label, value):
    global bad, checked
    if not isinstance(value, str) or value == "":
        return
    checked += 1
    if value not in literals:
        bad += 1
        print(f"ABSENT {case['id']} [{label}] {value!r}")
    elif value not in window(case.get("text_defined_at") or case["line"]):
        bad += 1
        print(f"HORS-LIGNE {case['id']} (ligne {case['line']}) [{label}] {value!r}")

for path in sorted(glob.glob(os.path.join(fixtures_dir, "*.json"))):
    doc = json.load(open(path, encoding="utf-8"))
    for case in doc["cases"]:
        a = case["args"]
        for k in ("text", "input", "output", "json"):
            if k in a: check(case, k, a[k])
        fns = case["fn"] if isinstance(case["fn"], list) else [case["fn"]]
        if isinstance(case["expected"], str) and fns[-1] in TEXT_FNS:
            check(case, "expected", case["expected"])
print(f"{checked} textes vérifiés, {bad} absent(s).")
sys.exit(1 if bad else 0)
