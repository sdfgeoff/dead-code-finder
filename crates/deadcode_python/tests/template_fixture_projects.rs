use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn union_template_alias_field_read() {
    let report = analyze_fixture("union_template_alias_field_read");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.first.FirstTemplate.template_type".to_string()));
    assert!(!symbols.contains(&"pkg.first.FirstTemplate.label".to_string()));
    assert!(symbols.contains(&"pkg.first.FirstTemplate.unused".to_string()));
    assert!(!symbols.contains(&"pkg.second.SecondTemplate.template_type".to_string()));
    assert!(symbols.contains(&"pkg.second.SecondTemplate.body".to_string()));
    assert!(!symbols.contains(&"pkg.third.ThirdTemplate.template_type".to_string()));
    assert!(symbols.contains(&"pkg.third.ThirdTemplate.detail".to_string()));
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
