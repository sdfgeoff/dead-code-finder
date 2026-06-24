use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn model_dump_json_if_expression_is_string() {
    let report = analyze_fixture("model_dump_json_if_expression_is_string");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.GeometryHash.geometry_hash".to_string()));
    assert!(symbols.contains(&"pkg.main.Geometry.unused".to_string()));
}

#[test]
fn model_dump_result_is_mapping() {
    let report = analyze_fixture("model_dump_result_is_mapping");

    assert!(report.diagnostics.is_empty());
}

#[test]
fn local_annotation_alias_expands_for_field_chain() {
    let report = analyze_fixture("local_annotation_alias_expands_for_field_chain");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Box.item".to_string()));
    assert!(symbols.contains(&"pkg.main.Item.unused".to_string()));
}

#[test]
fn date_fromisoformat_result_has_year() {
    let report = analyze_fixture("date_fromisoformat_result_has_year");

    assert!(report.diagnostics.is_empty());
}

#[test]
fn pydantic_model_validate_json_returns_class() {
    let report = analyze_fixture("pydantic_model_validate_json_returns_class");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Args.answer".to_string()));
    assert!(symbols.contains(&"pkg.main.Args.unused".to_string()));
}

#[test]
fn type_adapter_validate_python_returns_generic_arg() {
    let report = analyze_fixture("type_adapter_validate_python_returns_generic_arg");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.models.BaseEvent.source_id".to_string()));
    assert!(symbols.contains(&"pkg.models.BaseEvent.unused_base".to_string()));
}

#[test]
fn list_slice_preserves_collection_type() {
    let report = analyze_fixture("list_slice_preserves_collection_type");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(symbols.contains(&"pkg.main.dead".to_string()));
}

#[test]
fn async_contextmanager_return_type() {
    let report = analyze_fixture("async_contextmanager_return_type");

    assert!(report.diagnostics.is_empty());
}

#[test]
fn init_self_field_coalesced_constructor() {
    let report = analyze_fixture("init_self_field_coalesced_constructor");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Feature.value".to_string()));
    assert!(symbols.contains(&"pkg.main.Feature.unused".to_string()));
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
