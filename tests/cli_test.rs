use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn telegram_cli() -> Command {
    Command::cargo_bin("telegram-agent-cli").unwrap()
}

fn apply_runtime_env(command: &mut Command, runtime: &TempDir) {
    command.env("TELEGRAM_CLI_CONFIG_DIR", runtime.path().join("config"));
    command.env("TELEGRAM_CLI_DATA_DIR", runtime.path().join("data"));
    command.env("TELEGRAM_CLI_STATE_DIR", runtime.path().join("state"));
    command.env("TELEGRAM_CLI_CACHE_DIR", runtime.path().join("cache"));
}

#[test]
fn top_level_invocation_shows_help_and_exits_zero() {
    telegram_cli()
        .assert()
        .success()
        .stdout(predicate::str::contains("Telegram CLI for automation"));
}

#[test]
fn help_flag_is_plain_text() {
    telegram_cli()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: telegram-agent-cli"))
        .stdout(predicate::str::contains("Commands:"));
}

#[test]
fn short_help_flag_is_plain_text() {
    telegram_cli()
        .arg("-h")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: telegram-agent-cli"));
}

#[test]
fn version_flag_works() {
    telegram_cli()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("telegram-agent-cli"));
}

#[test]
fn structured_help_yaml() {
    telegram_cli()
        .args(["help", "--format", "yaml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("command:"))
        .stdout(predicate::str::contains("summary:"));
}

#[test]
fn structured_help_json() {
    telegram_cli()
        .args(["help", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"command\":"))
        .stdout(predicate::str::contains("\"summary\":"));
}

#[test]
fn structured_help_json_supports_bot_group() {
    telegram_cli()
        .args(["help", "bot", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"command\": \"telegram-agent-cli bot\"",
        ))
        .stdout(predicate::str::contains("\"summary\":"));
}

#[test]
fn structured_help_json_supports_daemon_leaf() {
    telegram_cli()
        .args(["help", "daemon", "status", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"command\": \"telegram-agent-cli daemon status\"",
        ))
        .stdout(predicate::str::contains("\"summary\":"));
}

#[test]
fn repl_help_from_help_subcommand_stays_plain_text() {
    telegram_cli()
        .args(["help", "repl", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: telegram-agent-cli repl"))
        .stdout(predicate::str::contains("\"command\":").not());
}

#[test]
fn structured_help_toml() {
    telegram_cli()
        .args(["help", "--format", "toml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("command = "))
        .stdout(predicate::str::contains("summary = "));
}

#[test]
fn paths_command_returns_runtime_directories() {
    telegram_cli()
        .args(["paths"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config:"))
        .stdout(predicate::str::contains("data:"))
        .stdout(predicate::str::contains("state:"))
        .stdout(predicate::str::contains("cache:"));
}

#[test]
fn context_show_returns_active_context() {
    telegram_cli()
        .args(["context", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("persisted_context:"))
        .stdout(predicate::str::contains("effective_context:"))
        .stdout(predicate::str::contains("mutation_path:"));
}

#[test]
fn missing_required_args_returns_structured_error_yaml() {
    telegram_cli()
        .args(["send"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("status: error"))
        .stderr(predicate::str::contains("code:"))
        .stderr(predicate::str::contains("message:"));
}

#[test]
fn missing_required_args_returns_structured_error_json() {
    telegram_cli()
        .args(["--format", "json", "send"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("\"status\": \"error\""))
        .stderr(predicate::str::contains("\"code\":"))
        .stderr(predicate::str::contains("\"message\":"));
}

#[test]
fn format_switching_json() {
    telegram_cli()
        .args(["--format", "json", "paths"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"config\":"))
        .stdout(predicate::str::contains("\"data\":"));
}

#[test]
fn help_subcommand_lists_subcommands() {
    telegram_cli()
        .args(["help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("account:"))
        .stdout(predicate::str::contains("daemon:"))
        .stdout(predicate::str::contains("message:"))
        .stdout(predicate::str::contains("repl:"));
}

#[test]
fn help_leaf_command_returns_details() {
    telegram_cli()
        .args(["help", "send"])
        .assert()
        .success()
        .stdout(predicate::str::contains("command:"))
        .stdout(predicate::str::contains("options:"));
}

#[test]
fn daemon_lifecycle_is_managed_by_cli() {
    let runtime = tempfile::tempdir().unwrap();
    let metadata_path = runtime.path().join("state/daemon/server.json");

    let mut start = telegram_cli();
    apply_runtime_env(&mut start, &runtime);
    start
        .args(["--format", "json", "daemon", "start"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"state\": \"running\""))
        .stdout(predicate::str::contains("\"running\": true"));
    assert!(
        metadata_path.exists(),
        "daemon metadata should exist after start"
    );

    let mut status = telegram_cli();
    apply_runtime_env(&mut status, &runtime);
    status
        .args(["--format", "json", "daemon", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"state\": \"running\""))
        .stdout(predicate::str::contains("\"running\": true"));

    let mut stop = telegram_cli();
    apply_runtime_env(&mut stop, &runtime);
    stop.args(["--format", "json", "daemon", "stop"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"state\": \"stopped\""))
        .stdout(predicate::str::contains("\"running\": false"));
    assert!(
        !metadata_path.exists(),
        "daemon metadata should be removed after stop"
    );

    let mut stopped_status = telegram_cli();
    apply_runtime_env(&mut stopped_status, &runtime);
    stopped_status
        .args(["--format", "json", "daemon", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"state\": \"stopped\""))
        .stdout(predicate::str::contains("\"running\": false"));
}
