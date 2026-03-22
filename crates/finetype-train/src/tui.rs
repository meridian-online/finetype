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
    fn on_batch_end(&mut self, epoch: usize, batch: usize, total_batches: usize);

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

    fn on_batch_end(&mut self, _epoch: usize, _batch: usize, _total_batches: usize) {
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
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{
        Block, Borders, Cell, Gauge, Paragraph, Row, Sparkline, Table, Wrap,
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

        fn on_batch_end(&mut self, epoch: usize, batch: usize, total_batches: usize) {
            if let Some(tx) = &self.tx {
                let _ = tx.send(RenderMsg::BatchEnd {
                    epoch,
                    batch,
                    total_batches,
                });
            }
        }

        fn on_epoch_end(&mut self, metrics: &EpochMetrics) {
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
                    }) => {
                        state.current_epoch = epoch;
                        state.current_batch = batch;
                        state.current_total_batches = total_batches;
                    }
                    Ok(RenderMsg::EpochEnd(metrics)) => {
                        state.epoch_history.push(metrics);
                    }
                    Ok(RenderMsg::TrainEnd) => {
                        state.finished = true;
                    }
                    Ok(RenderMsg::Shutdown) => {
                        // Leave alternate screen and print summary
                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                        print_final_summary(&state);
                        return Ok(());
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
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

        // Vertical layout: title(1) | charts(8) | table(flex) | progress(3)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // Title bar
                Constraint::Length(8),  // Sparkline charts
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
        let status = if state.finished {
            " [COMPLETE]"
        } else {
            ""
        };
        let text = format!(" FineType Training — {title}{status}  [{elapsed_str}]");
        let paragraph =
            Paragraph::new(text).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
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
        let block = Block::default().title(" Loss ").borders(Borders::ALL);
        let inner = block.inner(area);
        f.render_widget(block, area);

        if state.epoch_history.is_empty() {
            let msg = Paragraph::new("  Waiting for first epoch...");
            f.render_widget(msg, inner);
            return;
        }

        // Split inner vertically: train sparkline | val sparkline | legend
        let chart_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Min(1),
            ])
            .split(inner);

        // Scale losses to u64 for sparkline (multiply by 1000)
        let train_data: Vec<u64> = state
            .epoch_history
            .iter()
            .map(|m| (m.train_loss * 1000.0).min(65535.0) as u64)
            .collect();
        let val_data: Vec<u64> = state
            .epoch_history
            .iter()
            .map(|m| (m.val_loss * 1000.0).min(65535.0) as u64)
            .collect();

        let train_spark = Sparkline::default()
            .data(&train_data)
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(train_spark, chart_chunks[0]);

        let val_spark = Sparkline::default()
            .data(&val_data)
            .style(Style::default().fg(Color::Magenta));
        f.render_widget(val_spark, chart_chunks[1]);

        // Legend with latest values
        let last = state.epoch_history.last().unwrap();
        let legend = Line::from(vec![
            Span::styled("▄ train ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{:.4}  ", last.train_loss)),
            Span::styled("▄ val ", Style::default().fg(Color::Magenta)),
            Span::raw(format!("{:.4}", last.val_loss)),
        ]);
        let legend_widget = Paragraph::new(legend);
        f.render_widget(legend_widget, chart_chunks[2]);
    }

    fn draw_accuracy_chart(f: &mut ratatui::Frame, area: Rect, state: &RenderState) {
        let block = Block::default().title(" Accuracy ").borders(Borders::ALL);
        let inner = block.inner(area);
        f.render_widget(block, area);

        if state.epoch_history.is_empty() {
            let msg = Paragraph::new("  Waiting for first epoch...");
            f.render_widget(msg, inner);
            return;
        }

        let chart_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Min(1),
            ])
            .split(inner);

        // Scale accuracy to u64 (multiply by 1000, range 0-1000)
        let train_data: Vec<u64> = state
            .epoch_history
            .iter()
            .map(|m| (m.train_accuracy * 1000.0) as u64)
            .collect();
        let val_data: Vec<u64> = state
            .epoch_history
            .iter()
            .map(|m| (m.val_accuracy * 1000.0) as u64)
            .collect();

        let train_spark = Sparkline::default()
            .data(&train_data)
            .style(Style::default().fg(Color::Green));
        f.render_widget(train_spark, chart_chunks[0]);

        let val_spark = Sparkline::default()
            .data(&val_data)
            .style(Style::default().fg(Color::Blue));
        f.render_widget(val_spark, chart_chunks[1]);

        let last = state.epoch_history.last().unwrap();
        let legend = Line::from(vec![
            Span::styled("▄ train ", Style::default().fg(Color::Green)),
            Span::raw(format!("{:.1}%  ", last.train_accuracy * 100.0)),
            Span::styled("▄ val ", Style::default().fg(Color::Blue)),
            Span::raw(format!("{:.1}%", last.val_accuracy * 100.0)),
        ]);
        let legend_widget = Paragraph::new(legend);
        f.render_widget(legend_widget, chart_chunks[2]);
    }

    fn draw_epoch_table(f: &mut ratatui::Frame, area: Rect, state: &RenderState) {
        let header_cells = [
            "Epoch", "Train Loss", "Val Loss", "Train Acc", "Val Acc", "LR", "Time",
        ]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
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
}

#[cfg(feature = "tui")]
pub use tui_impl::TuiRenderer;
