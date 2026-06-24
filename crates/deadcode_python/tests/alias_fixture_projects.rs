use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn callable_alias_return_type_resolves_immediate_method_call() {
    let report = analyze_fixture("callable_alias_return_type_resolves_immediate_method_call");

    assert!(report.diagnostics.is_empty());
    assert_eq!(
        reachable_from(&report, "pkg.client.Client.create_example_item"),
        vec!["weak"]
    );
    assert_eq!(
        reachable_from(&report, "pkg.client.Client._post"),
        vec!["weak"]
    );
    assert_eq!(
        reachable_from(&report, "pkg.client.Client.unused"),
        Vec::<String>::new()
    );
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

fn reachable_from(report: &deadcode_core::AnalysisReport, symbol: &str) -> Vec<String> {
    report
        .findings
        .iter()
        .find(|finding| finding.symbol == symbol)
        .map(|finding| finding.reachable_from.clone())
        .unwrap_or_default()
}
