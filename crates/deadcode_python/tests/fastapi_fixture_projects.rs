use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
#[ignore = "documents current FastAPI router reachability bug: registering a route on an unused router should not make the endpoint live"]
fn unused_router_does_not_make_registered_endpoints_live() {
    let report = analyze_fixture("fastapi_unused_router_does_not_make_registered_endpoints_live");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"api.live_routes.live_endpoint".to_string()));
    assert!(!symbols.contains(&"api.live_routes.live_helper".to_string()));
    assert!(symbols.contains(&"api.unused_routes.unused_endpoint".to_string()));
    assert!(symbols.contains(&"api.unused_routes.unused_helper".to_string()));
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
