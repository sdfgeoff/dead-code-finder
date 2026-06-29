use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn model_dump_json_if_expression_is_string() {
    let report = analyze_fixture("model_dump_json_if_expression_is_string");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.GeometryHash.geometry_hash".to_string()));
    assert!(!symbols.contains(&"pkg.main.Geometry.unused".to_string()));
}

#[test]
fn model_dump_result_is_mapping() {
    let report = analyze_fixture("model_dump_result_is_mapping");

    assert!(report.diagnostics.is_empty());
}

#[test]
fn model_dump_marks_nested_default_fields_live() {
    let report = analyze_fixture("model_dump_marks_nested_default_fields_live");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Delivery.copy".to_string()));
    assert!(!symbols.contains(&"pkg.main.Delivery.blind_copy".to_string()));
    assert!(!symbols.contains(&"pkg.main.Content.charset".to_string()));
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
    assert!(!symbols.contains(&"pkg.main.Args.unused".to_string()));
    assert!(!symbols.contains(&"pkg.main.Args.details".to_string()));
    assert!(!symbols.contains(&"pkg.main.Details.status".to_string()));
    assert!(!symbols.contains(&"pkg.main.Details.nested_value".to_string()));
    assert!(!symbols.contains(&"pkg.main.Status.READY".to_string()));
    assert!(!symbols.contains(&"pkg.main.Status.WAITING".to_string()));
}

#[test]
fn live_pydantic_model_marks_model_config() {
    let report = analyze_fixture("live_pydantic_model_marks_model_config");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.LiveModel.model_config".to_string()));
    assert!(symbols.contains(&"pkg.main.DeadModel.model_config".to_string()));
}

#[test]
fn live_class_marks_slots_metadata() {
    let report = analyze_fixture("live_class_marks_slots_metadata");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.LiveBucket.__slots__".to_string()));
    assert!(symbols.contains(&"pkg.main.DeadBucket.__slots__".to_string()));
}

#[test]
fn type_adapter_validate_python_returns_generic_arg() {
    let report = analyze_fixture("type_adapter_validate_python_returns_generic_arg");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.models.BaseEvent.source_id".to_string()));
    assert!(!symbols.contains(&"pkg.models.BaseEvent.unused_base".to_string()));
    assert!(!symbols.contains(&"pkg.models.ExternalPayload.parsed_only".to_string()));
    assert!(symbols.contains(&"pkg.models.DeadPayload.dead_external".to_string()));
}

#[test]
fn type_adapter_validation_marks_default_and_nested_fields() {
    let report = analyze_fixture("type_adapter_validation_marks_default_and_nested_fields");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.models.Payload.name".to_string()));
    assert!(!symbols.contains(&"pkg.models.Payload.s3".to_string()));
    assert!(!symbols.contains(&"pkg.models.Feature.type".to_string()));
    assert!(!symbols.contains(&"pkg.models.Feature.id".to_string()));
    assert!(!symbols.contains(&"pkg.models.Feature.properties".to_string()));
    assert!(!symbols.contains(&"pkg.models.Feature.geometry".to_string()));
    assert!(!symbols.contains(&"pkg.models.FeatureCollection.type".to_string()));
    assert!(!symbols.contains(&"pkg.models.FeatureCollection.features".to_string()));
    assert!(symbols.contains(&"pkg.models.UnusedPayload.dead".to_string()));
}

#[test]
fn typed_dict_type_adapter_literal_keys() {
    let report = analyze_fixture("typed_dict_type_adapter_literal_keys");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.BrokerMessage.kind".to_string()));
    assert!(!symbols.contains(&"pkg.main.BrokerMessage.payload".to_string()));
    assert!(symbols.contains(&"pkg.main.BrokerMessage.unused".to_string()));
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

#[test]
fn union_subscript_uses_getitem_return_type() {
    let report = analyze_fixture("union_subscript_uses_getitem_return_type");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.CompatWorkbook.__getitem__".to_string()));
    assert!(!symbols.contains(&"pkg.main.CompatSheet.cell".to_string()));
    assert!(!symbols.contains(&"pkg.main.CompatCell.value".to_string()));
}

#[test]
fn reexported_enum_iteration_marks_members() {
    let report = analyze_fixture("reexported_enum_iteration_marks_members");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.models.examples.ExampleRole.SUBMITTER".to_string()));
    assert!(!symbols.contains(&"pkg.models.examples.ExampleRole.CONTACT".to_string()));
    assert!(!symbols.contains(&"pkg.models.examples.ExampleRole.EDITOR".to_string()));
    assert!(!symbols.contains(&"pkg.models.examples.ExampleRole.TASK_EDITOR".to_string()));
    assert!(!symbols.contains(&"pkg.models.examples.ExampleRole.INVOICE_RECIPIENT".to_string()));
    assert!(!symbols.contains(&"pkg.models.examples.ExampleRole.ACCOUNT_HOLDER".to_string()));
}

#[test]
fn imported_annotated_union_alias_field_read_marks_member_fields() {
    let report = analyze_fixture("imported_annotated_union_alias_field_read");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.events.FirstEvent.event_type".to_string()));
    assert!(!symbols.contains(&"pkg.events.SecondEvent.event_type".to_string()));
    assert!(symbols.contains(&"pkg.events.FirstEvent.dead_first".to_string()));
    assert!(symbols.contains(&"pkg.events.SecondEvent.payload".to_string()));
    assert!(symbols.contains(&"pkg.events.SecondEvent.dead_second".to_string()));
}

#[test]
fn live_subclass_marks_base_init_subclass_live() {
    let report = analyze_fixture("live_subclass_marks_base_init_subclass_live");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Base.__init_subclass__".to_string()));
    assert!(symbols.contains(&"pkg.main.DeadBase.__init_subclass__".to_string()));
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
