//! Baseline CLI contract and integration tests for the generated package
//! layout. Overlay-specific fixtures stay package-local to the generated
//! project when enabled.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command as StdCommand, Output, Stdio};
use tempfile::TempDir;

fn cmd() -> Command {
    Command::cargo_bin(env!("CARGO_PKG_NAME")).expect("binary should exist")
}

fn sandbox_args(temp_dir: &TempDir) -> Vec<String> {
    vec![
        "--config-dir".to_string(),
        temp_dir.path().join("config").display().to_string(),
        "--data-dir".to_string(),
        temp_dir.path().join("data").display().to_string(),
        "--state-dir".to_string(),
        temp_dir.path().join("state").display().to_string(),
        "--cache-dir".to_string(),
        temp_dir.path().join("cache").display().to_string(),
    ]
}

fn repl_supported() -> bool {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/repl.rs")
        .exists()
}

fn run_repl_session(temp_dir: &TempDir, format: Option<&str>, input: &str) -> Output {
    let mut command = StdCommand::new(assert_cmd::cargo::cargo_bin(env!("CARGO_PKG_NAME")));
    command.args(sandbox_args(temp_dir));
    if let Some(format) = format {
        command.args(["--format", format]);
    }
    command.arg("--repl");
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let mut child = command.spawn().expect("failed to spawn REPL session");
    child
        .stdin
        .take()
        .expect("stdin should be available")
        .write_all(input.as_bytes())
        .expect("failed to write REPL input");

    child
        .wait_with_output()
        .expect("failed to wait for REPL output")
}

#[test]
fn test_{{SKILL_NAME_SNAKE}}_version_prints_semver() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"\d+\.\d+\.\d+").unwrap());
}

#[test]
fn test_{{SKILL_NAME_SNAKE}}_top_level_auto_help_exits_zero() {
    cmd()
        .assert()
        .success()
        .stdout(predicate::str::contains("NAME"))
        .stdout(predicate::str::contains("Available subcommands"))
        .stdout(predicate::str::contains("run"));
}

#[test]
fn test_{{SKILL_NAME_SNAKE}}_non_leaf_auto_help_exits_zero() {
    cmd()
        .arg("context")
        .assert()
        .success()
        .stdout(predicate::str::contains("Available subcommands"))
        .stdout(predicate::str::contains("show"))
        .stdout(predicate::str::contains("use"));
}

#[test]
fn test_{{SKILL_NAME_SNAKE}}_help_flag_stays_plain_text_even_with_json_format() {
    let output = cmd()
        .args(["run", "--help", "--format", "json"])
        .output()
        .expect("failed to execute");

    assert!(output.status.success(), "expected exit 0");

    let stdout = String::from_utf8(output.stdout).expect("non-utf8 output");
    assert!(stdout.contains("NAME"));
    assert!(!stdout.trim_start().starts_with('{'));
}

#[test]
fn test_{{SKILL_NAME_SNAKE}}_structured_help_yaml() {
    let output = cmd()
        .args(["help", "run", "--format", "yaml"])
        .output()
        .expect("failed to execute");

    assert!(output.status.success(), "expected exit 0");

    let stdout = String::from_utf8(output.stdout).expect("non-utf8 output");
    let value: serde_yaml::Value =
        serde_yaml::from_str(&stdout).expect("stdout should be valid YAML");

    assert_eq!(value["command_path"][0], "run");
    assert!(value["runtime_directories"].is_mapping());
    assert!(value["active_context"].is_mapping());
}

#[test]
fn test_{{SKILL_NAME_SNAKE}}_structured_help_json() {
    let output = cmd()
        .args(["help", "context", "use", "--format", "json"])
        .output()
        .expect("failed to execute");

    assert!(output.status.success(), "expected exit 0");

    let stdout = String::from_utf8(output.stdout).expect("non-utf8 output");
    let value: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");

    assert_eq!(value["command_path"][0], "context");
    assert_eq!(value["command_path"][1], "use");
}

#[test]
fn test_{{SKILL_NAME_SNAKE}}_structured_help_toml() {
    let output = cmd()
        .args(["help", "paths", "--format", "toml"])
        .output()
        .expect("failed to execute");

    assert!(output.status.success(), "expected exit 0");

    let stdout = String::from_utf8(output.stdout).expect("non-utf8 output");
    let value: toml::Value = stdout.parse().expect("stdout should be valid TOML");

    assert_eq!(
        value
            .get("command_path")
            .and_then(|path| path.as_array())
            .and_then(|path| path.first())
            .and_then(|entry| entry.as_str()),
        Some("paths")
    );
}

#[test]
fn test_{{SKILL_NAME_SNAKE}}_missing_leaf_input_returns_structured_yaml_error() {
    let output = cmd().arg("run").output().expect("failed to execute");

    assert!(!output.status.success(), "expected non-zero exit");

    let stderr = String::from_utf8(output.stderr).expect("non-utf8 stderr");
    let value: serde_yaml::Value =
        serde_yaml::from_str(&stderr).expect("stderr should be valid YAML");

    assert_eq!(value["code"], "run.missing_input");
    assert!(value["message"]
        .as_str()
        .unwrap()
        .contains("requires <INPUT>"));
}

#[test]
fn test_{{SKILL_NAME_SNAKE}}_missing_leaf_input_returns_structured_json_error() {
    let output = cmd()
        .args(["run", "--format", "json"])
        .output()
        .expect("failed to execute");

    assert!(!output.status.success(), "expected non-zero exit");

    let stderr = String::from_utf8(output.stderr).expect("non-utf8 stderr");
    let value: serde_json::Value =
        serde_json::from_str(&stderr).expect("stderr should be valid JSON");

    assert_eq!(value["code"], "run.missing_input");
    assert!(value["message"]
        .as_str()
        .unwrap()
        .contains("requires <INPUT>"));
}

#[test]
fn test_{{SKILL_NAME_SNAKE}}_repl_help_stays_plain_text_when_feature_present() {
    if !repl_supported() {
        return;
    }

    let temp_dir = TempDir::new().expect("temp dir");
    let output = run_repl_session(&temp_dir, None, "help\nquit\n");

    assert!(output.status.success(), "expected exit 0");

    let stdout = String::from_utf8(output.stdout).expect("non-utf8 output");
    assert!(stdout.contains("REPL COMMANDS"));
    assert!(!stdout.trim_start().starts_with('{'));

    let stderr = String::from_utf8(output.stderr).expect("non-utf8 stderr");
    assert!(stderr.contains("{{SKILL_NAME}}> "));
}

#[test]
fn test_{{SKILL_NAME_SNAKE}}_repl_persists_history_when_feature_present() {
    if !repl_supported() {
        return;
    }

    let temp_dir = TempDir::new().expect("temp dir");
    let output = run_repl_session(
        &temp_dir,
        None,
        "help\nrun demo-input workspace=demo\nquit\n",
    );

    assert!(output.status.success(), "expected exit 0");

    let history = fs::read_to_string(temp_dir.path().join("state").join("repl-history.txt"))
        .expect("history file should exist");
    assert!(history.contains("help"));
    assert!(history.contains("run demo-input workspace=demo"));
}

#[test]
fn test_{{SKILL_NAME_SNAKE}}_repl_completion_support_is_present_when_feature_enabled() {
    if !repl_supported() {
        return;
    }

    let repl_source = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/repl.rs"))
        .expect("repl source should exist");

    assert!(repl_source.contains("impl Completer for ReplHelper"));
    assert!(repl_source.contains("completion_candidates"));
}

#[test]
fn test_{{SKILL_NAME_SNAKE}}_repl_json_output_stays_structured_when_feature_present() {
    if !repl_supported() {
        return;
    }

    let temp_dir = TempDir::new().expect("temp dir");
    let output = run_repl_session(
        &temp_dir,
        Some("json"),
        "run demo-input workspace=demo\nquit\n",
    );

    assert!(output.status.success(), "expected exit 0");

    let stdout = String::from_utf8(output.stdout).expect("non-utf8 output");
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout should be valid JSON");

    assert_eq!(value["input"], "demo-input");
    assert_eq!(value["effective_context"]["workspace"], "demo");
}

#[test]
fn test_{{SKILL_NAME_SNAKE}}_paths_reports_user_scoped_defaults() {
    let output = cmd().arg("paths").output().expect("failed to execute");

    assert!(output.status.success(), "expected exit 0");

    let stdout = String::from_utf8(output.stdout).expect("non-utf8 output");
    let value: serde_yaml::Value =
        serde_yaml::from_str(&stdout).expect("stdout should be valid YAML");

    assert_eq!(value["scope"], "user_scoped_default");
    assert!(!value["config_dir"].as_str().unwrap().is_empty());
}

#[test]
fn test_{{SKILL_NAME_SNAKE}}_context_use_and_show_round_trip() {
    let temp_dir = TempDir::new().expect("temp dir");
    let sandbox = sandbox_args(&temp_dir);

    cmd()
        .args(&sandbox)
        .args([
            "context",
            "use",
            "--selector",
            "workspace=demo",
            "--selector",
            "provider=staging",
        ])
        .assert()
        .success();

    let output = cmd()
        .args(&sandbox)
        .args(["context", "show"])
        .output()
        .expect("failed to execute");

    assert!(output.status.success(), "expected exit 0");

    let stdout = String::from_utf8(output.stdout).expect("non-utf8 output");
    let value: serde_yaml::Value =
        serde_yaml::from_str(&stdout).expect("stdout should be valid YAML");

    assert_eq!(
        value["persisted_context"]["selectors"]["workspace"],
        serde_yaml::Value::from("demo")
    );
    assert_eq!(
        value["effective_context"]["effective_values"]["provider"],
        serde_yaml::Value::from("staging")
    );
}

#[test]
fn test_{{SKILL_NAME_SNAKE}}_explicit_run_override_does_not_mutate_persisted_context() {
    let temp_dir = TempDir::new().expect("temp dir");
    let sandbox = sandbox_args(&temp_dir);

    cmd()
        .args(&sandbox)
        .args([
            "context",
            "use",
            "--selector",
            "workspace=demo",
            "--selector",
            "provider=staging",
        ])
        .assert()
        .success();

    let run_output = cmd()
        .args(&sandbox)
        .args(["run", "demo-input", "--selector", "provider=preview"])
        .output()
        .expect("failed to execute");

    assert!(run_output.status.success(), "expected exit 0");

    let stdout = String::from_utf8(run_output.stdout).expect("non-utf8 output");
    let run_value: serde_yaml::Value =
        serde_yaml::from_str(&stdout).expect("stdout should be valid YAML");
    assert_eq!(
        run_value["effective_context"]["provider"],
        serde_yaml::Value::from("preview")
    );

    let show_output = cmd()
        .args(&sandbox)
        .args(["context", "show"])
        .output()
        .expect("failed to execute");

    assert!(show_output.status.success(), "expected exit 0");

    let show_stdout = String::from_utf8(show_output.stdout).expect("non-utf8 output");
    let show_value: serde_yaml::Value =
        serde_yaml::from_str(&show_stdout).expect("stdout should be valid YAML");
    assert_eq!(
        show_value["effective_context"]["effective_values"]["provider"],
        serde_yaml::Value::from("staging")
    );
}
