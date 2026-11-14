"""Icons drawn for the fork where no pictogrammers icon fits. Released with the
fork's code (MIT OR Apache-2.0). Each value is (viewBox size, inner SVG).
Shapes are fills only: the editor colours icons with CSS `fill`. Mouse hints
mark their unpressed parts `class="dim"`, which ShortcutLabel.svelte styles."""


def _pixel_text(text, unit=1.0, x0=0.0, y0=0.0):
    """Digits and ':' in a 3x5 pixel font, as rects."""
    glyphs = {
        "1": ["010", "110", "010", "010", "111"],
        "2": ["110", "001", "010", "100", "111"],
        ":": ["0", "1", "0", "1", "0"],
    }
    out, x = [], x0
    for ch in text:
        rows = glyphs[ch]
        for r, row in enumerate(rows):
            for c, bit in enumerate(row):
                if bit == "1":
                    out.append(f'<rect x="{x + c * unit:g}" y="{y0 + r * unit:g}" width="{unit:g}" height="{unit:g}"/>')
        x += (len(rows[0]) + 1) * unit
    return "".join(out)


# A "D" for Designer: a bowl with a counter, and a nib-shaped notch.
LOGO_MARK = (
    '<path fill-rule="evenodd" d="M3 2h8.5a10 10 0 0 1 0 20H3z'
    'M7.5 6.5v11h4a5.5 5.5 0 0 0 0-11z"/>'
    '<circle cx="11.5" cy="12" r="1.6"/>'
)

# Mouse: body ring, button divider, and the parts that can be highlighted.
_RING = '<path {c} fill-rule="evenodd" d="M8 1a4.5 4.5 0 0 0-4.5 4.5v5a4.5 4.5 0 0 0 9 0v-5A4.5 4.5 0 0 0 8 1zm0 1a3.5 3.5 0 0 1 3.5 3.5v5a3.5 3.5 0 0 1-7 0v-5A3.5 3.5 0 0 1 8 2z"/>'
_DIVIDER = '<path {c} d="M7.5 2h1v4.5h-1zM4.5 6.5h7v1h-7z"/>'
_LMB = '<path {c} d="M7.5 2.05A3.5 3.5 0 0 0 4.5 5.5v1h3z"/>'
_RMB = '<path {c} d="M8.5 2.05A3.5 3.5 0 0 1 11.5 5.5v1h-3z"/>'
_WHEEL = '<rect {c} x="7.25" y="2.75" width="1.5" height="3" rx=".75"/>'
_DRAG = '<path d="M5 11l1.5-1.5v1h3v-1L11 11l-1.5 1.5v-1h-3v1z"/>'
_DOUBLE = '<path d="M6.5 9.5h1v3h-1zM8.5 9.5h1v3h-1z"/>'
_UP = '<path d="M8 9l2 2.5H6z"/>'
_DOWN = '<path d="M8 12.5l2-2.5H6z"/>'
D = 'class="dim"'


def _mouse(lit=None, extra=""):
    parts = [_RING.format(c=D), _DIVIDER.format(c=D)]
    parts.append(_LMB.format(c="" if lit == "l" else D))
    parts.append(_RMB.format(c="" if lit == "r" else D))
    if lit == "m":
        parts.append(_WHEEL.format(c=""))
    return "".join(parts) + extra


OWN_16PX_TWO_TONE = {
    "mouse-hint-none": (16, _RING.format(c="") + _DIVIDER.format(c="")),
    "mouse-hint-drag": (16, _mouse(extra=_DRAG)),
    "mouse-hint-lmb": (16, _mouse("l")),
    "mouse-hint-lmb-double": (16, _mouse("l", _DOUBLE)),
    "mouse-hint-lmb-drag": (16, _mouse("l", _DRAG)),
    "mouse-hint-rmb": (16, _mouse("r")),
    "mouse-hint-rmb-double": (16, _mouse("r", _DOUBLE)),
    "mouse-hint-rmb-drag": (16, _mouse("r", _DRAG)),
    "mouse-hint-mmb": (16, _mouse("m")),
    "mouse-hint-mmb-drag": (16, _mouse("m", _DRAG)),
    "mouse-hint-scroll-up": (16, _mouse("m", _UP)),
    "mouse-hint-scroll-down": (16, _mouse("m", _DOWN)),
}

# Guide ticks mark where the path ends; the bar shows how the cap extends.
_TICKS = '<path d="M9.5 1h1v3h-1zM9.5 12h1v3h-1z"/>'
_JUSTIFY_FULL = '<path d="M1 2h14v2H1zM1 6h14v2H1zM1 10h14v2H1z"/>'

OWN_16PX_SOLID = {
    "empty": (16, ""),
    "graphite-logo": (24, LOGO_MARK),
    "stroke-cap-butt": (16, _TICKS + '<path d="M1 5h9v6H1z"/>'),
    "stroke-cap-round": (16, _TICKS + '<path d="M1 5h9a3 3 0 0 1 0 6H1z"/>'),
    "stroke-cap-square": (16, _TICKS + '<path d="M1 5h12v6H1z"/>'),
    "stroke-join-miter": (16, '<path d="M1 15V1h14v5H6v9z"/>'),
    "stroke-join-bevel": (16, '<path d="M1 15V5l4-4h10v5H6v9z"/>'),
    "stroke-join-round": (16, '<path d="M1 15V6a5 5 0 0 1 5-5h9v5H6v9z"/>'),
    "text-justify-left": (16, _JUSTIFY_FULL + '<path d="M1 14h8v2H1z"/>'),
    "text-justify-center": (16, _JUSTIFY_FULL + '<path d="M4 14h8v2H4z"/>'),
    "text-justify-right": (16, _JUSTIFY_FULL + '<path d="M7 14h8v2H7z"/>'),
    "zoom-1x": (11, _pixel_text("1:1", 1, 1, 3)),
    "zoom-2x": (11, _pixel_text("2:1", 1, 1, 3)),
}

OWN_12PX_SOLID = {
    "empty-12px": (12, ""),
}
