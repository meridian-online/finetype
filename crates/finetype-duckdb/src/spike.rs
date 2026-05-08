//! Spike: minimal VTab proof-of-concept for ac-04 (not a production function).
//!
//! Goal: prove that the `vtab` feature is active under `loadable-extension`
//! and that a trivial table function `finetype_spike(n BIGINT)` compiles
//! and registers alongside the existing scalar functions.
//!
//! **Status.** Retained as living evidence of the vtab feasibility boundary
//! under duckdb-rs 1.4.4's safe API. The spike ratified rollback_plan
//! Scenario A for spec `2026-04-22-duckdb-extension-ergonomics`: no new
//! production DuckDB function is registered by this spec. The CLI calls
//! `finetype_core::table_validator::validate_table` directly and writes
//! results to the output `.db` via duckdb-rs (see ac-06, ac-09).
//!
//! **Decision reference.** `.orbit/choices/0064-validate-as-duckdb-reject-pipeline.md`
//! captures the pivot rationale and cites this module as the compile-time
//! evidence for finding (a) [vtab feature available] and finding (b)
//! [scalar + table function coexistence compiles].
//!
//! **Finding (c).** `BindInfo` in duckdb-rs 1.4.4 exposes no `Connection`,
//! catalog-lookup, or row-iteration primitives — so a table function
//! cannot read from an existing DuckDB table by name. This is the
//! structural blocker that ratified Scenario A. Full findings:
//! `.orbit/specs/2026-04-22-duckdb-extension-ergonomics/spike-duckdb-rs.md`.

use duckdb::{
    core::{DataChunkHandle, Inserter, LogicalTypeHandle, LogicalTypeId},
    vtab::{BindInfo, InitInfo, TableFunctionInfo, VTab},
    Result,
};
use std::{
    error::Error,
    ffi::CString,
    sync::atomic::{AtomicBool, Ordering},
};

#[repr(C)]
pub struct SpikeBindData {
    n: i64,
}

#[repr(C)]
pub struct SpikeInitData {
    done: AtomicBool,
}

pub struct FineTypeSpike;

impl VTab for FineTypeSpike {
    type InitData = SpikeInitData;
    type BindData = SpikeBindData;

    fn bind(bind: &BindInfo) -> Result<Self::BindData, Box<dyn std::error::Error>> {
        // One result column: "value" VARCHAR
        bind.add_result_column("value", LogicalTypeHandle::from(LogicalTypeId::Varchar));
        let n = bind.get_parameter(0).to_int64();
        Ok(SpikeBindData { n })
    }

    fn init(_: &InitInfo) -> Result<Self::InitData, Box<dyn std::error::Error>> {
        Ok(SpikeInitData {
            done: AtomicBool::new(false),
        })
    }

    fn func(
        func: &TableFunctionInfo<Self>,
        output: &mut DataChunkHandle,
    ) -> Result<(), Box<dyn Error>> {
        let init = func.get_init_data();
        let bind = func.get_bind_data();

        if init.done.swap(true, Ordering::Relaxed) {
            output.set_len(0);
            return Ok(());
        }

        let rows = bind.n.max(0) as usize;
        let vector = output.flat_vector(0);
        for i in 0..rows {
            let value = CString::new(format!("row-{i}"))?;
            vector.insert(i, value);
        }
        output.set_len(rows);
        Ok(())
    }

    fn parameters() -> Option<Vec<LogicalTypeHandle>> {
        Some(vec![LogicalTypeHandle::from(LogicalTypeId::Bigint)])
    }
}
