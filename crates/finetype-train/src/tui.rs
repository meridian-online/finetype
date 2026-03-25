//! Training dashboard — display-only TUI (no keyboard capture) for live training progress.
//!
//! Two implementations of [`TrainingRenderer`]:
//! - [`TuiRenderer`] — ratatui alternate screen with background render thread (requires `tui` feature)
//! - [`LogRenderer`] — tracing::info! calls (always available, used as fallback)
//!
//! Design: display-only. No `enable_raw_mode()` — safe for unattended overnight runs.

use crate::training::EpochMetrics;

// ── Trait ────────────────────────────────────────────────────────────────────

/// Renderer interface for training progress display.
pub trait TrainingRenderer: Send {
    /// Called once before the training loop begins.
    fn on_train_start(&mut self, total_epochs: usize, batches_per_epoch: usize);

    /// Called after each training batch completes.
    fn on_batch_end(
        &mut self,
        epoch: usize,
        batch: usize,
        total_batches: usize,
        batch_loss: f32,
    );

    /// Called after each epoch completes with full metrics.
    fn on_epoch_end(&mut self, metrics: &EpochMetrics);

    /// Called once after the training loop finishes.
    fn on_train_end(&mut self);
}

// ── LogRenderer (always available) ───────────────────────────────────────────

/// Fallback renderer that logs metrics via `tracing::info!`.
pub struct LogRenderer;

impl LogRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LogRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TrainingRenderer for LogRenderer {
    fn on_train_start(&mut self, _total_epochs: usize, _batches_per_epoch: usize) {
        // Training start is already logged by train_multi_branch before renderer calls.
    }

    fn on_batch_end(
        &mut self,
        _epoch: usize,
        _batch: usize,
        _total_batches: usize,
        _batch_loss: f32,
    ) {
        // Batch-level logging is too noisy for the log renderer.
    }

    fn on_epoch_end(&mut self, _metrics: &EpochMetrics) {
        // Epoch metrics are already logged by train_multi_branch after renderer calls.
    }

    fn on_train_end(&mut self) {
        // Training completion is already logged by train_multi_branch.
    }
}

// ── TuiRenderer (feature-gated) ─────────────────────────────────────────────

#[cfg(feature = "tui")]
mod tui_impl {
    use super::*;
    use std::io;
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    use crossterm::execute;
    use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
    use ratatui::backend::CrosstermBackend;
    use ratatui::layout::{Constraint, Direction, Layout, Rect};
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::Span;
    use ratatui::symbols::Marker;
    use ratatui::widgets::{
        Axis, Block, Borders, Cell, Chart, Dataset, Gauge, Paragraph, Row, Table, Wrap,
    };
    use ratatui::Terminal;

    /// Messages sent from the training thread to the render thread.
    enum RenderMsg {
        TrainStart {
            total_epochs: usize,
            batches_per_epoch: usize,
        },
        BatchEnd {
            epoch: usize,
            batch: usize,
            total_batches: usize,
            batch_loss: f32,
        },
        EpochEnd(EpochMetrics),
        TrainEnd,
        Shutdown,
    }

    /// State held by the render thread.
    struct RenderState {
        total_epochs: usize,
        #[allow(dead_code)]
        batches_per_epoch: usize,
        current_epoch: usize,
        current_batch: usize,
        current_total_batches: usize,
        epoch_history: Vec<EpochMetrics>,
        /// Batch-level loss history for the current epoch (cleared on epoch boundary).
        batch_loss_history: Vec<f64>,
        train_start: Instant,
        finished: bool,
    }

    impl RenderState {
        fn new() -> Self {
            Self {
                total_epochs: 0,
                batches_per_epoch: 0,
                current_epoch: 0,
                current_batch: 0,
                current_total_batches: 0,
                epoch_history: Vec::new(),
                batch_loss_history: Vec::new(),
                train_start: Instant::now(),
                finished: false,
            }
        }

        fn eta_string(&self) -> String {
            if self.epoch_history.is_empty() {
                return "calculating...".to_string();
            }
            let avg_epoch_time: f32 = self
                .epoch_history
                .iter()
                .map(|m| m.epoch_time_secs)
                .sum::<f32>()
                / self.epoch_history.len() as f32;
            let remaining_epochs = self.total_epochs.saturating_sub(self.epoch_history.len());
            let eta_secs = avg_epoch_time * remaining_epochs as f32;
            if eta_secs < 60.0 {
                format!("~{:.0}s", eta_secs)
            } else if eta_secs < 3600.0 {
                format!("~{:.0} min", eta_secs / 60.0)
            } else {
                format!("~{:.1} hr", eta_secs / 3600.0)
            }
        }
    }

    /// Display-only TUI renderer using ratatui alternate screen.
    ///
    /// Spawns a background render thread at <=10 fps. No keyboard capture.
    pub struct TuiRenderer {
        tx: Option<mpsc::Sender<RenderMsg>>,
        render_thread: Option<thread::JoinHandle<()>>,
    }

    impl TuiRenderer {
        /// Create and start the TUI renderer.
        ///
        /// Enters alternate screen immediately. The render thread draws at <=10 fps.
        pub fn new(title: String) -> io::Result<Self> {
            let (tx, rx) = mpsc::channel::<RenderMsg>();

            let render_thread = thread::spawn(move || {
                if let Err(e) = render_loop(rx, &title) {
                    eprintln!("TUI render error: {e}");
                }
            });

            Ok(Self {
                tx: Some(tx),
                render_thread: Some(render_thread),
            })
        }
    }

    impl TrainingRenderer for TuiRenderer {
        fn on_train_start(&mut self, total_epochs: usize, batches_per_epoch: usize) {
            if let Some(tx) = &self.tx {
                let _ = tx.send(RenderMsg::TrainStart {
                    total_epochs,
                    batches_per_epoch,
                });
            }
        }

        fn on_batch_end(
            &mut self,
            epoch: usize,
            batch: usize,
            total_batches: usize,
            batch_loss: f32,
        ) {
            if let Some(tx) = &self.tx {
                let _ = tx.send(RenderMsg::BatchEnd {
                    epoch,
                    batch,
                    total_batches,
                    batch_loss,
                });
            }
        }

        fn on_epoch_end(&mut self, metrics: &EpochMetrics) {
            // Log to stderr so tee/redirect captures epoch progress alongside the TUI
            eprintln!(
                "Epoch {:>3}: train_loss={:.4} val_loss={:.4} train_acc={:.1}% val_acc={:.1}% lr={:.2e} ({:.1}s)",
                metrics.epoch + 1,
                metrics.train_loss,
                metrics.val_loss,
                metrics.train_accuracy * 100.0,
                metrics.val_accuracy * 100.0,
                metrics.learning_rate,
                metrics.epoch_time_secs,
            );
            if let Some(tx) = &self.tx {
                let _ = tx.send(RenderMsg::EpochEnd(metrics.clone()));
            }
        }

        fn on_train_end(&mut self) {
            if let Some(tx) = &self.tx {
                let _ = tx.send(RenderMsg::TrainEnd);
                // Give render thread a moment to draw the final frame
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }

    impl Drop for TuiRenderer {
        fn drop(&mut self) {
            if let Some(tx) = self.tx.take() {
                let _ = tx.send(RenderMsg::Shutdown);
            }
            if let Some(handle) = self.render_thread.take() {
                let _ = handle.join();
            }
        }
    }

    /// Main render loop running in background thread.
    fn render_loop(rx: mpsc::Receiver<RenderMsg>, title: &str) -> io::Result<()> {
        // Panic hook guard — restore terminal if training thread panics.
        // Covers training-thread panics only; render-thread panics are handled
        // by the error path in the thread::spawn closure.
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            original_hook(info);
        }));

        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;

        let mut state = RenderState::new();
        let tick_rate = Duration::from_millis(100);
        let title = title.to_string();

        loop {
            // Drain all pending messages
            loop {
                match rx.try_recv() {
                    Ok(RenderMsg::TrainStart {
                        total_epochs,
                        batches_per_epoch,
                    }) => {
                        state.total_epochs = total_epochs;
                        state.batches_per_epoch = batches_per_epoch;
                        state.train_start = Instant::now();
                    }
                    Ok(RenderMsg::BatchEnd {
                        epoch,
                        batch,
                        total_batches,
                        batch_loss,
                    }) => {
                        state.current_epoch = epoch;
                        state.current_batch = batch;
                        state.current_total_batches = total_batches;
                        state.batch_loss_history.push(batch_loss as f64);
                    }
                    Ok(RenderMsg::EpochEnd(metrics)) => {
                        state.batch_loss_history.clear();
                        state.epoch_history.push(metrics);
                    }
                    Ok(RenderMsg::TrainEnd) => {
                        state.finished = true;
                    }
                    Ok(RenderMsg::Shutdown) => {
                        // Leave alternate screen and print summary
                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                        // Remove our panic hook (alternate screen is gone, hook is no longer needed)
                        let _ = std::panic::take_hook();
                        print_final_summary(&state);
                        return Ok(());
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                        let _ = std::panic::take_hook();
                        print_final_summary(&state);
                        return Ok(());
                    }
                }
            }

            // Draw frame
            terminal.draw(|f| draw_frame(f, &state, &title))?;

            thread::sleep(tick_rate);
        }
    }

    /// Draw the full dashboard frame.
    fn draw_frame(f: &mut ratatui::Frame, state: &RenderState, title: &str) {
        let area = f.area();

        // Vertical layout: title(1) | charts(12) | table(flex) | progress(3)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // Title bar
                Constraint::Length(12), // Line charts with axes
                Constraint::Min(5),    // Epoch table
                Constraint::Length(4), // Progress + ETA
            ])
            .split(area);

        draw_title(f, chunks[0], title, state);
        draw_charts(f, chunks[1], state);
        draw_epoch_table(f, chunks[2], state);
        draw_progress(f, chunks[3], state);
    }

    fn draw_title(f: &mut ratatui::Frame, area: Rect, title: &str, state: &RenderState) {
        let elapsed = state.train_start.elapsed().as_secs();
        let elapsed_str = if elapsed < 60 {
            format!("{elapsed}s")
        } else if elapsed < 3600 {
            format!("{}m {}s", elapsed / 60, elapsed % 60)
        } else {
            format!("{}h {}m", elapsed / 3600, (elapsed % 3600) / 60)
        };
        let status = if state.finished { " [COMPLETE]" } else { "" };
        let text = format!(" FineType Training — {title}{status}  [{elapsed_str}]");
        let paragraph = Paragraph::new(text).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        f.render_widget(paragraph, area);
    }

    fn draw_charts(f: &mut ratatui::Frame, area: Rect, state: &RenderState) {
        // Split horizontally: Loss | Accuracy
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        draw_loss_chart(f, chunks[0], state);
        draw_accuracy_chart(f, chunks[1], state);
    }

    fn draw_loss_chart(f: &mut ratatui::Frame, area: Rect, state: &RenderState) {
        if state.batch_loss_history.is_empty() && state.epoch_history.is_empty() {
            let block = Block::default().title(" Loss ").borders(Borders::ALL);
            let inner = block.inner(area);
            f.render_widget(block, area);
            let msg = Paragraph::new("  Waiting for first batch...");
            f.render_widget(msg, inner);
            return;
        }

        // Build dataset from batch-level or epoch-level data
        let batch_points: Vec<(f64, f64)>;
        let epoch_train_points: Vec<(f64, f64)>;
        let epoch_val_points: Vec<(f64, f64)>;

        let has_batch_data = !state.batch_loss_history.is_empty();

        if has_batch_data {
            batch_points = state
                .batch_loss_history
                .iter()
                .enumerate()
                .map(|(i, &v)| (i as f64, v))
                .collect();
        } else {
            batch_points = Vec::new();
        }

        epoch_train_points = state
            .epoch_history
            .iter()
            .map(|m| (m.epoch as f64, m.train_loss as f64))
            .collect();
        epoch_val_points = state
            .epoch_history
            .iter()
            .map(|m| (m.epoch as f64, m.val_loss as f64))
            .collect();

        // Compute Y bounds across all visible data
        let all_y: Vec<f64> = if has_batch_data {
            batch_points.iter().map(|p| p.1).collect()
        } else {
            epoch_train_points
                .iter()
                .chain(epoch_val_points.iter())
                .map(|p| p.1)
                .collect()
        };
        let y_min = all_y.iter().cloned().fold(f64::INFINITY, f64::min);
        let y_max = all_y.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let y_margin = (y_max - y_min).max(0.01) * 0.1;

        let mut datasets = Vec::new();

        if has_batch_data {
            // Intra-epoch: show batch loss as primary series
            datasets.push(
                Dataset::default()
                    .name("batch")
                    .marker(Marker::Braille)
                    .graph_type(ratatui::widgets::GraphType::Line)
                    .style(Style::default().fg(Color::Yellow))
                    .data(&batch_points),
            );
            let x_max = (batch_points.len() as f64).max(1.0);

            let chart = Chart::new(datasets)
                .block(Block::default().title(" Loss (batch) ").borders(Borders::ALL))
                .x_axis(
                    Axis::default()
                        .bounds([0.0, x_max])
                        .labels(vec![
                            Span::raw("0"),
                            Span::raw(format!("{}", batch_points.len())),
                        ]),
                )
                .y_axis(
                    Axis::default()
                        .bounds([y_min - y_margin, y_max + y_margin])
                        .labels(vec![
                            Span::raw(format!("{:.3}", y_min)),
                            Span::raw(format!("{:.3}", y_max)),
                        ]),
                );
            f.render_widget(chart, area);
        } else {
            // Between epochs: show epoch-level train + val
            datasets.push(
                Dataset::default()
                    .name("train")
                    .marker(Marker::Braille)
                    .graph_type(ratatui::widgets::GraphType::Line)
                    .style(Style::default().fg(Color::Yellow))
                    .data(&epoch_train_points),
            );
            datasets.push(
                Dataset::default()
                    .name("val")
                    .marker(Marker::Braille)
                    .graph_type(ratatui::widgets::GraphType::Line)
                    .style(Style::default().fg(Color::Magenta))
                    .data(&epoch_val_points),
            );
            let x_max = (state.total_epochs as f64).max(1.0);

            let chart = Chart::new(datasets)
                .block(Block::default().title(" Loss (epoch) ").borders(Borders::ALL))
                .x_axis(
                    Axis::default()
                        .bounds([0.0, x_max])
                        .labels(vec![
                            Span::raw("0"),
                            Span::raw(format!("{}", state.total_epochs)),
                        ]),
                )
                .y_axis(
                    Axis::default()
                        .bounds([y_min - y_margin, y_max + y_margin])
                        .labels(vec![
                            Span::raw(format!("{:.3}", y_min)),
                            Span::raw(format!("{:.3}", y_max)),
                        ]),
                );
            f.render_widget(chart, area);
        }
    }

    fn draw_accuracy_chart(f: &mut ratatui::Frame, area: Rect, state: &RenderState) {
        if state.epoch_history.is_empty() {
            let block = Block::default().title(" Accuracy ").borders(Borders::ALL);
            let inner = block.inner(area);
            f.render_widget(block, area);
            let msg = Paragraph::new("  Waiting for first epoch...");
            f.render_widget(msg, inner);
            return;
        }

        let train_points: Vec<(f64, f64)> = state
            .epoch_history
            .iter()
            .map(|m| (m.epoch as f64, m.train_accuracy as f64 * 100.0))
            .collect();
        let val_points: Vec<(f64, f64)> = state
            .epoch_history
            .iter()
            .map(|m| (m.epoch as f64, m.val_accuracy as f64 * 100.0))
            .collect();

        let all_y: Vec<f64> = train_points
            .iter()
            .chain(val_points.iter())
            .map(|p| p.1)
            .collect();
        let y_min = all_y.iter().cloned().fold(f64::INFINITY, f64::min);
        let y_max = all_y.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let y_margin = (y_max - y_min).max(1.0) * 0.1;
        let y_floor = (y_min - y_margin).max(0.0);
        let y_ceil = (y_max + y_margin).min(100.0);

        let x_max = (state.total_epochs as f64).max(1.0);

        let datasets = vec![
            Dataset::default()
                .name("train")
                .marker(Marker::Braille)
                .graph_type(ratatui::widgets::GraphType::Line)
                .style(Style::default().fg(Color::Green))
                .data(&train_points),
            Dataset::default()
                .name("val")
                .marker(Marker::Braille)
                .graph_type(ratatui::widgets::GraphType::Line)
                .style(Style::default().fg(Color::Blue))
                .data(&val_points),
        ];

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .title(" Accuracy (%) ")
                    .borders(Borders::ALL),
            )
            .x_axis(
                Axis::default()
                    .title("epoch")
                    .bounds([0.0, x_max])
                    .labels(vec![
                        Span::raw("0"),
                        Span::raw(format!("{}", state.total_epochs)),
                    ]),
            )
            .y_axis(
                Axis::default()
                    .bounds([y_floor, y_ceil])
                    .labels(vec![
                        Span::raw(format!("{:.0}%", y_floor)),
                        Span::raw(format!("{:.0}%", y_ceil)),
                    ]),
            );
        f.render_widget(chart, area);
    }

    fn draw_epoch_table(f: &mut ratatui::Frame, area: Rect, state: &RenderState) {
        let header_cells = [
            "Epoch",
            "Train Loss",
            "Val Loss",
            "Train Acc",
            "Val Acc",
            "LR",
            "Time",
        ]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        });
        let header = Row::new(header_cells).height(1);

        let rows: Vec<Row> = state
            .epoch_history
            .iter()
            .map(|m| {
                Row::new(vec![
                    Cell::from(format!("{:>3}/{}", m.epoch + 1, state.total_epochs)),
                    Cell::from(format!("{:.4}", m.train_loss)),
                    Cell::from(format!("{:.4}", m.val_loss)),
                    Cell::from(format!("{:.1}%", m.train_accuracy * 100.0)),
                    Cell::from(format!("{:.1}%", m.val_accuracy * 100.0)),
                    Cell::from(format!("{:.1e}", m.learning_rate)),
                    Cell::from(format!("{:.1}s", m.epoch_time_secs)),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(8),
                Constraint::Length(11),
                Constraint::Length(11),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(8),
            ],
        )
        .header(header)
        .block(Block::default().title(" Epochs ").borders(Borders::ALL));

        f.render_widget(table, area);
    }

    fn draw_progress(f: &mut ratatui::Frame, area: Rect, state: &RenderState) {
        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(inner);

        // Epoch progress gauge
        let completed_epochs = state.epoch_history.len();
        let epoch_pct = if state.total_epochs > 0 {
            ((completed_epochs as f64 / state.total_epochs as f64) * 100.0) as u16
        } else {
            0
        };

        // Batch progress within current epoch
        let batch_pct = if state.current_total_batches > 0 {
            ((state.current_batch as f64 / state.current_total_batches as f64) * 100.0) as u16
        } else {
            0
        };

        let epoch_label = format!(
            "Epoch {}/{} [{}%]    Batch {}/{} [{}%]",
            completed_epochs,
            state.total_epochs,
            epoch_pct,
            state.current_batch,
            state.current_total_batches,
            batch_pct,
        );
        let epoch_gauge = Gauge::default()
            .gauge_style(Style::default().fg(Color::Green))
            .percent(epoch_pct)
            .label(epoch_label);
        f.render_widget(epoch_gauge, chunks[0]);

        // ETA line
        let eta_text = if state.finished {
            let elapsed = state.train_start.elapsed().as_secs_f32();
            format!("  Training complete in {:.1}s", elapsed)
        } else {
            format!("  ETA: {}", state.eta_string())
        };
        let eta_paragraph = Paragraph::new(eta_text)
            .style(Style::default().fg(Color::DarkGray))
            .wrap(Wrap { trim: true });
        f.render_widget(eta_paragraph, chunks[1]);
    }

    /// Print a final summary to stdout after leaving the alternate screen.
    fn print_final_summary(state: &RenderState) {
        if state.epoch_history.is_empty() {
            return;
        }

        println!();
        println!("Training Summary");
        println!("{}", "=".repeat(70));
        println!(
            "{:>5}  {:>10}  {:>10}  {:>9}  {:>9}  {:>10}  {:>7}",
            "Epoch", "Train Loss", "Val Loss", "Train Acc", "Val Acc", "LR", "Time"
        );
        println!("{}", "-".repeat(70));
        for m in &state.epoch_history {
            println!(
                "{:>3}/{:<2} {:>10.4}  {:>10.4}  {:>8.1}%  {:>8.1}%  {:>10.2e}  {:>6.1}s",
                m.epoch + 1,
                state.total_epochs,
                m.train_loss,
                m.val_loss,
                m.train_accuracy * 100.0,
                m.val_accuracy * 100.0,
                m.learning_rate,
                m.epoch_time_secs,
            );
        }
        println!("{}", "=".repeat(70));

        // Best epoch
        if let Some(best) = state
            .epoch_history
            .iter()
            .max_by(|a, b| a.val_accuracy.partial_cmp(&b.val_accuracy).unwrap())
        {
            println!(
                "Best: epoch {} — val_acc={:.1}%, val_loss={:.4}",
                best.epoch + 1,
                best.val_accuracy * 100.0,
                best.val_loss,
            );
        }

        let total_time = state.train_start.elapsed().as_secs_f32();
        println!("Total time: {:.1}s", total_time);
        println!();
    }

    /// Render a single static frame of the TUI dashboard with synthetic data.
    ///
    /// Used for visual verification that the TUI layout, text, and charts render
    /// correctly before committing to an overnight training run.
    pub fn run_tui_demo() -> io::Result<()> {
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;

        // Build realistic synthetic state mimicking a mid-training run
        let mut state = RenderState::new();
        state.total_epochs = 20;
        state.current_epoch = 7;
        state.current_batch = 3200;
        state.current_total_batches = 5145;
        state.batches_per_epoch = 5145;

        // Realistic epoch history: loss decreasing, accuracy increasing
        let epoch_data: Vec<(f32, f32, f32, f32, f64, f32)> = vec![
            (3.72, 1.21, 0.405, 0.701, 1e-4, 123.5),
            (1.43, 0.62, 0.787, 0.825, 9.94e-5, 122.5),
            (0.89, 0.47, 0.847, 0.852, 9.76e-5, 121.8),
            (0.68, 0.43, 0.867, 0.865, 9.46e-5, 121.8),
            (0.57, 0.38, 0.877, 0.875, 9.05e-5, 122.4),
            (0.50, 0.36, 0.886, 0.880, 8.55e-5, 123.6),
            (0.46, 0.34, 0.892, 0.883, 7.96e-5, 121.9),
        ];
        for (i, (tl, vl, ta, va, lr, t)) in epoch_data.iter().enumerate() {
            state.epoch_history.push(EpochMetrics {
                epoch: i,
                train_loss: *tl,
                val_loss: *vl,
                train_accuracy: *ta,
                val_accuracy: *va,
                learning_rate: *lr,
                epoch_time_secs: *t,
            });
        }

        // Synthetic batch losses for current (in-progress) epoch 7
        // Simulate ~50 batches of a gradually decreasing loss
        let mut batch_losses = Vec::with_capacity(50);
        let mut loss = 0.42;
        for i in 0..50 {
            // Add some noise around a decreasing trend
            let noise = ((i as f64 * 1.7).sin() * 0.03) + ((i as f64 * 0.3).cos() * 0.02);
            batch_losses.push(loss + noise);
            loss -= 0.001;
        }
        state.batch_loss_history = batch_losses;

        let title = "sherlock-v4-sibling (demo)";

        // Draw one frame
        terminal.draw(|f| draw_frame(f, &state, title))?;

        // Wait for user to see it, then exit
        eprintln!("\n  TUI Demo — press Enter to exit...");
        let mut buf = String::new();
        let _ = io::stdin().read_line(&mut buf);

        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        println!("TUI demo complete. All panels rendered.");
        Ok(())
    }
}

#[cfg(feature = "tui")]
pub use tui_impl::TuiRenderer;

#[cfg(feature = "tui")]
pub use tui_impl::run_tui_demo;
