use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn reexported_generic_class_sequence_fields() {
    let report = analyze_fixture("reexported_generic_class_sequence_fields");
    let symbols = report
        .findings
        .iter()
        .map(|finding| finding.symbol.clone())
        .collect::<Vec<_>>();

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
