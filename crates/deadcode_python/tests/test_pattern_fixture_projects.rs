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
