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
fn attribute_union_branch_narrowing() {
    let report = analyze_fixture("attribute_union_branch_narrowing");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.RegionalStock.region".to_string()));
    assert!(!symbols.contains(&"pkg.main.RegionalStock.values".to_string()));
    assert!(!symbols.contains(&"pkg.main.GlobalStock.values".to_string()));
    assert!(symbols.contains(&"pkg.main.RegionalStock.unused".to_string()));
    assert!(symbols.contains(&"pkg.main.GlobalStock.unused".to_string()));
    assert!(symbols.contains(&"pkg.main.Species.unused".to_string()));
}

#[test]
fn branch_assignment_merge_after_early_return() {
    let report = analyze_fixture("branch_assignment_merge_after_early_return");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.ExampleItem.example_item_id".to_string()));
    assert!(!symbols.contains(&"pkg.main.ExampleItem.version_id".to_string()));
    assert!(symbols.contains(&"pkg.main.ExampleItem.unused".to_string()));
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

#[test]
fn local_call_list_comprehension_result_type() {
    let report = analyze_fixture("local_call_list_comprehension_result_type");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.ExampleInput.category".to_string()));
    assert!(!symbols.contains(&"pkg.main.ExampleInput.amount".to_string()));
    assert!(!symbols.contains(&"pkg.main.DimsAndMeasures.dimensions".to_string()));
    assert!(!symbols.contains(&"pkg.main.Dimensions.category".to_string()));
    assert!(symbols.contains(&"pkg.main.DimsAndMeasures.unused".to_string()));
    assert!(symbols.contains(&"pkg.main.Dimensions.unused".to_string()));
}

#[test]
fn local_call_list_literal_item_type() {
    let report = analyze_fixture("local_call_list_literal_item_type");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Feature.properties".to_string()));
    assert!(!symbols.contains(&"pkg.main.Feature.geometry".to_string()));
    assert!(!symbols.contains(&"pkg.main.Properties.label".to_string()));
    assert!(symbols.contains(&"pkg.main.Feature.unused".to_string()));
    assert!(symbols.contains(&"pkg.main.Properties.unused".to_string()));
}

#[test]
fn empty_list_append_infers_list_type() {
    let report = analyze_fixture("empty_list_append_infers_list_type");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.ExampleItem.name".to_string()));
    assert!(!symbols.contains(&"pkg.main.ExampleItem.tile_url".to_string()));
    assert!(symbols.contains(&"pkg.main.ExampleItem.unused".to_string()));
}

#[test]
fn cast_ifexp_list_comprehension_item_type() {
    let report = analyze_fixture("cast_ifexp_list_comprehension_item_type");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Polygon.bounds".to_string()));
    assert!(symbols.contains(&"pkg.main.Polygon.unused".to_string()));
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
