//! Answer, for a batch of (scheme, value) pairs, whether the value satisfies the
//! scheme's check digit.
//!
//! This exists so a gate written in Python can reach the check-digit code the
//! product actually uses instead of carrying a second implementation of it. The
//! authority is `finetype-core`'s `resolve`, which is the same lookup the
//! model's checksum substance guard performs; a scheme it does not know is
//! reported as `no-such-scheme` rather than passing.
//!
//! `scripts/check_taxonomy_content.py` is the caller.
//!
//! Protocol — stdin, one pair per line, tab-separated:
//!
//! ```text
//!     isbn<TAB>978-0-13-110362-7
//! ```
//!
//! stdout, one verdict per input line, in order: `ok`, `fail` or
//! `no-such-scheme`. A line that is blank or carries no tab is a usage error:
//! nothing is printed for it and the process exits 2, so a caller that mangles
//! its input gets an error rather than a verdict it can misread.

use finetype_core::checksum::resolve;
use std::io::{self, Read, Write};
use std::process;

fn main() {
    let mut input = String::new();
    if let Err(err) = io::stdin().read_to_string(&mut input) {
        eprintln!("checksum-samples: cannot read stdin: {err}");
        process::exit(2);
    }

    let mut out = String::new();
    for (index, line) in input.lines().enumerate() {
        let Some((scheme, value)) = line.split_once('\t') else {
            eprintln!(
                "checksum-samples: line {} is not `scheme<TAB>value`: {line:?}",
                index + 1
            );
            process::exit(2);
        };
        let verdict = match resolve(scheme) {
            None => "no-such-scheme",
            Some(check) if check(value) => "ok",
            Some(_) => "fail",
        };
        out.push_str(verdict);
        out.push('\n');
    }

    if let Err(err) = io::stdout().write_all(out.as_bytes()) {
        eprintln!("checksum-samples: cannot write stdout: {err}");
        process::exit(2);
    }
}
