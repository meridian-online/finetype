//! Library surface for `finetype-cli`.
//!
//! The `finetype` binary's primary entry point is `src/main.rs`. This `lib.rs`
//! exposes a narrow set of helpers that are also exercised from integration
//! tests (`tests/`) — Rust's test harness can only link against a crate's
//! library, not its binary. Anything declared here MUST stay in lockstep with
//! its caller in `main.rs`.

pub mod enum_emission;
pub mod transform_projection;
