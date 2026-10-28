#!/usr/bin/env python3
"""Assemble les icônes de Scriibe à partir des PNG tracés par `faire_icones.js` : `icon.ico` et `icon.png` de l'exécutable Windows, et le
masque 32×32 de la plume (octets de transparence) de l'icône de la zone de notification.

Usage : python3 tools/faire_icones.py <dossier-des-png>
"""
import sys
from pathlib import Path
from PIL import Image

dossier = Path(sys.argv[1])
depot = Path(__file__).resolve().parent.parent
icones = depot / "crates" / "cli" / "icons"
tailles = [16, 24, 32, 48, 64, 128, 256]
base = Image.open(dossier / "icone-windows-256.png").convert("RGBA")
base.save(icones / "icon.ico", sizes=[(t, t) for t in tailles])
Image.open(dossier / "icone-windows-512.png").convert("RGBA").save(icones / "icon.png", optimize=True)
plume = Image.open(dossier / "plume-32.png").convert("RGBA")
masque = bytes(plume.split()[3].tobytes())
assert len(masque) == 32 * 32
(depot / "crates" / "platform" / "src" / "windows" / "plume32.alpha").write_bytes(masque)
print("icon.ico, icon.png et plume32.alpha écrits")
