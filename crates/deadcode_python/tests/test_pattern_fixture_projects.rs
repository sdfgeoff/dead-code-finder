use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn default_test_patterns_exclude_test_helper_modules() {
    let report = analyze_fixture("default_test_patterns_exclude_test_helper_modules");
    let symbols = report
        .findings
        .iter()
        .map(|finding| finding.symbol.clone())
        .collect::<Vec<_>>();

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.service_test_helpers.Helper".to_string()));
    assert!(!symbols.contains(&"pkg.service_test_helpers.Helper.value".to_string()));
    assert!(!symbols.contains(&"pkg.service_test_helpers.make_helper".to_string()));
    assert!(symbols.contains(&"pkg.service.production_dead".to_string()));
}

#[test]
fn pytest_test_classes_are_test_roots() {
    let report = analyze_fixture("include_tests_reports_dead_helpers_inside_test_files");
    let symbols = report
        .findings
        .iter()
        .map(|finding| finding.symbol.clone())
        .collect::<Vec<_>>();

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.tests.test_service.TestCollected".to_string()));
    assert!(!symbols.contains(&"pkg.tests.test_service.TestCollected.test_method".to_string()));
    assert!(symbols.contains(&"pkg.tests.test_service.TestCollected.helper_method".to_string()));
    assert!(symbols.contains(&"pkg.tests.test_service.HelperClass".to_string()));
    assert!(symbols.contains(
        &"pkg.tests.test_service.HelperClass.test_like_method_on_non_test_class".to_string()
    ));
}

fn analyze_fixture(name: &str) -> deadcode_core::AnalysisReport {
    analyze_project(&AnalyzeOptions {
        config_path: fixture_path(name).join("dead-code-finder.json"),
    })
    .unwrap()
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}
