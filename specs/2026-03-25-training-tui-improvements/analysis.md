# Training TUI Improvements — Burn Reference Analysis

**Date:** 2026-03-25
**Analyst:** Nightingale
**Reference:** Burn TUI at `~/github/tracel-ai/burn/crates/burn-train/src/renderer/tui/`
**Our TUI:** `crates/finetype-train/src/tui.rs`

---

## Current State

FineType's training TUI uses ratatui with a background render thread (mpsc channel pattern). Features:
- Sparkline charts for loss and accuracy (train + val, stacked)
- Epoch history table with all metrics
- Progress bar with ETA
- Final summary to stdout after leaving alternate screen
- Display-only mode (no `enable_raw_mode()`) — safe for unattended overnight runs
- Epoch metrics logged to stderr for capture by `tee`

### Known Issues

1. **Dead screen during epochs** — Charts show "Waiting for first epoch..." until epoch 1 completes. Each epoch takes ~2 min on M1 Pro, so the TUI is static for the first 2 minutes.
2. **No axes or scale on charts** — Sparklines show relative shape but no absolute values. No Y-axis labels, no overlay of train/valid on same plot.
3. **No crash recovery** — If training panics, terminal stays in alternate screen mode.

---

## Burn TUI Architecture

Burn's TUI is a 12-file modular system built on the same ratatui + crossterm + mpsc stack we use.

### Layout

```
┌─────────────────────┬──────────────────────────────────────┐
│ Controls            │                                      │
│ (metric selection)  │  Chart area                          │
├─────────────────────┤  (line charts with axes,             │
│ Text Metrics        │   Braille markers, legend,           │
│ (current values)    │   tabbed metric selection)           │
├─────────────────────┤                                      │
│ Status              │                                      │
│ (mode + progress)   │                                      │
├─────────────────────┴──────────────────────────────────────┤
│ Progress bars (task + total) with ETA                      │
└────────────────────────────────────────────────────────────┘
```

### Key Design Patterns

1. **State/View separation** — Every component has a `FooState` (owns data, handles mutations) and a `FooView` (borrows state, renders). State creates view via `.view()` each frame.

2. **Metric-agnostic data model** — Metrics register dynamically (`MetricDefinition`). The TUI plots whatever metrics get registered, not hardcoded "loss" and "accuracy".

3. **Dual-resolution plotting** — `FullHistoryPlot` (250 samples max, decimates by 2x when full) and `RecentHistoryPlot` (1000 samples, sliding window). Toggle between them.

4. **Keyboard interaction** — `enable_raw_mode()` + event polling. Arrow keys switch metrics/plot types. `q` opens quit popup with graceful stop vs kill.

5. **Panic hook restoration** — Saves/restores the panic hook so terminal state is always cleaned up on crash.

---

## Feature Comparison

```
| Feature                       | FineType | Burn | Notes                                           |
|-------------------------------|----------|------|-------------------------------------------------|
| Alternate screen              | Yes      | Yes  | Same approach                                   |
| Background render thread      | Yes      | Yes  | Same mpsc pattern                               |
| Batch-level chart updates     | No       | Yes  | Burn updates plots every batch                  |
| Line charts with axes         | No       | Yes  | Burn uses Chart widget with Dataset/Axis        |
| Train/valid overlay           | No       | Yes  | Same chart, different colours + legend           |
| Braille markers               | No       | Yes  | Higher resolution than sparklines               |
| Dual-resolution history       | No       | Yes  | Full (250pt decimated) + Recent (1000pt window) |
| Metric selection (tabs)       | No       | Yes  | Arrow keys cycle Loss/Acc/LR/custom             |
| Keyboard interaction          | No       | Yes  | We're display-only (by design for overnight)    |
| Panic hook guard              | No       | Yes  | Terminal restore on crash                       |
| Epoch history table           | Yes      | No   | Our unique strength                             |
| Final stdout summary          | Yes      | No   | Our unique strength                             |
| Stderr logging with TUI       | Yes      | No   | Our unique strength (for tee capture)           |
| Display-only (unattended)     | Yes      | No   | Burn requires raw mode                          |
```

---

## Prioritised Improvements

### P1: Batch-level loss in charts

**Problem:** Charts are frozen for ~2 min during each epoch. First epoch shows "Waiting..." placeholder.

**What Burn does:** Pushes numeric values on every batch to plot buffers.

**Implementation:**
- Extend `RenderMsg::BatchEnd` to include `batch_loss: f32`
- Add `batch_loss_history: Vec<f64>` to `RenderState`
- Replace "Waiting for first epoch..." with a live loss sparkline/chart fed by batch data
- Clear batch history at epoch boundaries (or keep as intra-epoch detail)
- The training loop already computes `loss_val` per batch — just send it

**Effort:** Moderate. Changes to `RenderMsg`, `RenderState`, `TrainingRenderer` trait, training loop, and chart rendering.

### P2: Line charts with axes

**Problem:** Sparklines have no Y-axis labels, no scale, cannot overlay train/valid.

**What Burn does:** ratatui `Chart` widget with `Dataset`, `Axis`, Braille markers, legend.

**Implementation:**
- Replace two sparkline panels with `Chart` widgets
- Store data as `Vec<(f64, f64)>` (x=epoch, y=value)
- Two `Dataset` entries per chart (train + valid) with different colours
- `Axis` with min/max labels from data bounds
- ratatui's `Chart` is already in our dependency tree

**Effort:** Moderate. Mainly rendering changes, no trait modifications.

### P3: Panic hook guard

**Problem:** If training panics, terminal stays in alternate screen (broken terminal).

**What Burn does:** Saves panic hook, installs wrapper that calls `LeaveAlternateScreen`, restores original on clean exit.

**Implementation:**
```rust
// In render_loop, before entering alternate screen:
let original_hook = std::panic::take_hook();
std::panic::set_hook(Box::new(move |info| {
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
    original_hook(info);
}));
// On clean exit:
std::panic::set_hook(original_hook); // restore
```

**Effort:** Low. ~15 lines in `render_loop`.

### P4: Dual-resolution history

**Problem:** Single view per metric — no zoom in/out.

**What Burn does:** Full history (250pt, decimated) + Recent history (1000pt, sliding window).

**Implementation:**
- Only relevant after P2 (line charts)
- Two point buffers per metric
- Automatic toggle: recent during training, full on completion
- Or minimal keyboard support (P6) for manual toggle

**Effort:** Moderate. Depends on P2.

### P5: State/View separation

**Problem:** `draw_*` functions take `&RenderState` directly. Gets tangled as we add components.

**What Burn does:** `FooState` + `FooView` per component.

**Implementation:**
- Extract `ProgressView`, `ChartView`, `TableView` structs
- State creates view via `.view()`, view has `render(frame, area)`
- Cleaner extension path for future components

**Effort:** Low. Refactor only, no behaviour change.

### P6: Optional keyboard support

**Problem:** Cannot switch metrics, toggle views, or interrupt from TUI.

**What Burn does:** Full keyboard with quit popup, metric cycling.

**Implementation:**
- `--interactive` flag (default off for overnight safety)
- When on: `enable_raw_mode()`, poll events
- `q` → set shutdown flag checked by training loop
- Arrow keys → cycle metrics (only if multiple registered)
- Must be opt-in to preserve unattended run safety

**Effort:** Moderate. New input handling thread/integration.

---

## Recommendation

**For the current sprint:** Implement P1 (batch-level loss) + P3 (panic hook). These directly improve the overnight training experience — P1 eliminates the dead screen, P3 protects against terminal corruption.

**Next sprint:** P2 (line charts with axes) for a proper visual upgrade, then P4 (dual resolution) and P5 (refactor) to set up for P6 (keyboard).

**Do not adopt from Burn:**
- Metric-agnostic registration system — over-engineered for our 4-metric use case
- Full keyboard interaction as default — conflicts with our unattended overnight runs
- State/View separation immediately — only worthwhile once we have more components

---

## Key Burn Files Referenced

```
| File                  | Purpose                              |
|-----------------------|--------------------------------------|
| renderer.rs           | Top-level TuiMetricsRenderer, panic hook |
| metric_numeric.rs     | Chart rendering with Dataset/Axis    |
| recent_history.rs     | Sliding window plot (1000 points)    |
| full_history.rs       | Decimated full history (250 points)  |
| progress.rs           | Dual progress bar with warmup ETA    |
| status.rs             | Mode + items processed panel         |
| base.rs               | Layout manager, frame dispatch       |
| popup.rs              | Quit confirmation dialog             |
```
