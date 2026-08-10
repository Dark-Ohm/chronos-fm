#!/usr/bin/env python3
"""
T001 mechanical consumer edit.

For every Chronos theme token `theme::CONST` in the 19 consumer files:
  * replace it with `theme::const_snake(cx)`  (the new bridge fn), and
  * strip the surrounding `rgb(...)` wrapper so the call yields an `Hsla`
    directly (the `bg`/`text_color`/... methods take `impl Into<Hsla>`).

The `cx` argument is the app context already in scope at every call site
(render/handler functions take `&mut Context<Self>` / `&mut App`, which
coerce to `&App`). After running, `cargo build` surfaces any site where the
context variable is named differently (e.g. `_cx`); those are fixed by hand.
"""

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parent.parent.parent

CONSTS = {
    "WHITE": "white", "BLACK": "black",
    "GRAY_50": "gray_50", "GRAY_100": "gray_100", "GRAY_200": "gray_200",
    "GRAY_300": "gray_300", "GRAY_400": "gray_400", "GRAY_500": "gray_500",
    "GRAY_600": "gray_600", "GRAY_700": "gray_700", "GRAY_800": "gray_800",
    "GRAY_900": "gray_900",
    "BG": "bg", "BG_SECONDARY": "bg_secondary", "BG_HOVER": "bg_hover",
    "FG": "fg", "FG_SECONDARY": "fg_secondary", "MUTED": "muted",
    "BORDER": "border", "BORDER_HOVER": "border_hover",
    "TOOLBAR_BG": "toolbar_bg", "TOOLBAR_HOVER": "toolbar_hover",
    "TOOLBAR_TEXT": "toolbar_text", "TOOLBAR_ACTIVE_BG": "toolbar_active_bg",
    "TOOLBAR_ACTIVE_TEXT": "toolbar_active_text", "TOOLBAR_BORDER": "toolbar_border",
    "ACCENT": "accent", "ACCENT_HOVER": "accent_hover",
    "ACCENT_LIGHT": "accent_light", "DANGER": "danger",
}

# Longest first so e.g. GRAY_500 is not clipped by GRAY_50.
ALTS = "|".join(sorted(CONSTS, key=len, reverse=True))
token_re = re.compile(r"theme::(" + ALTS + r")\b")


def repl_token(m):
    return f"theme::{CONSTS[m.group(1)]}(cx)"


rgb_single_re = re.compile(r"rgb\(\s*(theme::[a-z0-9_]+\(cx\))\s*\)")
rgb_if_re = re.compile(
    r"rgb\(\s*(if\s+[A-Za-z_][A-Za-z0-9_]*\s*\{[^}]*\}\s*else\s*\{[^}]*\})\s*\)",
    re.DOTALL,
)

FILES = [
    "crates/chronos-fm-pages/src/explorer/page.rs",
    "crates/chronos-fm-pages/src/explorer/view.rs",
    "crates/chronos-fm-pages/src/explorer/view/header.rs",
    "crates/chronos-fm-pages/src/explorer/view/listing/grid.rs",
    "crates/chronos-fm-pages/src/explorer/view/listing/list.rs",
    "crates/chronos-fm-pages/src/explorer/view/listing/row.rs",
    "crates/chronos-fm-pages/src/explorer/view/listing/search_bar.rs",
    "crates/chronos-fm-pages/src/explorer/view/preview.rs",
    "crates/chronos-fm-pages/src/explorer/view/sidebar.rs",
    "crates/chronos-fm-pages/src/extensions.rs",
    "crates/chronos-fm-pages/src/git.rs",
    "crates/chronos-fm-pages/src/pane_group.rs",
    "crates/chronos-fm-pages/src/root.rs",
    "crates/chronos-fm-pages/src/s3.rs",
    "crates/chronos-fm-pages/src/settings.rs",
    "crates/chronos-fm-ui/src/components/file_list.rs",
    "crates/chronos-fm-ui/src/components/layout/footer.rs",
    "crates/chronos-fm-ui/src/components/layout/unified_toolbar.rs",
    "crates/chronos-fm-ui/src/components/pane.rs",
]


def main():
    total = 0
    for rel in FILES:
        p = REPO / rel
        text = p.read_text(encoding="utf-8")
        orig = text
        text = token_re.sub(repl_token, text)
        text = rgb_single_re.sub(r"\1", text)
        text = rgb_if_re.sub(r"\1", text)
        if text != orig:
            p.write_text(text, encoding="utf-8")
            n = sum(orig.count(c) for c in ("theme::",))  # rough
            print(f"updated {rel}")
            total += 1
    print(f"{total} files updated")


if __name__ == "__main__":
    main()
