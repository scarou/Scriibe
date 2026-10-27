#!/usr/bin/env python3
"""Extrait les textes du test de frappe de TypingTest.swift vers un JSON, sans rien retaper.

Usage : python3 tools/extraire_frappe.py <TypingTest.swift> ui/src/lib/typing-texts.json
"""
import json, re, sys

src = open(sys.argv[1], encoding="utf-8").read()
start = src.index("static let texts = [")
end = src.index("\n    ]\n", start)
block = src[start:end]
STR = r'"((?:[^"\\\n]|\\.)*)"'

def decode(s):
    s = re.sub(r"\\u\{([0-9A-Fa-f]+)\}", lambda m: chr(int(m.group(1), 16)), s)
    return s.replace('\\"', '"').replace("\\n", "\n").replace("\\\\", "\\")

texts = [decode(m) for m in re.findall(STR, block)]
json.dump(texts, open(sys.argv[2], "w", encoding="utf-8"), ensure_ascii=False, indent=1)
print(f"{len(texts)} textes")
