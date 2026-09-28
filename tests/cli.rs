#![cfg(feature = "cli")]

use std::{fs, path::Path, process::Command};

use tempfile::TempDir;

const OTHER: &str = "package example\nfunc main() {}\n";
const TEST: &str =
    "package example\n\n// Test helper\n// Another comment\nfunc helper() {}\nfunc second() {}\n";

fn project() -> TempDir {
    let root = TempDir::new().unwrap();
    fs::write(root.path().join("main.go"), OTHER).unwrap();
    fs::write(root.path().join("main_test.go"), TEST).unwrap();
    root
}

fn run(root: &Path, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_tokei"))
        .current_dir(root)
        .env("NO_COLOR", "1")
        .args(args)
        .arg(".")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).unwrap()
}

fn row<'a>(output: &'a str, name: &str) -> Vec<&'a str> {
    output
        .lines()
        .map(|line| line.trim_start().strip_prefix("|- ").unwrap_or(line))
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .find(|columns| columns.first() == Some(&name))
        .unwrap_or_else(|| panic!("Missing {name} row in:\n{output}"))
}

#[test]
fn default_output_partitions_tests_before_total() {
    let root = project();
    let output = run(root.path(), &[]);
    assert_eq!(row(&output, "Go"), ["Go"]);
    assert_eq!(row(&output, "Tests"), ["Tests", "1", "6", "3", "2", "1"]);
    assert_eq!(row(&output, "Other"), ["Other", "1", "2", "2", "0", "0"]);
    assert_eq!(row(&output, "(Total)"), ["(Total)", "8", "5", "2", "1"]);
    assert_eq!(row(&output, "Total"), ["Total", "2", "8", "5", "2", "1"]);
    assert!(output.find("Tests").unwrap() < output.find("Other").unwrap());
    assert!(output.find("Other").unwrap() < output.find("(Total)").unwrap());
    assert!(output.lines().any(|line| line.starts_with(" |- Tests")));
    assert!(output.lines().any(|line| line.starts_with(" |- Other")));
    assert!(output.lines().any(|line| line.starts_with(" (Total)")));
}

#[test]
fn compact_output_keeps_inclusive_totals() {
    let root = project();
    let output = run(root.path(), &["--compact"]);
    assert_eq!(row(&output, "Go"), ["Go", "2", "8", "5", "2", "1"]);
    assert!(!output.contains("Tests"));
    assert!(!output.contains("Other"));
}

#[test]
fn file_listing_keeps_default_breakdown() {
    let root = project();
    let output = run(root.path(), &["--files"]);
    assert_eq!(row(&output, "Tests"), ["Tests", "1", "6", "3", "2", "1"]);
    assert!(output.contains("main_test.go"));
    assert!(output.contains("main.go"));
}

#[test]
fn compact_file_listing_hides_breakdown() {
    let root = project();
    let output = run(root.path(), &["--compact", "--files"]);
    assert!(!output.contains("Tests"));
    assert!(!output.contains("Other"));
    assert!(output.contains("main_test.go"));
}

#[test]
fn sorting_keeps_test_breakdown() {
    let root = project();
    for option in ["--sort", "--rsort"] {
        let output = run(root.path(), &[option, "code"]);
        assert_eq!(row(&output, "Tests"), ["Tests", "1", "6", "3", "2", "1"]);
        assert_eq!(row(&output, "Other"), ["Other", "1", "2", "2", "0", "0"]);
    }
}

#[test]
fn all_test_files_omit_other_row() {
    let root = TempDir::new().unwrap();
    fs::write(root.path().join("main_test.go"), TEST).unwrap();
    let output = run(root.path(), &[]);
    assert_eq!(row(&output, "Tests"), ["Tests", "1", "6", "3", "2", "1"]);
    assert!(!output.contains("Other"));
}

#[test]
fn unclassified_files_keep_single_language_row() {
    let root = TempDir::new().unwrap();
    fs::write(root.path().join("main.go"), OTHER).unwrap();
    let output = run(root.path(), &[]);
    assert_eq!(row(&output, "Go"), ["Go", "1", "2", "2", "0", "0"]);
    assert!(!output.contains("Tests"));
    assert!(!output.contains("(Total)"));
}

#[test]
fn streaming_json_identifies_test_files() {
    let root = project();
    let output = run(root.path(), &["--streaming", "json"]);
    let reports: Vec<serde_json::Value> = output
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(reports.len(), 2);
    for report in reports {
        let name = report["stats"]["name"].as_str().unwrap();
        assert_eq!(report["stats"]["is_test"], name.ends_with("main_test.go"));
    }
}
