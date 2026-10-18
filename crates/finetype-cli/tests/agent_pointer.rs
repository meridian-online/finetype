//! finetype runs no MCP server: `finetype mcp` fails and names arcform's
//! `arc mcp` and `arc skills install`, and `finetype --help` says the same
//! without listing `mcp` as a command.

use std::process::Command;

fn finetype(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_finetype"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn mcp_stub_fails_and_names_arc() {
    let out = finetype(&["mcp"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("`arc mcp`"), "{stderr}");
    assert!(stderr.contains("`arc skills install`"), "{stderr}");
}

#[test]
fn help_points_agents_to_arc_and_hides_mcp() {
    let out = finetype(&["--help"]);
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("`arc mcp`"), "{stdout}");
    assert!(stdout.contains("`arc skills install`"), "{stdout}");
    assert!(
        !stdout.lines().any(|l| l.trim_start().starts_with("mcp ")),
        "mcp listed as a command:\n{stdout}"
    );
}
