# T057 — Preview pane: image + HTML support

**Priority:** P1 explorer polish. **Related:** preview tab / inspector.

## Problem

Image preview was wired but **broken**: `img(String)` maps to embedded assets,
not filesystem paths — images never painted. HTML was only syntax-highlighted
source (or plain text) with no readable extract.

## Done in this ticket (land with code)

1. **Images (disk):** load via `PathBuf` → `Resource::Path` (png/jpg/gif/bmp/svg/webp/ico/tiff).
2. **Images (archive):** decode bytes → `gpui::Image` + `preview_image_data`.
3. **HTML:** `.html`/`.htm`/`.xhtml` → readable text extract (strip tags/scripts)
   with banner; if extract empty, fall back to syntax-highlighted source.
4. Unit tests for classification + HTML strip + image path outcome.

## Residual (not this ticket)

- Full **browser-rendered** HTML (WebKit/webview) — separate if product wants it.
- Toggle Source ↔ Text extract in preview header UI.
- Image zoom/pan/EXIF.

## Done when

Code merged; unit tests green; optional grim of PNG + HTML file in preview;
architect ACCEPT (no self-ACCEPT).

## Evidence

- Claim: disk images use PathBuf — `view/preview.rs`
- Claim: HTML extract — `preview.rs` `html_to_readable_text` + tests
