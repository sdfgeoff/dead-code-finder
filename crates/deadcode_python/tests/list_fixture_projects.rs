use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn list_comprehension_preserves_item_type() {
    let report = analyze_fixture("list_comprehension_preserves_item_type");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.PlantingEvent.event_type".to_string()));
    assert!(!symbols.contains(&"pkg.main.PlantingEvent.ets_species".to_string()));
    assert!(!symbols.contains(&"pkg.main.PlantingEvent.species".to_string()));
    assert!(symbols.contains(&"pkg.main.PlantingEvent.unused".to_string()));
    assert!(symbols.contains(&"pkg.main.ClearanceEvent.unused".to_string()));
}

#[test]
fn isinstance_list_comprehension_narrows_union_item() {
    let report = analyze_fixture("isinstance_list_comprehension_narrows_union_item");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.CacheMiss.key".to_string()));
    assert!(!symbols.contains(&"pkg.main.CacheMiss.input".to_string()));
    assert!(!symbols.contains(&"pkg.main.Input.value".to_string()));
    assert!(symbols.contains(&"pkg.main.CacheMiss.unused".to_string()));
    assert!(symbols.contains(&"pkg.main.CacheHit.unused".to_string()));
    assert!(symbols.contains(&"pkg.main.Input.unused".to_string()));
}

#[test]
fn max_call_preserves_iterable_item_type() {
    let report = analyze_fixture("max_call_preserves_iterable_item_type");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Properties.amount".to_string()));
    assert!(!symbols.contains(&"pkg.main.Properties.report_category".to_string()));
    assert!(!symbols.contains(&"pkg.main.Properties.credit_scheme".to_string()));
    assert!(symbols.contains(&"pkg.main.Properties.unused".to_string()));
}

#[test]
fn class_field_alias_expands_for_max_item() {
    let report = analyze_fixture("class_field_alias_expands_for_max_item");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Properties.amount".to_string()));
    assert!(!symbols.contains(&"pkg.main.Properties.report_category".to_string()));
    assert!(!symbols.contains(&"pkg.main.Properties.credit_scheme".to_string()));
    assert!(symbols.contains(&"pkg.main.Properties.unused".to_string()));
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
