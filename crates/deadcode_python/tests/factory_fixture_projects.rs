use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn declarative_factory_callable_return_from_keyword() {
    let report = analyze_fixture("declarative_factory_callable_return_from_keyword");
    let symbols = report
        .findings
        .iter()
        .map(|finding| finding.symbol.clone())
        .collect::<Vec<_>>();

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.db.TitleRow.field_code".to_string()));
    assert!(!symbols.contains(&"pkg.db.TitleRow.properties".to_string()));
    assert!(!symbols.contains(&"pkg.db.TitleProperties.owners".to_string()));
    assert!(symbols.contains(&"pkg.db.TitleRow.unused".to_string()));
    assert!(symbols.contains(&"pkg.db.TitleProperties.unused".to_string()));
}

#[test]
fn factory_model_surface_fields_marked_live() {
    let report = analyze_fixture("factory_model_surface_fields_marked_live");
    let symbols = report
        .findings
        .iter()
        .map(|finding| finding.symbol.clone())
        .collect::<Vec<_>>();

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.db.InputRow.user_id".to_string()));
    assert!(!symbols.contains(&"pkg.db.InputRow.tenant_id".to_string()));
    assert!(!symbols.contains(&"pkg.db.OutputRow.id".to_string()));
    assert!(!symbols.contains(&"pkg.db.OutputRow.name".to_string()));
    assert!(!symbols.contains(&"pkg.db.OutputRow.extra".to_string()));
    assert!(!symbols.contains(&"pkg.db.FirstEvent.kind".to_string()));
    assert!(!symbols.contains(&"pkg.db.FirstEvent.payload".to_string()));
    assert!(!symbols.contains(&"pkg.db.SecondEvent.kind".to_string()));
    assert!(!symbols.contains(&"pkg.db.SecondEvent.code".to_string()));
    assert!(!symbols.contains(&"pkg.db.PositionalInput.user_id".to_string()));
    assert!(!symbols.contains(&"pkg.db.PositionalInput.record_id".to_string()));
    assert!(!symbols.contains(&"pkg.db.PositionalOutput.user_id".to_string()));
    assert!(!symbols.contains(&"pkg.db.PositionalOutput.group_id".to_string()));
    assert!(!symbols.contains(&"pkg.db.PositionalOutput.record_id".to_string()));
    assert!(symbols.contains(&"pkg.db.DeadRow.value".to_string()));
}

#[test]
fn generic_callable_factory_surfaces_without_rules() {
    let report = analyze_fixture("generic_callable_factory_surfaces_without_rules");
    let symbols = report
        .findings
        .iter()
        .map(|finding| finding.symbol.clone())
        .collect::<Vec<_>>();

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.db.InputRow.required".to_string()));
    assert!(!symbols.contains(&"pkg.db.InputRow.serialized_only".to_string()));
    assert!(!symbols.contains(&"pkg.db.OutputRow.id".to_string()));
    assert!(!symbols.contains(&"pkg.db.OutputRow.constructed_only".to_string()));
    assert!(!symbols.contains(&"pkg.db.BatchInput.item_id".to_string()));
    assert!(!symbols.contains(&"pkg.db.BatchInput.payload".to_string()));
    assert!(symbols.contains(&"pkg.db.DeadRow.value".to_string()));
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
