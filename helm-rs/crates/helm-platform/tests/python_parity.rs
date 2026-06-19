//! Cross-check Rust build_command output against captured Python expectations.

use helm_platform::{build_command, RunMode};
use std::path::PathBuf;

#[derive(serde::Deserialize)]
struct Case {
    name: String,
    command: String,
    args: Vec<String>,
    platform: String,
    venv_path: Option<String>,
    run_mode: String,
    #[serde(default)]
    expected: Option<Vec<String>>,
    #[serde(default)]
    error: Option<String>,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
        .join("build_commands.json")
}

#[test]
fn matches_python_build_command() {
    let raw = std::fs::read_to_string(fixture_path()).expect("read fixture");
    let cases: Vec<Case> = serde_json::from_str(&raw).expect("parse fixture");
    assert!(!cases.is_empty(), "no cases loaded");

    let mut failures = Vec::new();
    for case in &cases {
        let mode = RunMode::parse(&case.run_mode);
        let result = build_command(
            &case.command,
            &case.args,
            Some(&case.platform),
            case.venv_path.as_deref(),
            mode,
        );

        match (&case.expected, &case.error, &result) {
            (Some(exp), _, Ok(actual)) if actual != exp => {
                failures.push(format!(
                    "{}: expected {:?}, got {:?}",
                    case.name, exp, actual
                ));
            }
            (None, Some(_err_msg), Err(_)) => {} // both errored — OK
            (Some(exp), _, Err(e)) => {
                failures.push(format!(
                    "{}: Python returned {:?}, Rust errored: {}",
                    case.name, exp, e
                ));
            }
            (None, _, Ok(actual)) => {
                failures.push(format!(
                    "{}: Python errored, Rust returned {:?}",
                    case.name, actual
                ));
            }
            _ => {}
        }
    }

    if !failures.is_empty() {
        for f in &failures {
            eprintln!("{}", f);
        }
        panic!("{} parity failures (of {})", failures.len(), cases.len());
    }
    println!("{} parity cases matched", cases.len());
}
