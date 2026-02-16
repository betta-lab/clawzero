use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

fn clawzero() -> assert_cmd::Command {
    cargo_bin_cmd!("clawzero")
}

// --- CLI basic tests (no LLM call) ---

#[test]
fn help_shows_usage() {
    clawzero()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Ultra-fast AI agent CLI"));
}

#[test]
fn version_flag() {
    clawzero()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("clawzero"));
}

#[test]
fn config_subcommand() {
    clawzero()
        .arg("config")
        .assert()
        .success()
        .stdout(predicate::str::contains("Default model:"))
        .stdout(predicate::str::contains("Providers:"));
}

#[test]
fn invalid_model_spec_errors() {
    clawzero()
        .args(["--model", "no-slash", "hello"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid model spec"));
}

#[test]
fn unknown_provider_errors() {
    clawzero()
        .args(["--model", "nonexistent/model", "hello"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown provider"));
}

#[test]
fn init_subcommand_in_help() {
    clawzero()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("init"));
}

// --- LLM integration tests ---

#[test]
fn oneshot_simple_question() {
    clawzero()
        .args(["What is 2+2? Reply with just the number."])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains("4"));
}

#[test]
fn oneshot_nonempty_response() {
    clawzero()
        .args(["Say exactly: hello world"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains("hello").or(predicate::str::contains("Hello")));
}

#[test]
fn oneshot_tool_use_bash() {
    clawzero()
        .args(["Run `echo clawzero_test_ok` using bash and tell me the output."])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        .stdout(predicate::str::contains("clawzero_test_ok"));
}

#[test]
fn oneshot_tool_use_file_read() {
    clawzero()
        .args(["Read the file Cargo.toml and tell me the package name."])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        .stdout(predicate::str::contains("clawzero"));
}

#[test]
fn oneshot_multiword_prompt() {
    clawzero()
        .args(["List", "three", "colors.", "Reply", "in", "one", "line."])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}
