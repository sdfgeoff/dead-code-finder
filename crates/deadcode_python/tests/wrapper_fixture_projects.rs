use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn base_typed_factory_wrapper_concrete_flow() {
    let report = analyze_fixture("base_typed_factory_wrapper_concrete_flow");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.providers.WrapperConnection.lookup".to_string()));
    assert!(!symbols.contains(&"pkg.providers.NetworkConnection.lookup".to_string()));
    assert!(!symbols.contains(&"pkg.providers.MemoryConnection.lookup".to_string()));
    assert!(symbols.contains(&"pkg.providers.WrapperConnection.write".to_string()));
    assert!(symbols.contains(&"pkg.providers.NetworkConnection.write".to_string()));
    assert!(symbols.contains(&"pkg.providers.MemoryConnection.write".to_string()));
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
