# T057 — Preview pane: image + HTML (WebKit) support

**Priority:** P1 explorer polish.

## Delivered

### Images
- Disk: `img(PathBuf)` so gpui loads from filesystem (String was embedded-asset bug).
- Archive: `gpui::Image` bytes path.

### HTML as web component
- `.html` / `.htm` / `.xhtml` load into **embedded WebKit** via `gpui-wry` + `wry` (`build_as_child` on the GPUI window).
- `file://` URL for disk files; `load_html` for archive members.
- Webview hidden when leaving HTML selection.
- Fallback to source + status if webview init fails (e.g. platform limits).

## Dependencies
- `gpui-wry` → `../Source/gpui-component/crates/webview`
- `wry` (`lb-wry` 0.53.3) — needs **webkit2gtk** system libs (dev/runtime)
- `url` for `file://`

## Residual / known risk
- Linux Wayland + child webview can be flaky (wry docs: X11-oriented child). If blank, check logs / fall back path.
- No Source/Web toggle UI yet.
- Image zoom/pan residual.

## Done when
Unit tests green; live grim of rendered HTML (not raw tags); architect ACCEPT.
