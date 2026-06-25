use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn callable_decorator_rule_registers_function() {
    let report = analyze_fixture("callable_decorator_rule_registers_function");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Model.normalize".to_string()));
    assert!(symbols.contains(&"pkg.main.Model.dead".to_string()));
}

#[test]
fn bare_decorator_rule_registers_function() {
    let report = analyze_fixture("bare_decorator_rule_registers_function");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.run_task".to_string()));
    assert!(!symbols.contains(&"pkg.main.Resource.__enter__".to_string()));
    assert!(!symbols.contains(&"pkg.main.Resource.__exit__".to_string()));
    assert!(!symbols.contains(&"pkg.background.WrapperResource.__enter__".to_string()));
    assert!(!symbols.contains(&"pkg.background.WrapperResource.__exit__".to_string()));
    assert!(symbols.contains(&"pkg.main.DeadResource.__enter__".to_string()));
    assert!(symbols.contains(&"pkg.main.DeadResource.__exit__".to_string()));
    assert!(symbols.contains(&"pkg.background.DeadWrapperResource.__enter__".to_string()));
    assert!(symbols.contains(&"pkg.background.DeadWrapperResource.__exit__".to_string()));
}

#[test]
fn decorator_factory_callable_wrapper_object_marks_call_live() {
    let report = analyze_fixture("decorator_factory_callable_wrapper_object");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.cache.CallableCache.__call__".to_string()));
    assert!(!symbols.contains(&"pkg.cache.CallableCache.helper".to_string()));
    assert!(symbols.contains(&"pkg.cache.UnusedCallableCache.__call__".to_string()));
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
