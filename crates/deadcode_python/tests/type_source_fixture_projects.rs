use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn type_source_generic_factory_returns_project_model() {
    let report = analyze_fixture("type_source_generic_factory_returns_project_model");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"app.main.Payload.used".to_string()));
    assert!(symbols.contains(&"app.main.Payload.unused".to_string()));
    assert!(!symbols.iter().any(|symbol| symbol.starts_with("typedlib.")));
}

#[test]
fn stub_type_source_generic_factory() {
    let report = analyze_fixture("stub_type_source_generic_factory");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"app.main.Payload.used".to_string()));
    assert!(symbols.contains(&"app.main.Payload.unused".to_string()));
    assert!(!symbols.iter().any(|symbol| symbol.starts_with("typedlib.")));
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
