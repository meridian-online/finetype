# Spec Review

**Date:** 2026-03-25
**Reviewer:** Context-separated agent (fresh session)
**Spec:** orbit/specs/2026-03-25-training-tui-improvements/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] Scope constraint contradicts AC-2/AC-3

**Category:** missing-requirement
**Description:** The constraint "All changes in crates/finetype-train/src/tui.rs plus the new demo binary" is impossible to satisfy. Batch loss is computed in `multi_branch.rs:1875` (`loss_val`) and the call site at `multi_branch.rs:1884` doesn't pass it. Threading batch_loss requires changing the call site in `multi_branch.rs`.
**Evidence:** `multi_branch.rs:1884` calls `renderer.on_batch_end(epoch, batch_num + 1, total_batches)` — no loss argument. `tui.rs:19` defines `fn on_batch_end(&mut self, epoch: usize, batch: usize, total_batches: usize)` — no loss parameter.
**Recommendation:** Relax constraint to "changes in tui.rs, multi_branch.rs (call site only), plus the new demo binary."

### [WARN] "or via an extended BatchEnd message" is misleading

**Category:** constraint-conflict
**Description:** AC-2's "or" suggests an alternative that avoids multi_branch.rs changes. There is none — both approaches require the call site to supply the loss value.
**Recommendation:** Remove the hedge. Pick one approach: direct signature change.

### [WARN] No automated verification for demo binary

**Category:** test-gap
**Description:** AC-1 and AC-3 rely entirely on visual inspection. No assertion that the demo binary exits 0 or handles non-TTY environments (CI).
**Recommendation:** Explicitly scope "visual inspection only, not CI-verified" or add a `--dry-run` mode that validates state construction without entering alternate screen.

### [WARN] Panic hook scope is narrower than spec implies

**Category:** failure-mode
**Description:** The panic hook covers training-thread panics only. A render-thread panic bypasses the hook — the error path at `tui.rs:163` currently just `eprintln!`s without calling `LeaveAlternateScreen`.
**Recommendation:** Scope AC-4 to "training-thread panics" or add `catch_unwind` around the render loop body.

### [WARN] Batch loss clearing semantics unspecified

**Category:** assumption
**Description:** "Vec is cleared at each epoch boundary" but doesn't specify where — training thread or render thread. The ontology schema conflates `TuiDemoState` (demo binary) with `RenderState` (render thread), which are separate structs.
**Recommendation:** Clarify: `batch_loss_history` lives in `RenderState`, accumulated via `RenderMsg::BatchEnd { loss }`, cleared on `RenderMsg::EpochEnd`.

### [INFO] RenderState is private — demo binary can't access it

**Category:** assumption
**Description:** `RenderState` and `draw_frame` are private inside `mod tui_impl`. The demo binary (in `src/bin/`) cannot access them directly.
**Recommendation:** Require a `pub fn run_tui_demo()` exported from `tui_impl`, consistent with the existing `pub use tui_impl::TuiRenderer` pattern.

### [INFO] Val loss sparkline has no batch-level equivalent

**Category:** missing-requirement
**Description:** Current loss chart shows two sparklines (train + val). Batch data only has one series. Spec doesn't address what replaces the val loss sparkline.
**Recommendation:** Decide: single batch-loss sparkline replaces both; val loss appears only in legend and epoch table.

---

## Honest Assessment

The spec is well-scoped for a STANDARD-tier change and the existing TUI code is clean. The critical issue is the scope constraint — "all changes in tui.rs" is factually wrong and will cause confusion immediately. The RenderState visibility issue needs a concrete decision before the demo binary can be designed. Fix the scope constraint, add the visibility decision and val-loss sparkline decision, and this is ready to implement.
