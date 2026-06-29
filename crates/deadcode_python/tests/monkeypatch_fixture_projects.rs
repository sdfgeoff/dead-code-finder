use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn monkeypatch_callable_return_overrides_mark_only_used_fake_methods_live() {
    let report = analyze_fixture("monkeypatch_callable_return_override_flow");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"app.tests.test_runtime.FakeClient.set".to_string()));
    assert!(!symbols.contains(&"app.tests.test_runtime.FakeClient.get".to_string()));
    assert!(!symbols.contains(&"app.tests.test_runtime.FakeClient.publish".to_string()));
    assert!(symbols.contains(&"app.tests.test_runtime.FakeClient.unused".to_string()));
}

fn analyze_fixture(name: &str) -> deadcode_core::AnalysisReport {
    let root = fixture_root(name);
    analyze_project(&AnalyzeOptions::new(root.join("dead-code-finder.json")))
        .unwrap_or_else(|error| panic!("fixture at {} failed: {error}", root.display()))
}

fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn finding_symbols(report: &deadcode_core::AnalysisReport) -> Vec<String> {
    report
        .findings
        .iter()
        .map(|finding| finding.symbol.clone())
        .collect()
}
