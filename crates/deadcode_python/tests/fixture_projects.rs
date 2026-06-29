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
fn builtin_open_context_manager_suppresses_file_reads() {
    let report = analyze_fixture("builtin_open_context_manager_suppresses_file_reads");

    assert!(report.diagnostics.is_empty());
}

#[test]
fn contextmanager_wraps_generator_return() {
    let report = analyze_fixture("contextmanager_wraps_generator_return");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Resource.used".to_string()));
    assert!(!symbols.contains(&"pkg.main.Resource.used_field".to_string()));
    assert!(symbols.contains(&"pkg.main.Resource.unused".to_string()));
    assert!(symbols.contains(&"pkg.main.Resource.unused_field".to_string()));
}

#[test]
fn external_type_flows_suppress_unresolved_receivers() {
    let report = analyze_fixture("external_type_flows_suppress_unresolved_receivers");

    assert!(report.diagnostics.is_empty());
}

#[test]
fn external_reexports_preserve_external_type_flow() {
    let report = analyze_fixture("external_reexports_preserve_external_type_flow");

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
fn local_call_result_field_read_resolves_type() {
    let report = analyze_fixture("local_call_result_field_read_resolves_type");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Settings.host".to_string()));
    assert!(symbols.contains(&"pkg.main.Settings.unused".to_string()));
}

#[test]
fn optional_ifexp_resolves_guarded_field_reads() {
    let report = analyze_fixture("optional_ifexp_resolves_guarded_field_reads");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Settings.actions".to_string()));
    assert!(!symbols.contains(&"pkg.main.Settings.notifications".to_string()));
    assert!(!symbols.contains(&"pkg.main.Actions.enabled".to_string()));
    assert!(symbols.contains(&"pkg.main.Actions.unused".to_string()));
    assert!(symbols.contains(&"pkg.main.Settings.unused".to_string()));
}

#[test]
fn optional_list_ifexp_empty_list_binds_list_type() {
    let report = analyze_fixture("optional_list_ifexp_empty_list_binds_list_type");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Process.message_items".to_string()));
    assert!(!symbols.contains(&"pkg.main.ExampleMessage.to".to_string()));
    assert!(symbols.contains(&"pkg.main.ExampleMessage.unused".to_string()));
    assert!(symbols.contains(&"pkg.main.Process.unused".to_string()));
}

#[test]
fn optional_list_bool_or_empty_list_binds_item_type() {
    let report = analyze_fixture("optional_list_bool_or_empty_list_binds_item_type");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.BaseProperties.class_labels".to_string()));
    assert!(symbols.contains(&"pkg.main.Properties.unused".to_string()));
}

#[test]
fn bool_or_coalesce_resolves_optional_field_type() {
    let report = analyze_fixture("bool_or_coalesce_resolves_optional_field_type");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Settings.branding".to_string()));
    assert!(!symbols.contains(&"pkg.main.Branding.logo_url".to_string()));
    assert!(symbols.contains(&"pkg.main.Branding.unused".to_string()));
    assert!(symbols.contains(&"pkg.main.Settings.unused".to_string()));
}

#[test]
fn local_return_list_iteration_resolves_item_fields() {
    let report = analyze_fixture("local_return_list_iteration_resolves_item_fields");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.QueuedEvent.record_id".to_string()));
    assert!(!symbols.contains(&"pkg.main.QueuedEvent.event".to_string()));
    assert!(!symbols.contains(&"pkg.main.Event.event_type".to_string()));
    assert!(symbols.contains(&"pkg.main.Event.unused".to_string()));
}

#[test]
fn local_return_tuple_unpack_binds_positional_types() {
    let report = analyze_fixture("local_return_tuple_unpack_binds_positional_types");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.ExampleRef.version_id".to_string()));
    assert!(!symbols.contains(&"pkg.main.Properties.name".to_string()));
    assert!(symbols.contains(&"pkg.main.ExampleRef.unused".to_string()));
    assert!(symbols.contains(&"pkg.main.Properties.unused".to_string()));
}

#[test]
fn string_methods_and_unpacking_bind_builtin_types() {
    let report = analyze_fixture("string_methods_and_unpacking_bind_builtin_types");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Parsed.method".to_string()));
    assert!(!symbols.contains(&"pkg.main.Parsed.url".to_string()));
    assert!(symbols.contains(&"pkg.main.Parsed.unused".to_string()));
}

#[test]
fn optional_string_split_items_resolve_methods() {
    let report = analyze_fixture("optional_string_split_items_resolve_methods");

    assert!(report.diagnostics.is_empty());
}

#[test]
fn builtin_constructor_method_chain_unpack() {
    let report = analyze_fixture("builtin_constructor_method_chain_unpack");

    assert!(report.findings.is_empty());
    assert!(report.diagnostics.is_empty());
}

#[test]
fn io_bytesio_constructor_and_tuple_return() {
    let report = analyze_fixture("io_bytesio_constructor_and_tuple_return");

    assert!(report.findings.is_empty());
    assert!(report.diagnostics.is_empty());
}

#[test]
fn executor_callable_return_tuple_unpack() {
    let report = analyze_fixture("executor_callable_return_tuple_unpack");

    assert!(report.findings.is_empty());
    assert!(report.diagnostics.is_empty());
}

#[test]
fn pathlib_path_join_preserves_external_receiver() {
    let report = analyze_fixture("pathlib_path_join_preserves_external_receiver");

    assert!(report.findings.is_empty());
    assert!(report.diagnostics.is_empty());
}

#[test]
fn builtin_collection_constructor_preserves_iterable_item_type() {
    let report = analyze_fixture("builtin_collection_constructor_preserves_iterable_item_type");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Feature.geometry".to_string()));
    assert!(!symbols.contains(&"pkg.main.Feature.properties".to_string()));
    assert!(!symbols.contains(&"pkg.main.Properties.status".to_string()));
    assert!(symbols.contains(&"pkg.main.Feature.unused".to_string()));
    assert!(symbols.contains(&"pkg.main.Properties.unused".to_string()));
}

#[test]
fn string_join_result_is_string() {
    let report = analyze_fixture("string_join_result_is_string");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.ExampleItem.example_item_id".to_string()));
    assert!(!symbols.contains(&"pkg.main.ExampleItem.version_id".to_string()));
    assert!(!symbols.contains(&"pkg.main.ExampleCollection.example_items".to_string()));
    assert!(symbols.contains(&"pkg.main.ExampleItem.unused".to_string()));
    assert!(symbols.contains(&"pkg.main.ExampleCollection.unused".to_string()));
}

#[test]
fn datetime_optional_arithmetic_resolves_methods() {
    let report = analyze_fixture("datetime_optional_arithmetic_resolves_methods");

    assert!(report.findings.is_empty());
    assert!(report.diagnostics.is_empty());
}

#[test]
fn awaited_mapping_subscript_resolves_generic_item_fields() {
    let report = analyze_fixture("awaited_mapping_subscript_resolves_generic_item_fields");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.models.Feature.geometry".to_string()));
    assert!(!symbols.contains(&"pkg.models.Feature.properties".to_string()));
    assert!(!symbols.contains(&"pkg.models.Properties.name".to_string()));
    assert!(symbols.contains(&"pkg.models.Properties.unused".to_string()));
}

#[test]
fn imported_module_list_literals_resolve_item_fields() {
    let report = analyze_fixture("imported_module_list_literals_resolve_item_fields");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.schemes.Scheme.name".to_string()));
    assert!(!symbols.contains(&"pkg.schemes.Scheme.enabled".to_string()));
    assert!(symbols.contains(&"pkg.schemes.Scheme.unused".to_string()));
}

#[test]
fn mapping_get_results_resolve_value_fields() {
    let report = analyze_fixture("mapping_get_results_resolve_value_fields");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Item.name".to_string()));
    assert!(symbols.contains(&"pkg.main.Item.unused".to_string()));
}

#[test]
fn cast_ifexp_resolves_mapping_get_receiver() {
    let report = analyze_fixture("cast_ifexp_resolves_mapping_get_receiver");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Result.error".to_string()));
    assert!(symbols.contains(&"pkg.main.Result.unused".to_string()));
}

#[test]
fn mapping_setdefault_results_resolve_value_fields() {
    let report = analyze_fixture("mapping_setdefault_results_resolve_value_fields");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Accumulator.id".to_string()));
    assert!(!symbols.contains(&"pkg.main.Accumulator.name".to_string()));
    assert!(symbols.contains(&"pkg.main.Accumulator.unused".to_string()));
}

#[test]
fn dict_items_iteration_resolves_value_fields() {
    let report = analyze_fixture("dict_items_iteration_resolves_value_fields");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Heading.stock_heading".to_string()));
    assert!(!symbols.contains(&"pkg.main.Heading.residual_heading".to_string()));
    assert!(symbols.contains(&"pkg.main.Heading.unused".to_string()));
}

#[test]
fn dict_comprehension_resolves_value_fields() {
    let report = analyze_fixture("dict_comprehension_resolves_value_fields");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Sheet.label".to_string()));
    assert!(symbols.contains(&"pkg.main.Sheet.unused".to_string()));
}

#[test]
fn property_methods_resolve_as_fields() {
    let report = analyze_fixture("property_methods_resolve_as_fields");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Workbook.worksheets".to_string()));
    assert!(!symbols.contains(&"pkg.main.Sheet.label".to_string()));
    assert!(symbols.contains(&"pkg.main.Sheet.unused".to_string()));
}

#[test]
fn callable_return_async_iterator_resolves_item_fields() {
    let report = analyze_fixture("callable_return_async_iterator_resolves_item_fields");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.main.Chunk.choices".to_string()));
    assert!(!symbols.contains(&"pkg.main.Choice.text".to_string()));
    assert!(symbols.contains(&"pkg.main.Chunk.unused".to_string()));
    assert!(symbols.contains(&"pkg.main.Choice.unused".to_string()));
}

#[test]
fn fluent_external_base_methods_preserve_receiver_type() {
    let report = analyze_fixture("fluent_external_base_methods_preserve_receiver_type");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.settings.Config.host".to_string()));
    assert!(symbols.contains(&"pkg.settings.Config.unused".to_string()));
}

#[test]
fn generic_method_typevar_return_from_type_argument() {
    let report = analyze_fixture("generic_method_typevar_return_from_type_argument");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.service.Model.field".to_string()));
    assert!(symbols.contains(&"pkg.service.Model.unused".to_string()));
}

#[test]
fn imported_generic_type_alias_resolves_member_fields() {
    let report = analyze_fixture("imported_generic_type_alias_resolves_member_fields");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.geo.FeatureCollection.features".to_string()));
    assert!(!symbols.contains(&"pkg.geo.Feature.properties".to_string()));
    assert!(!symbols.contains(&"pkg.geo.Feature.geometry".to_string()));
    assert!(!symbols.contains(&"pkg.geo.BoundaryProperties.title_area".to_string()));
    assert!(symbols.contains(&"pkg.geo.BoundaryProperties.unused".to_string()));
}

#[test]
fn reexported_generic_type_alias_resolves_member_fields() {
    let report = analyze_fixture("reexported_generic_type_alias_resolves_member_fields");
    let symbols = finding_symbols(&report);

    assert!(report.diagnostics.is_empty());
    assert!(!symbols.contains(&"pkg.core.FeatureCollection.features".to_string()));
    assert!(!symbols.contains(&"pkg.core.Feature.properties".to_string()));
    assert!(!symbols.contains(&"pkg.core.Feature.geometry".to_string()));
    assert!(!symbols.contains(&"pkg.core.BoundaryProperties.title_area".to_string()));
    assert!(symbols.contains(&"pkg.core.BoundaryProperties.unused".to_string()));
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

#[test]
fn duplicate_field_definitions_report_once() {
    let report = analyze_fixture("duplicate_field_definitions_report_once");
    let symbols = finding_symbols(&report);

    assert_eq!(
        symbols
            .iter()
            .filter(|symbol| symbol.as_str() == "pkg.main.Record.count")
            .count(),
        1
    );
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
