//! Proves `mensung version` and `mensung check-update` work against the
//! real compiled binary, and specifically that neither one needs (or
//! triggers an install of) the medication database: running either in a
//! directory with no `medical_database.men` must not print the "no
//! database found" prompt `data.rs` shows for every other mode.

use std::process::Command;

#[test]
fn version_prints_the_crate_version_and_needs_no_database() {
    let dir = std::env::temp_dir().join("mensung-version-no-db-test");
    std::fs::create_dir_all(&dir).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mensung"))
        .env("MENSUNG_DATA_DIR", &dir)
        .args(["version"])
        .output()
        .expect("running the real mensung binary should succeed");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        format!("mensung {}", env!("CARGO_PKG_VERSION"))
    );
    assert!(
        !stdout.contains("No medication database found"),
        "version should never trigger the dataset install prompt, stdout was:\n{stdout}"
    );
    assert!(!dir.join("medical_database.men").exists());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
#[ignore = "hits the live GitHub API"]
fn check_update_reaches_the_real_github_api_and_needs_no_database() {
    let dir = std::env::temp_dir().join("mensung-check-update-no-db-test");
    std::fs::create_dir_all(&dir).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mensung"))
        .env("MENSUNG_DATA_DIR", &dir)
        .args(["check-update"])
        .output()
        .expect("running the real mensung binary should succeed");

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Checking for updates"),
        "stdout was:\n{stdout}"
    );
    // Whether it reports "up to date" or "a new version is available"
    // depends on the real, live release state of the repository, which
    // this test does not control and must not assume; either outcome
    // proves the real request round-tripped successfully.
    assert!(
        stdout.contains("You are running the latest version")
            || stdout.contains("A new version is available"),
        "stdout was:\n{stdout}"
    );
    assert!(
        !stdout.contains("No medication database found"),
        "check-update should never trigger the dataset install prompt, stdout was:\n{stdout}"
    );
    assert!(!dir.join("medical_database.men").exists());

    std::fs::remove_dir_all(&dir).ok();
}
