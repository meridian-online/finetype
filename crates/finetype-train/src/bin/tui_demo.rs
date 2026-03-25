//! Standalone TUI demo — renders one static frame with synthetic data for visual verification.
//!
//! Run: `cargo run -p finetype-train --bin tui-demo`

fn main() {
    #[cfg(feature = "tui")]
    {
        if let Err(e) = finetype_train::tui::run_tui_demo() {
            eprintln!("TUI demo error: {e}");
            std::process::exit(1);
        }
    }

    #[cfg(not(feature = "tui"))]
    {
        eprintln!("TUI demo requires the 'tui' feature. Build with: cargo run -p finetype-train --features tui --bin tui-demo");
        std::process::exit(1);
    }
}
