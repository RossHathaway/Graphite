#!/usr/bin/env python3
"""Generate the fork's own branding/ folder, replacing Graphite's branded assets
(which the Graphite Branding License forbids in modified builds).

    npm pack @mdi/svg && tar xzf mdi-svg-*.tgz      # pictogrammers' icons, Apache-2.0
    python3 designer-branding/make_branding.py <path to the extracted package/>
    node designer-branding/render_pngs.mjs           # favicons and app icon

Writes every file under branding/ that frontend/src/icons.ts and
frontend/index.html use. The output is committed, so a build needs neither
this script nor the npm package.
"""

import json
import re
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from icon_map import SOLID_12PX, SOLID_16PX, TWO_TONE_24PX  # noqa: E402
from own_icons import LOGO_MARK, OWN_12PX_SOLID, OWN_16PX_SOLID, OWN_16PX_TWO_TONE  # noqa: E402

PRODUCT = "Designer"
ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "branding"
NS = 'xmlns="http://www.w3.org/2000/svg"'


def svg(view, size, inner):
    return f'<svg {NS} viewBox="0 0 {view} {view}" width="{size}" height="{size}">{inner}</svg>\n'


def mdi_path(pkg, name):
    text = (pkg / "svg" / f"{name}.svg").read_text()
    paths = re.findall(r'<path d="([^"]+)"', text)
    if not paths:
        raise SystemExit(f"no path in mdi icon {name}")
    return "".join(f'<path d="{d}"/>' for d in paths)


def check_names(pkg):
    meta = {m["name"]: m for m in json.loads((pkg / "meta.json").read_text())}
    names = list(SOLID_12PX.values()) + list(SOLID_16PX.values()) + [n for n, _ in TWO_TONE_24PX.values()]
    bad = [n for n in names if n not in meta or meta[n].get("deprecated") or "Brand / Logo" in meta[n].get("tags", [])]
    if bad:
        raise SystemExit(f"missing, deprecated or brand icons: {bad}")


def write(rel, text):
    path = OUT / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def main():
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    pkg = Path(sys.argv[1])
    check_names(pkg)

    keep = {"favicons"}  # PNGs come from render_pngs.mjs; keep them across reruns
    if OUT.exists():
        for child in OUT.iterdir():
            if child.name not in keep:
                shutil.rmtree(child) if child.is_dir() else child.unlink()

    for name, mdi in SOLID_12PX.items():
        write(f"assets/icon-12px-solid/{name}.svg", svg(24, 12, mdi_path(pkg, mdi)))
    for name, (view, inner) in OWN_12PX_SOLID.items():
        write(f"assets/icon-12px-solid/{name}.svg", svg(view, 12, inner))

    for name, mdi in SOLID_16PX.items():
        write(f"assets/icon-16px-solid/{name}.svg", svg(24, 16, mdi_path(pkg, mdi)))
    for name, (view, inner) in OWN_16PX_SOLID.items():
        write(f"assets/icon-16px-solid/{name}.svg", svg(view, 16, inner))

    for name, (view, inner) in OWN_16PX_TWO_TONE.items():
        write(f"assets/icon-16px-two-tone/{name}.svg", svg(view, 16, inner))

    # Tool icons: the shelf tints the glyph by tool category unless the tool is active.
    for name, (mdi, colour) in TWO_TONE_24PX.items():
        write(f"assets/icon-24px-two-tone/{name}.svg", svg(24, 24, f'<g class="{colour}">{mdi_path(pkg, mdi)}</g>'))

    # Welcome-screen logotype: the mark and the product name in the editor's own font.
    write(
        "assets/graphics/graphite-logotype-solid.svg",
        f'<svg {NS} viewBox="0 0 128 30" width="128" height="30">'
        f'<g transform="translate(0 3)">{LOGO_MARK}</g>'
        f'<text x="30" y="23" font-family="\'Source Sans Pro\', sans-serif" font-weight="700" font-size="23">{PRODUCT}</text>'
        "</svg>\n",
    )
    # Square mark, for favicons and the app icon (render_pngs.mjs).
    write("mark.svg", svg(24, 512, f'<rect width="24" height="24" rx="5" fill="#2b2b2b"/><g fill="#f0f0f0">{LOGO_MARK}</g>'))

    (OUT / "favicons").mkdir(exist_ok=True)
    write(
        "favicons/site.webmanifest",
        json.dumps(
            {
                "name": f"{PRODUCT} editor",
                "short_name": PRODUCT,
                "icons": [
                    {"src": "android-chrome-192x192.png", "sizes": "192x192", "type": "image/png"},
                    {"src": "android-chrome-512x512.png", "sizes": "512x512", "type": "image/png"},
                ],
                "display": "standalone",
            }
        )
        + "\n",
    )

    shutil.copy(Path(__file__).parent / "BRANDING.md", OUT / "BRANDING.md")
    shutil.copy(pkg / "LICENSE", OUT / "LICENSE-pictogrammers.txt")
    count = sum(1 for _ in (OUT / "assets").rglob("*.svg"))
    print(f"wrote {count} SVGs to {OUT}")


if __name__ == "__main__":
    main()
