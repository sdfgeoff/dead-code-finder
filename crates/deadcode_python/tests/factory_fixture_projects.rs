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
