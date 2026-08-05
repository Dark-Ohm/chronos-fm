#!/usr/bin/env python3
"""
Chronos-FM theme generator (T001 — theme reskin infrastructure).

Reads the Base16-like seed palette from the design spec
(docs/superpowers/specs/2026-08-05-theme-reskin-design.md, §2) and emits a
gpui-component ThemeSet JSON with two themes, "Chronos Dark" and
"Chronos Light", into:

    crates/chronos-fm/assets/themes/chronos.theme.json

The ~150 derived ThemeColor keys are NOT hand-picked from thin air: every
value is produced by a *fixed* rule applied to the seed — either a direct
seed role, or a lightness ± step / mix / alpha operation over a seed role.
This keeps the dark and light palettes predictably consistent (1:1 with the
ChronOS schemes the spec is derived from).

Color math is plain sRGB<->HSL; no external dependencies. Because we emit
every key explicitly, gpui-component's ThemeColor::apply_config never falls
back to the library default palette — what you see in this file is what is
rendered (subject only to apply_config's alpha clamps on the three
selection surfaces, which we respect by pre-clamping our alpha values).

Re-run:  python3 script/dev/gen_chronos_theme.py
"""

import colorsys
import json
import os

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def hex_to_rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i : i + 2], 16) for i in (0, 2, 4))


def rgb_to_hex(r, g, b):
    return "#%02X%02X%02X" % (round(r), round(g), round(b))


def hex_to_hls(h):
    r, g, b = (x / 255 for x in hex_to_rgb(h))
    return colorsys.rgb_to_hls(r, g, b)  # (h, lightness, saturation)


def hls_to_hex(h, l, s):
    r, g, b = colorsys.hls_to_rgb(h, l, s)
    return rgb_to_hex(r * 255, g * 255, b * 255)


def shift_light(hexv, dl):
    """Additive lightness shift in [-1, 1] (HSL lightness axis)."""
    h, l, s = hex_to_hls(hexv)
    l = max(0.0, min(1.0, l + dl))
    return hls_to_hex(h, l, s)


def mix(ha, hb, t):
    """Linear RGB mix, t = weight of hb (0..1)."""
    ra, ga, ba = hex_to_rgb(ha)
    rb, gb, bb = hex_to_rgb(hb)
    return rgb_to_hex(
        ra + (rb - ra) * t, ga + (gb - ga) * t, ba + (bb - ba) * t
    )


def with_alpha(hexv, a):
    """Append 2-digit alpha (0..1) -> #RRGGBBAA."""
    return hexv + "%02X" % round(max(0.0, min(1.0, a)) * 255)


# ---------------------------------------------------------------------------
# Seed palette (from spec §2).  One source of truth per mode.
# ---------------------------------------------------------------------------

SEED = {
    # "Chronos Dark" == ChronOS DEFAULT_BASE16 (Catppuccin Mocha)
    "dark": {
        "bg.primary": "1e1e2e",
        "bg.secondary": "25253b",
        "bg.tertiary": "181825",
        "bg.elevated": "313244",
        "text.primary": "cdd6f4",
        "text.secondary": "a6adc8",
        "text.muted": "6c7086",
        "text.disabled": "45475a",
        "border": "313244",          # dark border == bg.elevated
        "border.subtle": "45475a",   # == text.disabled
        "accent": "007acc",          # accent.primary / border.focused (shared both modes)
        "accent.hover": "cba6f7",    # accent.hover (Mocha mauve)
        "status.error": "f38ba8",
        "status.warning": "f9e2af",
        "status.success": "a6e3a1",
        "status.info": "89b4fa",
        "status.info_teal": "94e2d5",
    },
    # "Chronos Light" == ChronOS "Light C" (NOT Latte inversion)
    "light": {
        "bg.primary": "dde0f2",
        "bg.secondary": "e6e9fa",
        "bg.tertiary": "eceefa",
        "bg.elevated": "e0e3f4",
        "text.primary": "2c2e4a",
        "text.secondary": "5a5d80",
        "text.muted": "7d80a6",
        "text.disabled": "9a9dc0",
        "border": "c4c8e6",          # border.default (cardBorder)
        "border.subtle": "d4d7ee",   # border.subtle
        "accent": "007acc",          # identical accent in both modes (per spec rule)
        "accent.hover": "007acc",
        "status.error": "d20f39",    # Latte red: Mocha pastel unreadable on light
        "status.warning": "df8e1d",
        "status.success": "40a02b",
        "status.info": "1e66f5",
        "status.info_teal": "1e66f5",
    },
}


def build_colors(mode):
    s = SEED[mode]
    bg = "#" + s["bg.primary"]
    bg2 = "#" + s["bg.secondary"]
    bg3 = "#" + s["bg.tertiary"]
    bge = "#" + s["bg.elevated"]
    fg = "#" + s["text.primary"]
    fg2 = "#" + s["text.secondary"]
    fg3 = "#" + s["text.muted"]
    fgd = "#" + s["text.disabled"]
    border = "#" + s["border"]
    borders = "#" + s["border.subtle"]
    accent = "#" + s["accent"]
    accent_h = "#" + s["accent.hover"]
    err = "#" + s["status.error"]
    warn = "#" + s["status.warning"]
    ok = "#" + s["status.success"]
    info = "#" + s["status.info"]
    teal = "#" + s["status.info_teal"]

    WHITE = "#FFFFFF"
    c = {}

    # --- core surfaces / text / borders -------------------------------------
    c["background"] = bg
    c["foreground"] = fg
    c["border"] = border
    c["primary"] = accent
    c["primary_foreground"] = WHITE
    c["primary_hover"] = shift_light(accent, +0.08)
    c["primary_active"] = shift_light(accent, -0.08)
    c["secondary"] = bg2
    c["secondary_foreground"] = fg2
    c["secondary_hover"] = mix(bg2, bg, 0.30)
    c["secondary_active"] = shift_light(bg2, -0.05)
    c["muted"] = bg3
    c["muted_foreground"] = fg3
    c["accent"] = accent  # hard rule: accent (007acc) identical in both modes
    c["accent_foreground"] = fg
    c["input"] = border
    c["caret"] = accent
    c["link"] = accent
    c["link_hover"] = shift_light(accent, +0.08)
    c["link_active"] = shift_light(accent, -0.05)
    c["ring"] = accent
    c["overlay"] = with_alpha(bg, 0.6)
    c["window_border"] = border
    c["drop_target"] = with_alpha(accent, 0.20)
    c["drag_border"] = accent
    c["progress_bar"] = accent
    # selection: pre-clamped alpha (apply_config caps at 0.3) -> stays stable
    c["selection"] = with_alpha(accent, 0.22)

    # --- base / chart colors ------------------------------------------------
    c["red"] = err
    c["green"] = ok
    c["blue"] = accent
    c["yellow"] = warn
    c["magenta"] = accent_h
    c["cyan"] = teal
    c["red_light"] = mix(err, bg, 0.5)
    c["green_light"] = mix(ok, bg, 0.5)
    c["blue_light"] = mix(accent, bg, 0.5)
    c["yellow_light"] = mix(warn, bg, 0.5)
    c["magenta_light"] = mix(accent_h, bg, 0.5)
    c["cyan_light"] = mix(teal, bg, 0.5)
    c["chart_1"] = shift_light(accent, +0.25)
    c["chart_2"] = shift_light(accent, +0.12)
    c["chart_3"] = accent
    c["chart_4"] = shift_light(accent, -0.12)
    c["chart_5"] = shift_light(accent, -0.25)
    c["chart_bullish"] = ok
    c["chart_bearish"] = err

    # --- status -------------------------------------------------------------
    c["danger"] = err
    c["danger_foreground"] = WHITE
    c["danger_hover"] = mix(err, bg, 0.20)
    c["danger_active"] = shift_light(err, -0.08)
    c["warning"] = warn
    c["warning_foreground"] = WHITE
    c["warning_hover"] = mix(warn, bg, 0.20)
    c["warning_active"] = shift_light(warn, -0.08)
    c["success"] = ok
    c["success_foreground"] = WHITE
    c["success_hover"] = mix(ok, bg, 0.20)
    c["success_active"] = shift_light(ok, -0.08)
    c["info"] = info
    c["info_foreground"] = WHITE
    c["info_hover"] = mix(info, bg, 0.20)
    c["info_active"] = shift_light(info, -0.08)

    # --- buttons ------------------------------------------------------------
    def button(kind, base, fg0):
        p = "button" if kind == "" else f"button_{kind}"
        c[p] = base
        c[p + "_foreground"] = fg0
        c[p + "_hover"] = mix(base, bg, 0.20)
        c[p + "_active"] = shift_light(base, -0.08)

    button("", bg2, fg)
    button("primary", accent, WHITE)
    button("secondary", bg2, fg)
    button("success", ok, WHITE)
    button("info", info, WHITE)
    button("warning", warn, WHITE)
    button("danger", err, WHITE)

    # --- lists / tables -----------------------------------------------------
    c["list"] = bg
    c["list_even"] = bg2
    c["list_head"] = bg2
    c["list_hover"] = shift_light(bg, +0.05)          # neutral hover surface
    c["list_active"] = with_alpha(accent, 0.18)       # pre-clamped (cap 0.2)
    c["list_active_border"] = with_alpha(accent, 0.50)
    c["table"] = bg2
    c["table_even"] = bg2
    c["table_head"] = bg2
    c["table_foot"] = bg2
    c["table_hover"] = shift_light(bg, +0.05)
    c["table_active"] = with_alpha(accent, 0.18)
    c["table_active_border"] = with_alpha(accent, 0.50)
    c["table_row_border"] = border
    c["table_head_foreground"] = fg3
    c["table_foot_foreground"] = fg3

    # --- sidebar (left nav rail) --------------------------------------------
    c["sidebar"] = bg3
    c["sidebar_foreground"] = fg
    c["sidebar_border"] = border
    c["sidebar_accent"] = with_alpha(accent, 0.18)
    c["sidebar_accent_foreground"] = fg
    c["sidebar_primary"] = accent
    c["sidebar_primary_foreground"] = WHITE

    # --- tabs ----------------------------------------------------------------
    c["tab"] = bg
    c["tab_bar"] = bg2
    c["tab_bar_segmented"] = bg2
    c["tab_active"] = bg2
    c["tab_active_foreground"] = fg
    c["tab_foreground"] = fg2

    # --- popovers / groups / accordion --------------------------------------
    c["popover"] = bg2
    c["popover_foreground"] = fg
    c["group_box"] = bg2
    c["group_box_foreground"] = fg
    c["accordion"] = bg2
    c["accordion_hover"] = shift_light(bg2, +0.05)
    c["description_list_label"] = bg2
    c["description_list_label_foreground"] = fg3

    # --- title / status bar / tiles -----------------------------------------
    c["title_bar"] = bg
    c["title_bar_border"] = border
    c["status_bar"] = bg2
    c["status_bar_border"] = border
    c["tiles"] = bg

    # --- scrollbar / skeleton / slider / switch -----------------------------
    c["scrollbar"] = bg
    c["scrollbar_thumb"] = with_alpha(fg2, 0.40)
    c["scrollbar_thumb_hover"] = with_alpha(fg2, 0.60)
    c["skeleton"] = bg3
    c["slider_bar"] = accent
    c["slider_thumb"] = fg
    c["switch"] = bg3
    c["switch_thumb"] = bg

    return c


def main():
    themes = []
    for mode, name in (("dark", "Chronos Dark"), ("light", "Chronos Light")):
        themes.append(
            {
                "name": name,
                "mode": mode,
                "colors": build_colors(mode),
            }
        )

    doc = {
        "name": "Chronos",
        "author": "ChronOS / Chronos-FM",
        "url": "https://github.com/chronos-ecosystem/Chronos-FM",
        "themes": themes,
    }

    out_path = os.path.join(
        REPO_ROOT, "crates", "chronos-fm", "assets", "themes", "chronos.theme.json"
    )
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(doc, f, indent=2)
        f.write("\n")

    # Validation echo: the three anchor colors the architect pixel-checks.
    dark = themes[0]["colors"]
    light = themes[1]["colors"]
    print(f"wrote {out_path}")
    print(f"  anchor (dark)  background={dark['background']} "
          f"accent/primary={dark['primary']} border={dark['border']}")
    print(f"  anchor (light) background={light['background']} "
          f"accent/primary={light['primary']} border={light['border']}")
    print(f"  total keys per theme: {len(dark)}")


if __name__ == "__main__":
    main()
