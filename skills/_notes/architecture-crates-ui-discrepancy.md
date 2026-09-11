# ARCHITECTURE.md `crates/ui` Discrepancy

> Task 3 of skills-graph hardening. See [[checkpoint-skill]] for the full index.

## Problem

`ARCHITECTURE.md` describes a planned `crates/ui` fork of `gpui-component`
that does **not exist on disk**. During the sibling-confusion RED test, both
agents independently flagged this: the doc presents a *planned* component
library as if it were real, but `crates/` contains only `app`, `luau`,
`plugins`, `services`.

This is a doc-vs-reality gap. It is NOT a `start-here` problem — it lives in
`ARCHITECTURE.md`, which `start-here` Step 0 tells agents to treat as
canonical. A canonical doc that describes non-existent structure is a trap.

## Evidence

- `ARCHITECTURE.md` §2/§3: references a future `crates/ui` fold-in of
  `gpui-component`.
- `chronos/Cargo.toml` + `crates/`: no `crates/ui`, no `gpui_component`
  dependency. Confirmed in the sibling test (RED + GREEN both read the
  manifest and found `gpui_component` absent).
- `chronos-shell` skill correctly states: "No UI component library — every
  element is built from raw `gpui::div()`/`.flex()`."

## Recommended fix

Mark the `crates/ui` section as **explicitly planned / not yet created**
(inline), or remove it until the work starts. Do not present it as current
state.

## Status

- [ ] File a `philip` doc-accuracy pass on `ARCHITECTURE.md`
- [ ] Correct the `crates/ui` claim (planned vs. real)

## Related
- [[checkpoint-skill]] — Task 1
- [[engram-island]] — Task 2
- [[documentation-investigation-dead-links]] — Task 4
- [[chronos-shell]] — already accurate on this point
