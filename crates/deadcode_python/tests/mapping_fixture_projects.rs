use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn dict_values_and_fstring_flow_resolve_fields_and_methods() {
    let report = analyze_fixture("dict_values_and_fstring_flow");
    let symbols = report
        .findings
        .iter()
        .map(|finding| finding.symbol.clone())
        .collect::<Vec<_>>();

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Registered.name".to_string()));
    assert!(!symbols.contains(&"pkg.main.Registered.max_calls".to_string()));
    assert!(symbols.contains(&"pkg.main.Registered.unused".to_string()));
}

#[test]
fn dict_comprehension_tuple_target_items() {
    let report = analyze_fixture("dict_comprehension_tuple_target_items");
    let symbols = report
        .findings
        .iter()
        .map(|finding| finding.symbol.clone())
        .collect::<Vec<_>>();

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Source.live".to_string()));
    assert!(symbols.contains(&"pkg.main.Source.unused".to_string()));
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
