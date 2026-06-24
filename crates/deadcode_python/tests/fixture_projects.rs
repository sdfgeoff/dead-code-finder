use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn multi_root_workspace_resolves_cross_root_usage() {
    let report = analyze_fixture("multi_root_workspace_resolves_cross_root_usage");
    let symbols = finding_symbols(&report);

    assert!(!symbols.contains(&"shared.service.live".to_string()));
    assert!(symbols.contains(&"shared.service.dead".to_string()));
}

#[test]
fn package_glob_roots_expand_for_workspace_packages() {
    let report = analyze_fixture("package_glob_roots_expand_for_workspace_packages");
    let symbols = finding_symbols(&report);

    assert!(!symbols.contains(&"two.lib.live".to_string()));
    assert!(symbols.contains(&"two.lib.dead".to_string()));
}

#[test]
fn fastapi_style_rules_cover_routes_dependencies_and_models() {
    let report = analyze_fixture("fastapi_style_rules_cover_routes_dependencies_and_models");
    let symbols = finding_symbols(&report);

    assert!(!symbols.contains(&"api.main.list_users".to_string()));
    assert!(!symbols.contains(&"api.main.get_user".to_string()));
    assert!(!symbols.contains(&"api.main.ExampleEntity.name".to_string()));
    assert!(symbols.contains(&"api.main.ExampleEntity.age".to_string()));
    assert!(symbols.contains(&"api.main.unused_dependency".to_string()));
}

#[test]
fn fastapi_multi_module_fixture_covers_app_shape() {
    let report = analyze_fixture("fastapi_multi_module_fixture_covers_app_shape");
    let symbols = finding_symbols(&report);

    assert!(!symbols.contains(&"api.routes.entities.read_user".to_string()));
    assert!(!symbols.contains(&"api.dependencies.get_current_user".to_string()));
    assert!(!symbols.contains(&"api.models.ExampleEntity.name".to_string()));
    assert!(symbols.contains(&"api.dependencies.unused_dependency".to_string()));
    assert!(symbols.contains(&"api.models.ExampleEntity.field_text".to_string()));
    assert!(symbols.contains(&"api.routes.admin.admin_dashboard".to_string()));
}

#[test]
fn route_glob_rules_activate_dynamic_route_modules() {
    let report = analyze_fixture("route_glob_rules_activate_dynamic_route_modules");
    let symbols = finding_symbols(&report);

    assert!(!symbols.contains(&"api.entities.route.list_users".to_string()));
}

#[test]
fn context_managers_and_weak_scripts_are_tracked_separately() {
    let report = analyze_fixture("context_managers_and_weak_scripts");
    let symbols = finding_symbols(&report);
    let script_only = report
        .findings
        .iter()
        .find(|finding| finding.symbol == "pkg.service.script_only")
        .unwrap();

    assert!(!symbols.contains(&"pkg.resources.Resource.__enter__".to_string()));
    assert!(!symbols.contains(&"pkg.resources.Resource.__exit__".to_string()));
    assert!(symbols.contains(&"pkg.resources.UnusedResource.__enter__".to_string()));
    assert!(symbols.contains(&"pkg.resources.UnusedResource.__exit__".to_string()));
    assert_eq!(script_only.reachable_from, vec!["weak".to_string()]);
    assert!(symbols.contains(&"pkg.service.dead".to_string()));
}

#[test]
fn external_type_flows_suppress_unresolved_receivers() {
    let report = analyze_fixture("external_type_flows_suppress_unresolved_receivers");

    assert!(report.diagnostics.is_empty());
}

#[test]
fn generic_collection_iteration_resolves_feature_fields() {
    let report = analyze_fixture("generic_collection_iteration_resolves_feature_fields");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.models.Feature.geometry".to_string()));
    assert!(!symbols.contains(&"pkg.models.Feature.properties".to_string()));
    assert!(symbols.contains(&"pkg.models.Feature.unused".to_string()));
}

#[test]
fn local_return_annotations_resolve_call_results() {
    let report = analyze_fixture("local_return_annotations_resolve_call_results");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.client.Response.id".to_string()));
    assert!(!symbols.contains(&"pkg.client.Response.created".to_string()));
    assert!(symbols.contains(&"pkg.client.Response.unused".to_string()));
}

#[test]
fn scripts_inheritance_generics_and_unresolved_receivers_are_reported() {
    let report = analyze_fixture("scripts_inheritance_generics_and_unresolved_receivers");
    let symbols = finding_symbols(&report);
    let diagnostics = report
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();

    assert!(!symbols.contains(&"pkg.script.ExampleEntity.save".to_string()));
    assert!(symbols.contains(&"pkg.script.Other.save".to_string()));
    assert!(diagnostics.contains(&"DCF101"));
}

#[test]
fn generated_service_graph_is_deterministic_and_debuggable() {
    let report = analyze_fixture("generated_service_graph_is_deterministic_and_debuggable");
    let symbols = finding_symbols(&report);

    assert!(!symbols.contains(&"pkg.generated.node_3".to_string()));
    assert!(symbols.contains(&"pkg.generated.node_4".to_string()));
    assert!(symbols.contains(&"pkg.generated.node_5".to_string()));
}

#[test]
fn test_usage_does_not_keep_production_symbols_alive() {
    let report = analyze_fixture("test_usage_does_not_keep_production_symbols_alive");
    let helper = report
        .findings
        .iter()
        .find(|finding| finding.symbol == "pkg.service.helper")
        .unwrap();

    assert_eq!(helper.reachable_from, vec!["test".to_string()]);
}

#[test]
fn include_tests_reports_dead_helpers_inside_test_files() {
    let report = analyze_fixture("include_tests_reports_dead_helpers_inside_test_files");
    let symbols = finding_symbols(&report);

    assert!(!symbols.contains(&"pkg.tests.test_service.test_live".to_string()));
    assert!(symbols.contains(&"pkg.tests.test_service.dead_helper".to_string()));
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
