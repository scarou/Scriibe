#!/usr/bin/env python3
"""Extrait les phrases types de l'essai de la mise au propre (`CleanTest.samples`) de CleanTest.swift vers un JSON, sans rien retaper.

Usage : python3 tools/extraire_phrases_types.py <CleanTest.swift> crates/core/data/clean_samples.json
"""
import json, re, sys

src = open(sys.argv[1], encoding="utf-8").read()
start = src.index("static let samples = [")
end = src.index("\n    ]\n", start)
STR = r'"((?:[^"\\\n]|\\.)*)"'

def decode(s):
    s = re.sub(r"\\u\{([0-9A-Fa-f]+)\}", lambda m: chr(int(m.group(1), 16)), s)
    return s.replace('\\"', '"').replace("\\n", "\n").replace("\\\\", "\\")

samples = [decode(m) for m in re.findall(STR, src[start:end])]
json.dump(samples, open(sys.argv[2], "w", encoding="utf-8"), ensure_ascii=False, indent=1)
print(f"{len(samples)} phrases types")
