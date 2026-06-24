use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn string_forward_ref_annotations_resolve_receiver_fields() {
    let report = analyze_fixture("string_forward_ref_annotations");
    let symbols = report
        .findings
        .iter()
        .map(|finding| finding.symbol.clone())
        .collect::<Vec<_>>();

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Bound.minx".to_string()));
    assert!(!symbols.contains(&"pkg.main.Bound.maxx".to_string()));
    assert!(symbols.contains(&"pkg.main.Bound.unused".to_string()));
}

#[test]
fn imported_external_type_alias_suppresses_receiver_warnings() {
    let report = analyze_fixture("imported_external_type_alias_suppresses_receiver");
    let symbols = report
        .findings
        .iter()
        .map(|finding| finding.symbol.clone())
        .collect::<Vec<_>>();

    assert!(report.diagnostics.is_empty());
    assert!(symbols.contains(&"pkg.main.Props.value".to_string()));
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
