use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn decorator_boundary_models_mark_nested_pydantic_fields_live() {
    let report = analyze_fixture("decorator_boundary_models_mark_nested_pydantic_fields_live");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"api.main.RequestModel.name".to_string()));
    assert!(!symbols.contains(&"api.main.ResponseModel.item".to_string()));
    assert!(!symbols.contains(&"api.main.ResponseModel.total".to_string()));
    assert!(!symbols.contains(&"api.main.FirstVariant.kind".to_string()));
    assert!(!symbols.contains(&"api.main.FirstVariant.value".to_string()));
    assert!(!symbols.contains(&"api.main.SecondVariant.kind".to_string()));
    assert!(!symbols.contains(&"api.main.SecondVariant.label".to_string()));
    assert!(!symbols.contains(&"api.main.OptionalNested.enabled".to_string()));
    assert!(symbols.contains(&"api.main.NotExposed.field".to_string()));
}

#[test]
fn call_rule_marks_class_argument_member_live() {
    let report = analyze_fixture("call_rule_marks_class_argument_member");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.middleware.LoggedMiddleware.dispatch".to_string()));
    assert!(!symbols.contains(&"pkg.middleware.LoggedMiddleware.record_request".to_string()));
    assert!(symbols.contains(&"pkg.middleware.UnusedMiddleware.dispatch".to_string()));
}

#[test]
fn boundary_enum_parameter_marks_all_members() {
    let report = analyze_fixture("boundary_enum_parameter_marks_all_members");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.examples.UserTagEnum.ROLE_ALPHA".to_string()));
    assert!(!symbols.contains(&"pkg.examples.UserTagEnum.ROLE_BETA".to_string()));
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
