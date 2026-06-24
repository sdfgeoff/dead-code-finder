use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn structural_protocol_call_argument_marks_implementation_methods_live() {
    let report = analyze_fixture("structural_protocol_call_argument");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.History.get_events".to_string()));
    assert!(!symbols.contains(&"pkg.main.History.append_event".to_string()));
    assert!(symbols.contains(&"pkg.main.UnusedHistory.get_events".to_string()));
    assert!(symbols.contains(&"pkg.main.UnusedHistory.append_event".to_string()));
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

fn finding_symbols(report: &deadcode_core::AnalysisReport) -> Vec<String> {
    report
        .findings
        .iter()
        .map(|finding| finding.symbol.clone())
        .collect()
}
