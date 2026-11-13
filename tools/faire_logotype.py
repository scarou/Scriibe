#!/usr/bin/env python3
"""Fabrique le logotype de Scriibe à partir de celui de Scribe (lecture seule) : « scri » à la plume, « be » à la machine, le curseur.

Les lettres de Scribe (Dancing Script et Courier Prime, SIL OFL, déjà converties en tracés) sont reprises telles quelles ; le « i » de
« scri » est doublé pour écrire « Scriibe », et « be » et le curseur sont décalés d'autant. Sorties : les SVG de `logo/` (une couleur par
système, fond clair et fond sombre, un ton) et, dans `logo/anime/`, les six versions dont le curseur clignote lentement (chantier 8).

Usage : python3 tools/faire_logotype.py <scribe-logotype.svg de Scribe> [décalage du second « i », 25 par défaut]
"""
import os, re, sys

os.makedirs("logo/anime", exist_ok=True)

src = open(sys.argv[1], encoding="utf-8").read()
DX = float(sys.argv[2]) if len(sys.argv) > 2 else 25.0

symbol = re.search(r'<path transform="[^"]+" fill="url\(#g\)" fill-rule="evenodd" d="([^"]+)"', src).group(0)
scri, be = re.findall(r'<path fill="url\(#g\)" d="([^"]+)"', src)
cursor_x = float(re.search(r'<rect x="([\d.]+)"', src).group(1))


def shift(d, dx):
    """Décale un tracé fait seulement de M, L, Q et Z absolus (x en premier dans chaque paire)."""
    out = []
    for token in re.findall(r"[MLQZ]|-?[\d.]+", d):
        out.append(token)
    result, index = [], 0
    for token in out:
        if token in "MLQZ":
            result.append(token)
            index = 0
        else:
            value = float(token) + (dx if index % 2 == 0 else 0)
            result.append(f"{value:.2f}")
            index += 1
    return " ".join(result).replace(" Z", "Z").replace("Z ", "Z ")


subpaths = [s for s in re.split(r"(?=M)", scri) if s]
# Sous-tracés de « scri » : s (0 à 2), c (3), r (4), i (5) et son point (6).
second_i = "".join(shift(s, DX) for s in subpaths[5:7])
scri_doubled = scri.rstrip() + second_i
be_shifted = shift(be, DX) if False else "".join(shift(s, DX) for s in re.split(r"(?=M)", be) if s)
cursor = cursor_x + DX
width = 466 + DX

SYSTEMS = {"windows": ("#2F6FED", "#2257C7"), "macos": ("#00B8AD", "#009C93"), "linux": ("#7B52E0", "#5E3CC0")}
HEADER = 'xmlns="http://www.w3.org/2000/svg" viewBox="0 16 {w} 172" width="{w}" height="172"'.replace("{w}", f"{width:g}")
NOTE = (
    "  <!-- Mot complet de Scriibe : la plume qui écoute, « scrii » à la plume (Dancing Script, OFL), « be » à la machine (Courier Prime, OFL), le curseur.\n"
    "       Lettres converties en tracés : aucune police à installer. Voir logo/LISEZMOI.md. -->\n"
)


ANIM_STYLE = (
    "  <style>.c{animation:b 2.4s ease-in-out infinite}@keyframes b{0%,40%{opacity:1}55%,85%{opacity:0}100%{opacity:1}}"
    "@media (prefers-reduced-motion:reduce){.c{animation:none}}</style>\n"
)


def svg(fill_a, fill_b, end, *, mono=False, animated=False):
    if mono:
        paint, defs, cur = "currentColor", "", "currentColor"
    else:
        paint = "url(#g)"
        defs = (
            f'  <defs><linearGradient id="g" x1="0" x2="{640 + DX:g}" y1="0" y2="0" gradientUnits="userSpaceOnUse">'
            f'<stop offset=".05" stop-color="{fill_a}"/><stop offset=".32" stop-color="{fill_b}"/><stop offset=".66" stop-color="{end}"/>'
            "</linearGradient></defs>\n"
        )
        cur = fill_a
    sym = symbol.replace('fill="url(#g)"', f'fill="{paint}"')
    style = ANIM_STYLE if animated else ""
    klass = ' class="c"' if animated else ""
    return (
        f"<svg {HEADER}>\n  <title>Scriibe</title>\n{NOTE}{style}{defs}  {sym}/>\n"
        f'  <path fill="{paint}" d="{scri_doubled}"/>\n  <path fill="{paint}" d="{be_shifted}"/>\n'
        f'  <rect{klass} x="{cursor:.2f}" y="58" width="7" height="94" rx="2" fill="{cur}"/>\n</svg>\n'
    )


for name, (a, b) in SYSTEMS.items():
    open(f"logo/scriibe-logotype-{name}.svg", "w", encoding="utf-8").write(svg(a, b, "#1C2427"))
    open(f"logo/scriibe-logotype-{name}-sombre.svg", "w", encoding="utf-8").write(svg(a, b, "#F4F7F7"))
    open(f"logo/anime/scriibe-logotype-{name}.svg", "w", encoding="utf-8").write(svg(a, b, "#1C2427", animated=True))
    open(f"logo/anime/scriibe-logotype-{name}-sombre.svg", "w", encoding="utf-8").write(svg(a, b, "#F4F7F7", animated=True))
open("logo/scriibe-logotype-mono.svg", "w", encoding="utf-8").write(svg("", "", "", mono=True))

print("logotype fabriqué, décalage", DX)
