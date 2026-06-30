use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn structural_protocol_call_argument_marks_implementation_methods_live() {
    let report = analyze_fixture("structural_protocol_call_argument");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.History.get_events".to_string()));
    assert!(!symbols.contains(&"pkg.main.History.append_event".to_string()));
    assert!(symbols.contains(&"pkg.main.UnusedHistory.get_events".to_string()));
    assert!(symbols.contains(&"pkg.main.UnusedHistory.append_event".to_string()));
}

#[test]
fn structural_protocol_tuple_return_marks_implementation_methods_live() {
    let report = analyze_fixture("structural_protocol_tuple_return");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.History.get_events".to_string()));
    assert!(!symbols.contains(&"pkg.main.History.append_event".to_string()));
    assert!(!symbols.contains(&"pkg.main.WrappedHistory.get_events".to_string()));
    assert!(!symbols.contains(&"pkg.main.WrappedHistory.append_event".to_string()));
    assert!(symbols.contains(&"pkg.main.UnusedHistory.get_events".to_string()));
    assert!(symbols.contains(&"pkg.main.UnusedHistory.append_event".to_string()));
}

#[test]
fn lambda_protocol_call_argument_marks_implementation_methods_live() {
    let report = analyze_fixture("lambda_protocol_call_argument");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.ExampleContext.to_context_prompt".to_string()));
    assert!(!symbols.contains(&"pkg.main.History.get_events".to_string()));
    assert!(!symbols.contains(&"pkg.main.History.append_event".to_string()));
    assert!(symbols.contains(&"pkg.main.UnusedHistory.get_events".to_string()));
    assert!(symbols.contains(&"pkg.main.UnusedHistory.append_event".to_string()));
}

#[test]
fn cross_module_lambda_protocol_call_argument_marks_implementation_methods_live() {
    let report = analyze_fixture("lambda_protocol_call_argument_cross_module");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"app.context.ExampleContext.to_context_prompt".to_string()));
    assert!(!symbols.contains(&"app.context.ExampleContext.resource_id".to_string()));
}

#[test]
fn protocol_concrete_flow_through_forwarded_call_marks_implementation_methods_live() {
    let report = analyze_fixture("protocol_concrete_flow_through_forwarded_call");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.ExampleContext.to_context_prompt".to_string()));
    assert!(symbols.contains(&"pkg.main.UnusedContext.to_context_prompt".to_string()));
}

#[test]
fn protocol_default_parameter_concrete_flow_marks_implementation_methods_live() {
    let report = analyze_fixture("protocol_default_parameter_concrete_flow");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.ExampleContext.to_context_prompt".to_string()));
    assert!(!symbols.contains(&"pkg.main.ExampleContext.resource_id".to_string()));
    assert!(symbols.contains(&"pkg.main.UnusedContext.to_context_prompt".to_string()));
}

#[test]
fn protocol_imported_default_parameter_concrete_flow_marks_implementation_methods_live() {
    let report = analyze_fixture("protocol_imported_default_parameter_concrete_flow");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.context.ExampleContext.to_context_prompt".to_string()));
    assert!(!symbols.contains(&"pkg.context.ExampleContext.resource_id".to_string()));
    assert!(symbols.contains(&"pkg.context.UnusedContext.to_context_prompt".to_string()));
}

#[test]
fn base_typed_field_forwards_virtual_methods() {
    let report = analyze_fixture("base_typed_field_forwards_virtual_methods");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.TrackedCacheConnection.get_item".to_string()));
    assert!(!symbols.contains(&"pkg.main.TrackedCacheConnection.set_item".to_string()));
    assert!(!symbols.contains(&"pkg.main.MemoryCacheConnection.get_item".to_string()));
    assert!(!symbols.contains(&"pkg.main.MemoryCacheConnection.set_item".to_string()));
    assert!(symbols.contains(&"pkg.main.UnusedCacheConnection.get_item".to_string()));
    assert!(symbols.contains(&"pkg.main.UnusedCacheConnection.set_item".to_string()));
}

#[test]
fn protocol_constructor_field_flow_from_pytest_fixture_marks_fake_methods_live() {
    let report = analyze_fixture("protocol_constructor_field_flow_from_pytest_fixture");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.tests.test_service.FakeMessageSource.read_message".to_string()));
    assert!(!symbols.contains(&"pkg.tests.test_service.FakeMessageSink.write_message".to_string()));
    assert!(!symbols.contains(&"pkg.tests.test_service.FakeSinkProvider.open_sink".to_string()));
    assert!(symbols
        .contains(&"pkg.tests.test_service.FakeMessageSource.unused_source_helper".to_string()));
    assert!(
        symbols.contains(&"pkg.tests.test_service.FakeMessageSink.unused_sink_helper".to_string())
    );
    assert!(!symbols.contains(&"pkg.tests.test_service.FakeProviderSink.write_message".to_string()));
    assert!(symbols.contains(
        &"pkg.tests.test_service.FakeProviderSink.unused_provider_sink_helper".to_string()
    ));
    assert!(symbols
        .contains(&"pkg.tests.test_service.FakeSinkProvider.unused_provider_helper".to_string()));
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
