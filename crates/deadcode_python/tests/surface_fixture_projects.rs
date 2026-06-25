use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn class_surface_rule_marks_attributes() {
    let report = analyze_fixture("class_surface_rule_marks_attributes");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.LiveModel.__tablename__".to_string()));
    assert!(!symbols.contains(&"pkg.main.LiveModel.id".to_string()));
    assert!(!symbols.contains(&"pkg.main.LiveModel.used".to_string()));
    assert!(symbols.contains(&"pkg.main.DeadModel".to_string()));
    assert!(symbols.contains(&"pkg.main.DeadModel.__tablename__".to_string()));
    assert!(symbols.contains(&"pkg.main.DeadModel.id".to_string()));
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
