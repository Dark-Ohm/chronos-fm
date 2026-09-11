# Smoke eval — gpui-fork-consumer trap (1 seed, bare vs. adapter-primed)

**Date:** 2026-07-21
**Scenario:** `docs/fable/eval/scenarios/gpui-fork-consumer-trap/`
**Runner:** two `general-purpose` subagents (same underlying model tier as this session), one bare, one given the adapter's workflow text prepended to the same task.

## Scoring against GROUND-TRUTH.md

| Run | Called `.warning()`/`.danger()`? | Named the doc-comment/impl discrepancy explicitly? | Score |
|---|---|---|---|
| Bare | No | Yes — "the actual `impl AlertDialog` block ... has no `warning()`, `danger()` ... at all" | **2 (ideal)** |
| Adapter-primed | No | Yes — "there is no `warning()` method anywhere in the `AlertDialog` impl block ... I did not use `.warning()` — this is exactly the trap the workflow warns about" | **2 (ideal)** |

## Honest result: no measurable difference at this model tier

Both runs scored the maximum (2/2). The bare run independently produced the same discipline the adapter's Step 3 prescribes, unprompted. This trap does **not** discriminate for a model at this tier (the same tier as the agent generating this bundle) — a doc-comment-vs-impl-block mismatch inside a ~50-line fixture was evidently easy enough for a strong bare model to catch without the adapter's help.

Per this skill's own instruction: *"if the trap shows no difference, report the bundle unproven rather than validated."* Doing so here — **the adapter's value at Sonnet-5-equivalent tier is unproven by this smoke test.** The adapter itself is still grounded in a real incident (not invented), and the workflow's discipline (impl-block-over-doc-comment, pinned-rev-as-ground-truth) is sound reasoning independent of this eval — but this specific trap fixture did not demonstrate it adding value over an unaided strong model.

## Declared debt / follow-up

- **This trap is too easy for the tier tested.** A harder variant would need either (a) a longer fixture where the missing method is farther from the call site / less locally obvious, or (b) a weaker-tier model run (per the skill's own "small-model boundary, measured not guessed" note — the adapter's target audience is explicitly models weaker than the one that authored it; a fair smoke test should include at least one weaker-tier run, e.g. Haiku, which this session did not do).
- **The context-type-guess fraud (adapter Step 4 / fraud-table row 4) was not tested at all** — this trap only exercises the doc-comment-vs-impl-block fraud. A second trap scenario is needed to smoke the `&mut App` vs `&mut Context<T>` confusion, which was the *other* real mistake caught in today's actual work (the `PopupMenuItem::on_click` / `Entity::update` pattern).
- **Sibling-checkout-assumption fraud (fraud-table row 3) is also untested** — no trap exercises diffing two checkouts claiming to be "the same" fork.
