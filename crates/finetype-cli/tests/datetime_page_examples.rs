//! The datetime domain page on meridian.online prints a loop that runs
//! `finetype infer -i` over one example value per category of the datetime
//! domain. A reader who runs it must see each value read as a type in the
//! category the page lists it under.
//!
//! `the_page_loop_reads_each_value_in_its_category` runs the CLI and is part of
//! the default suite. `ft_infer_reads_the_page_loop_as_the_cli_does` also needs
//! the loadable extension (`make build-extension`) and the `duckdb` CLI, so it
//! is ignored by default and run explicitly by the CI job that builds both:
//!
//! ```sh
//! make build-extension
//! cargo test --release -p finetype-cli --test datetime_page_examples -- --include-ignored
//! ```

use std::path::PathBuf;
use std::process::Command;

/// The loop as the page prints it, verbatim. The values under test are parsed out
/// of this string, so the test runs exactly what a reader copies.
const PAGE_LOOP: &str = r#"for v in '04-03-24' '04/Mar/2024:05:06:07 +00' '05:06 AM' 'Monday' '1705312200000000' 'America/New_York' 'FY2024' 'PT30M'; do finetype infer -i "$v"; done"#;

/// What each value must read as.
enum Expect {
    /// A type in this category; the page names the category, not the leaf.
    Category(&'static str),
    /// This exact type: the answer the CLI already gave before the shape rules,
    /// which must not move.
    Exact(&'static str),
}

/// One entry per value, in the loop's order.
const EXPECTED: [(&str, Expect); 8] = [
    ("04-03-24", Expect::Category("datetime.date.")),
    (
        "04/Mar/2024:05:06:07 +00",
        Expect::Category("datetime.timestamp."),
    ),
    ("05:06 AM", Expect::Exact("datetime.time.hm_12h")),
    ("Monday", Expect::Exact("datetime.component.day_of_week")),
    (
        "1705312200000000",
        Expect::Exact("datetime.epoch.unix_microseconds"),
    ),
    ("America/New_York", Expect::Category("datetime.offset.")),
    ("FY2024", Expect::Exact("datetime.period.fiscal_year")),
    ("PT30M", Expect::Category("datetime.duration.")),
];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// The single-quoted values between `for v in` and `; do`.
fn page_values() -> Vec<&'static str> {
    let list = PAGE_LOOP
        .strip_prefix("for v in ")
        .and_then(|rest| rest.split_once("; do"))
        .map(|(list, _)| list)
        .expect("the loop reads `for v in … ; do …`");
    list.split('\'')
        .enumerate()
        .filter(|(i, _)| i % 2 == 1)
        .map(|(_, v)| v)
        .collect()
}

/// The values parsed from the loop are the ones `EXPECTED` describes, in order.
fn checked_values() -> Vec<&'static str> {
    let values = page_values();
    let expected: Vec<&str> = EXPECTED.iter().map(|(v, _)| *v).collect();
    assert_eq!(values, expected, "the loop's values and EXPECTED disagree");
    values
}

/// `finetype infer -i <value>`, as the loop runs it; the type is the first line.
fn cli_infer(value: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_finetype"))
        .args(["infer", "-i", value])
        .current_dir(workspace_root())
        .output()
        .expect("run finetype infer");
    assert!(
        out.status.success(),
        "finetype infer -i {value:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .expect("utf8")
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// Every value whose answer misses its expectation, as `value: got (wanted)`.
fn misses(answers: &[(&str, String)]) -> Vec<String> {
    EXPECTED
        .iter()
        .zip(answers)
        .filter_map(|((value, expect), (_, got))| {
            let (ok, wanted) = match expect {
                Expect::Category(prefix) => (got.starts_with(prefix), format!("{prefix}*")),
                Expect::Exact(label) => (got == label, label.to_string()),
            };
            (!ok).then(|| format!("{value:?}: {got} (wanted {wanted})"))
        })
        .collect()
}

#[test]
fn the_page_loop_reads_each_value_in_its_category() {
    let answers: Vec<(&str, String)> = checked_values()
        .into_iter()
        .map(|v| (v, cli_infer(v)))
        .collect();
    let missed = misses(&answers);
    assert!(
        missed.is_empty(),
        "finetype infer -i read {} of the page's eight outside the expected type:\n  {}",
        missed.len(),
        missed.join("\n  ")
    );
}

#[test]
#[ignore = "needs `make build-extension` and the duckdb CLI; CI runs it with --include-ignored"]
fn ft_infer_reads_the_page_loop_as_the_cli_does() {
    let root = workspace_root();
    let extension = std::env::var_os("FINETYPE_EXTENSION")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target/release/finetype.duckdb_extension"));
    assert!(
        extension.is_file(),
        "no extension at {} — run `make build-extension` first",
        extension.display()
    );
    let values = checked_values();
    let rows: Vec<String> = values
        .iter()
        .enumerate()
        .map(|(i, v)| format!("({i}, '{v}')"))
        .collect();
    let ext_dir = std::env::temp_dir().join(format!("finetype-ext-{}", std::process::id()));
    let sql = format!(
        "SET extension_directory='{}';\nLOAD '{}';\nSELECT ft_infer(v) FROM (VALUES {}) t(i, v) ORDER BY i;",
        ext_dir.display(),
        extension.display(),
        rows.join(", ")
    );
    let out = Command::new("duckdb")
        .args(["-unsigned", "-no-init", "-noheader", "-list", "-c", &sql])
        .current_dir(&root)
        .output()
        .expect("run the duckdb CLI");
    assert!(
        out.status.success(),
        "duckdb failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let ext_labels: Vec<String> = String::from_utf8(out.stdout)
        .expect("utf8")
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(
        ext_labels.len(),
        values.len(),
        "ft_infer returned {} rows for {} values: {ext_labels:?}",
        ext_labels.len(),
        values.len()
    );

    let answers: Vec<(&str, String)> = values.iter().copied().zip(ext_labels).collect();
    let missed = misses(&answers);
    assert!(
        missed.is_empty(),
        "ft_infer read {} of the page's eight outside the expected type:\n  {}",
        missed.len(),
        missed.join("\n  ")
    );
    let disagree: Vec<String> = answers
        .iter()
        .filter_map(|(v, ext)| {
            let cli = cli_infer(v);
            (cli != *ext).then(|| format!("{v:?}: ft_infer {ext}, CLI {cli}"))
        })
        .collect();
    assert!(
        disagree.is_empty(),
        "ft_infer and finetype infer -i disagree:\n  {}",
        disagree.join("\n  ")
    );
}
