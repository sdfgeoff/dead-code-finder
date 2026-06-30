use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn root_groups_report_custom_entrypoint_sets() {
    let report = analyze_fixture("root_groups_report_custom_entrypoint_sets");

    assert!(report.diagnostics.is_empty());
    assert_eq!(
        reachable_from(&report, "pkg.service.scripts_only"),
        vec!["scripts"]
    );
    assert_eq!(
        reachable_from(&report, "pkg.service.tests_only"),
        vec!["tests"]
    );
    assert!(report
        .findings
        .iter()
        .all(|finding| !finding.reachable_from.contains(&"weak".to_string())));
    assert_eq!(
        reachable_from(&report, "pkg.service.completely_dead"),
        Vec::<String>::new()
    );
    assert_eq!(
        reachable_from(&report, "pkg.tests.test_service.dead_test_helper"),
        Vec::<String>::new()
    );
    assert!(!contains_symbol(&report, "pkg.service.production_only"));
    assert!(!contains_symbol(&report, "pkg.service.shared"));
}

#[test]
fn root_groups_can_count_auxiliary_entrypoints_as_used() {
    let report = analyze_fixture("root_groups_count_auxiliary_entrypoints_as_used");

    assert!(report.diagnostics.is_empty());
    assert!(!contains_symbol(&report, "pkg.service.production_only"));
    assert!(!contains_symbol(&report, "pkg.service.scripts_only"));
    assert!(!contains_symbol(&report, "pkg.service.tests_only"));
    assert!(!contains_symbol(&report, "pkg.service.shared"));
    assert_eq!(
        reachable_from(&report, "pkg.service.completely_dead"),
        Vec::<String>::new()
    );
    assert_eq!(
        reachable_from(&report, "pkg.tests.test_service.dead_test_helper"),
        Vec::<String>::new()
    );
}

#[test]
fn allow_comment_roots_function_and_helpers() {
    let report = analyze_fixture("allow_comments_root_function_and_helpers");

    assert!(report.diagnostics.is_empty());
    assert!(!contains_symbol(&report, "pkg.main.api_surface"));
    assert!(!contains_symbol(&report, "pkg.main.helper"));
    assert!(contains_symbol(&report, "pkg.main.dead"));
}

#[test]
fn allow_comment_roots_class_scope() {
    let report = analyze_fixture("allow_comments_root_class_scope");

    assert!(report.diagnostics.is_empty());
    assert!(!contains_symbol(&report, "pkg.main.Client"));
    assert!(!contains_symbol(&report, "pkg.main.Client.value"));
    assert!(!contains_symbol(&report, "pkg.main.Client.endpoint"));
    assert!(!contains_symbol(&report, "pkg.main.Client.other_endpoint"));
    assert!(!contains_symbol(&report, "pkg.main.helper"));
    assert!(!contains_symbol(&report, "pkg.main.UploadFile"));
    assert!(contains_symbol(&report, "pkg.main.DeadClient"));
    assert!(contains_symbol(&report, "pkg.main.DeadClient.value"));
    assert!(contains_symbol(&report, "pkg.main.DeadClient.endpoint"));
}

#[test]
fn allow_comment_roots_file_scope() {
    let report = analyze_fixture("allow_comments_root_file_scope");

    assert!(report.diagnostics.is_empty());
    assert!(report.findings.is_empty());
}

#[test]
fn allow_comment_file_scope_can_filter_by_code() {
    let report = analyze_fixture("allow_comments_file_code_scope");

    assert!(report.diagnostics.is_empty());
    assert!(!contains_symbol(&report, "pkg.main.api_surface"));
    assert!(!contains_symbol(&report, "pkg.main.helper"));
    assert!(contains_symbol(&report, "pkg.main.DeadClient"));
    assert!(contains_symbol(&report, "pkg.main.DeadClient.value"));
    assert!(contains_symbol(&report, "pkg.main.DeadClient.endpoint"));
}

fn analyze_fixture(name: &str) -> deadcode_core::AnalysisReport {
    analyze_project(&AnalyzeOptions::new(
        fixture_root().join(name).join("dead-code-finder.json"),
    ))
    .unwrap()
}

fn reachable_from(report: &deadcode_core::AnalysisReport, symbol: &str) -> Vec<String> {
    report
        .findings
        .iter()
        .find(|finding| finding.symbol == symbol)
        .map(|finding| finding.reachable_from.clone())
        .unwrap_or_default()
}

fn contains_symbol(report: &deadcode_core::AnalysisReport, symbol: &str) -> bool {
    report
        .findings
        .iter()
        .any(|finding| finding.symbol == symbol)
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}
