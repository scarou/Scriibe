#!/usr/bin/env python3
"""Extrait les phrases du banc de dictée (et leurs contrôles) de ReadingBench.swift vers un JSON, sans rien retaper.

Usage : python3 tools/extraire_banc.py <ReadingBench.swift> crates/core/data/bench_phrases.json
"""
import json, re, sys

src = open(sys.argv[1], encoding="utf-8").read()
start = src.index("static let phrases: [Phrase] = [")
end = src.index("\n    ]\n", start)
block = src[start:end]
STR = r'"((?:[^"\\\n]|\\.)*)"'

def decode(s):
    s = re.sub(r"\\u\{([0-9A-Fa-f]+)\}", lambda m: chr(int(m.group(1), 16)), s)
    return s.replace('\\"', '"').replace("\\n", "\n").replace("\\\\", "\\")

phrases = []
for m in re.finditer(r"Phrase\(category: " + STR + r",\s*text: " + STR + r",\s*expect: " + STR + r",\s*checks: \[(.*?)\]\)", block, re.S):
    category, text, expect, checks_src = m.group(1), m.group(2), m.group(3), m.group(4)
    checks = []
    for c in re.finditer(r"\.(has|lacks|exact)\(((?:" + STR + r"(?:,\s*)?)+)\)|\.(question|newLine)", checks_src):
        if c.group(1):
            args = [decode(a) for a in re.findall(STR, c.group(2))]
            if c.group(1) == "exact":
                checks.append({"kind": "exact", "label": args[0], "form": args[1]})
            else:
                checks.append({"kind": c.group(1), "label": args[0], "forms": args[1:]})
        else:
            checks.append({"kind": "question" if c.group(c.lastindex) == "question" else "new_line"})
    phrases.append({"category": decode(category), "text": decode(text), "expect": decode(expect), "checks": checks})

json.dump(phrases, open(sys.argv[2], "w", encoding="utf-8"), ensure_ascii=False, indent=1)
print(f"{len(phrases)} phrases, {sum(len(p['checks']) for p in phrases)} contrôles")
