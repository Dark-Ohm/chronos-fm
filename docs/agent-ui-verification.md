# Agent UI verification (Linux)

How an AI agent verifies its own GUI changes **during development** by building, launching,
operating, and observing the real app. This is interactive manual verification driven by the
agent — not CI. For headless rendering details see [build-and-display-linux.md](build-and-display-linux.md).

Use this whenever you change something user-visible (layout, a new panel, preview behavior,
navigation) and want evidence it actually works, not just that it compiles.

## What works and what doesn't

- The app renders **without a GPU** (Mesa `llvmpipe` software Vulkan) and **without a physical
  display** (any X server: an existing Xvnc, or a throwaway `Xvfb`). So this loop runs in a
  sandbox/headless box.
- There is **no DOM / accessibility tree** to query. Drive input by **screen coordinates** with
  `xdotool`, and verify by **looking at a screenshot** (read the PNG back). There is no
  `getByRole`-style selector. Keep scenarios coordinate-stable and re-screenshot after each step.
- For logic/state assertions prefer in-process `#[gpui::test]` (`TestAppContext` / `run_until_parked`)
  — faster and deterministic. This doc is for *visual / black-box* confirmation that complements it.

## One-time prerequisites

```bash
# Build-link symlinks so the GUI links without root (creates ~/.local/devlibs):
./script/ui-run.sh setup

# Tools (need root once; ask the user to run if sudo needs a password):
sudo apt-get install -y xdotool                 # input synthesis (click/type/keys)
sudo apt-get install -y ffmpeg                  # optional: screen recording -> mp4
sudo apt-get install -y xvfb                    # optional: only if there is NO existing X server
# Runtime Vulkan (llvmpipe) is usually already present: ls /usr/lib/x86_64-linux-gnu/libvulkan_lvp.so
```

`xwd` (screenshot capture) is normally preinstalled. PNG conversion uses `script/xwd2png.py`
(stdlib only — no ImageMagick/ffmpeg required for stills).

## The loop

```bash
# 1. Implement your change, then build (note the RUSTFLAGS linker path from `setup`):
RUSTFLAGS="-L $HOME/.local/devlibs" cargo build -p nohrs

# 2. Launch + wait for first real frame. Prints the window id you'll target.
./script/ui-run.sh launch
#   -> WINDOW=16777217 DISPLAY=:108 PID=...

# 3. Baseline screenshot, then read it back to see the current state.
./script/ui-run.sh shot /tmp/step0.png      # then Read /tmp/step0.png

# 4. Drive the UI. Coordinates are absolute screen pixels (match the full-screen PNG).
DISP=$(./script/ui-run.sh display); WIN=$(./script/ui-run.sh win)
DISPLAY=$DISP xdotool windowactivate "$WIN" \
  mousemove 450 666 sleep 0.4 click 1 sleep 1.5      # e.g. select a file row
./script/ui-run.sh shot /tmp/step1.png      # then Read /tmp/step1.png to confirm the effect

# 5. Repeat step 4 for each action. When done:
./script/ui-run.sh stop
```

Verification = **Read the PNG** and check the expected change happened (row highlighted, preview
populated, view toggled, breadcrumb advanced, ...). Compare against your baseline shot.

### xdotool cheatsheet (always prefix `DISPLAY=$DISP`)

| Intent | Command |
| --- | --- |
| Single click at point | `xdotool mousemove X Y click 1` |
| Double click (open folder) | `xdotool mousemove X Y click --repeat 2 --delay 200 1` |
| Type text | `xdotool type "query"` |
| Press a key / chord | `xdotool key Return` / `xdotool key ctrl+f` |
| Pace between steps | `xdotool sleep 1.2` (built-in; do **not** rely on the `sleep` binary) |
| Focus the window first | `xdotool windowactivate $WIN` |

Reference coordinates (root view, maximized 1280x800; re-shoot to recheck after layout changes):
file rows step ~32px from y≈186 (first row) downward; List/Grid toggle ≈ (1122,92)/(1185,92);
search ≈ (1240,92); breadcrumb back/forward ≈ (101,92)/(137,92).

## Recording a demo clip (optional)

```bash
DISP=$(./script/ui-run.sh display)
ffmpeg -y -f x11grab -draw_mouse 1 -framerate 20 -video_size 1280x800 -i "$DISP" \
  -c:v libx264 -pix_fmt yuv420p -movflags +faststart /tmp/demo.mp4 >/tmp/ffmpeg.log 2>&1 &
FF=$!
DISPLAY=$DISP xdotool windowactivate "$(./script/ui-run.sh win)" sleep 1 \
  <your action chain with sleeps> sleep 1
kill -INT $FF; wait $FF        # finalize the mp4 cleanly
```

Deliver `/tmp/demo.mp4` (or a still PNG) to the user — there is no chat integration to post it
for you.

## Gotchas (these cost real time)

- **Never `pkill -f nohrs`** — the pattern matches this script's own path / your shell and kills
  it (exit ~143/144). Use `pkill -x nohrs` (exact process name). `ui-run.sh stop` does this.
- **Find the window by PID, not name.** The app does not set `WM_NAME`, so `xdotool search --name`
  fails. Use `--pid` (handled by `ui-run.sh win`).
- **Black window right after launch is normal.** llvmpipe draws the first frame late; wait for the
  `Refreshing every` log line (the helper does) before screenshotting or clicking.
- **Avoid the `sleep` binary in driver scripts.** Some agent shells block long foreground sleeps.
  Use `xdotool sleep N` or `perl -e 'select(undef,undef,undef,N)'`.
- **Coordinates are brittle.** After any layout-affecting change, take a fresh baseline shot and
  re-derive coordinates before scripting clicks.
- **Headless without VNC:** if `./script/ui-run.sh display` finds nothing, start one:
  `Xvfb :99 -screen 0 1280x800x24 & export DISPLAY=:99`.
