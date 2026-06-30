use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn reexported_generic_class_sequence_fields() {
    let report = analyze_fixture("reexported_generic_class_sequence_fields");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.geo.Feature.properties".to_string()));
    assert!(!symbols.contains(&"pkg.main.DumpSource.keep".to_string()));
    assert!(!symbols.contains(&"pkg.main.DumpTarget.keep".to_string()));
    assert!(!symbols.contains(&"pkg.main.DumpTarget.extra".to_string()));
    assert!(!symbols.contains(&"pkg.main.Properties.amount".to_string()));
    assert!(!symbols.contains(&"pkg.main.Stats.amount".to_string()));
    assert!(symbols.contains(&"pkg.geo.Feature.unused".to_string()));
    assert!(symbols.contains(&"pkg.main.DumpSource.skip".to_string()));
    assert!(symbols.contains(&"pkg.main.DumpTarget.skip".to_string()));
    assert!(symbols.contains(&"pkg.main.Properties.unused".to_string()));
    assert!(symbols.contains(&"pkg.main.Stats.unused".to_string()));
}

#[test]
fn type_parameter_from_type_argument_substitutes_inner_model() {
    let report = analyze_fixture("type_parameter_from_type_argument_substitutes_inner_model");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Envelope.item".to_string()));
    assert!(!symbols.contains(&"pkg.main.Payload.used".to_string()));
    assert!(!symbols.contains(&"pkg.main.Payload.parsed_only".to_string()));
    assert!(symbols.contains(&"pkg.main.DeadPayload.dead".to_string()));
}

#[test]
fn cross_module_type_argument_validated_method_return() {
    let report = analyze_fixture("cross_module_type_argument_validated_method_return");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.client.Envelope.item".to_string()));
    assert!(!symbols.contains(&"pkg.main.Payload.used".to_string()));
    assert!(!symbols.contains(&"pkg.main.Payload.parsed_only".to_string()));
    assert!(symbols.contains(&"pkg.main.DeadPayload.dead".to_string()));
}

#[test]
fn imported_typevar_generic_validation_surface() {
    let report = analyze_fixture("imported_typevar_generic_validation_surface");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.container.Envelope.item".to_string()));
    assert!(!symbols.contains(&"pkg.main.Payload.used".to_string()));
    assert!(!symbols.contains(&"pkg.main.Payload.parsed_only".to_string()));
    assert!(symbols.contains(&"pkg.main.DeadPayload.dead".to_string()));
}

#[test]
fn imported_generic_protocol_field_read() {
    let report = analyze_fixture("imported_generic_protocol_field_read");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.protocols.LoaderProtocol.cache_key".to_string()));
    assert!(symbols.contains(&"pkg.protocols.DeadProtocol.cache_key".to_string()));
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
