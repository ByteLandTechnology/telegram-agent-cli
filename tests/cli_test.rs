use assert_cmd::Command;
use predicates::prelude::*;

fn telegram_cli() -> Command {
    Command::cargo_bin("telegram-agent-cli").unwrap()
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
