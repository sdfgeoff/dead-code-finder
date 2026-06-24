use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn starred_list_comprehension_references_are_traversed() {
    let report = analyze_fixture("starred_list_comprehension_references");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.used".to_string()));
    assert!(symbols.contains(&"pkg.main.dead".to_string()));
}

#[test]
fn registry_stored_subtype_method_liveness() {
    let report = analyze_fixture("registry_stored_subtype_method_liveness");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Tool.execute".to_string()));
    assert!(!symbols.contains(&"pkg.main.LiveTool.name".to_string()));
    assert!(!symbols.contains(&"pkg.main.LiveTool.execute".to_string()));
    assert!(symbols.contains(&"pkg.main.DeadTool.name".to_string()));
    assert!(symbols.contains(&"pkg.main.DeadTool.execute".to_string()));
}

#[test]
fn generator_expression_references_are_traversed() {
    let report = analyze_fixture("generator_expression_references");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.to_items".to_string()));
    assert!(!symbols.contains(&"pkg.main.score".to_string()));
    assert!(!symbols.contains(&"pkg.main.Item.value".to_string()));
    assert!(symbols.contains(&"pkg.main.dead".to_string()));
}

#[test]
fn async_statements_traverse_references() {
    let report = analyze_fixture("async_statements_traverse_references");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.ExampleContext.to_context_prompt".to_string()));
    assert!(!symbols.contains(&"pkg.main.Client.send".to_string()));
    assert!(symbols.contains(&"pkg.main.Client.__aenter__".to_string()));
    assert!(symbols.contains(&"pkg.main.Client.__aexit__".to_string()));
    assert!(symbols.contains(&"pkg.main.UnusedContext.to_context_prompt".to_string()));
    assert!(symbols.contains(&"pkg.main.UnusedClient.send".to_string()));
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
