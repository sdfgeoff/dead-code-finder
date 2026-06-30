use super::*;

mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use crate::config::{ConstructorRule, LoadedProjectConfig, ResolvedRoot, RuleConfig};
    use crate::symbol_index::index_project;

    use super::*;

    #[test]
    fn reports_dead_islands_not_just_unreferenced_functions() {
        let workspace = test_workspace("reports_dead_islands_not_just_unreferenced_functions");
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("main.py"),
            r#"
def live():
    helper()

def helper():
    pass

def old_view():
    old_helper()

def old_helper():
    pass

live()
"#,
        )
        .unwrap();
        let config = loaded_config(
            &workspace,
            vec![root(&package, "pkg")],
            vec!["pkg/main.py".to_string()],
        );

        let index = index_project(&config).unwrap();
        let findings = find_unused_symbols(&index);
        let symbols = finding_symbols(&findings);

        assert!(!symbols.contains(&"pkg.main.live".to_string()));
        assert!(!symbols.contains(&"pkg.main.helper".to_string()));
        assert!(symbols.contains(&"pkg.main.old_view".to_string()));
        assert!(symbols.contains(&"pkg.main.old_helper".to_string()));
    }

    #[test]
    fn follows_used_import_bindings_to_imported_symbols() {
        let workspace = test_workspace("follows_used_import_bindings_to_imported_symbols");
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("main.py"),
            "from pkg.helpers import live\n\nlive()\n",
        )
        .unwrap();
        fs::write(
            package.join("helpers.py"),
            r#"
def live():
    pass

def dead():
    pass
"#,
        )
        .unwrap();
        let config = loaded_config(
            &workspace,
            vec![root(&package, "pkg")],
            vec!["pkg/main.py".to_string()],
        );

        let index = index_project(&config).unwrap();
        let findings = find_unused_symbols(&index);
        let symbols = finding_symbols(&findings);

        assert!(!symbols.contains(&"pkg.helpers.live".to_string()));
        assert!(symbols.contains(&"pkg.helpers.dead".to_string()));
    }

    #[test]
    fn treats_main_guard_as_entrypoint() {
        let workspace = test_workspace("treats_main_guard_as_entrypoint");
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("script.py"),
            r#"
def live():
    pass

def dead():
    pass

if __name__ == "__main__":
    live()
"#,
        )
        .unwrap();
        let config = loaded_config(&workspace, vec![root(&package, "pkg")], Vec::new());

        let index = index_project(&config).unwrap();
        let findings = find_unused_symbols(&index);
        let symbols = finding_symbols(&findings);

        assert!(!symbols.contains(&"pkg.script.live".to_string()));
        assert!(symbols.contains(&"pkg.script.dead".to_string()));
    }

    #[test]
    fn resolves_method_call_from_constructor_assignment_without_name_matching() {
        let workspace = test_workspace(
            "resolves_method_call_from_constructor_assignment_without_name_matching",
        );
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("main.py"),
            r#"
class ExampleEntity:
    def save(self):
        pass

class Other:
    def save(self):
        pass

def run():
    entity = ExampleEntity()
    entity.save()

run()
"#,
        )
        .unwrap();
        let config = loaded_config(
            &workspace,
            vec![root(&package, "pkg")],
            vec!["pkg/main.py".to_string()],
        );

        let index = index_project(&config).unwrap();
        let findings = find_unused_symbols(&index);
        let symbols = finding_symbols(&findings);

        assert!(!symbols.contains(&"pkg.main.ExampleEntity.save".to_string()));
        assert!(symbols.contains(&"pkg.main.Other.save".to_string()));
    }

    #[test]
    fn construction_marks_init_method_live() {
        let workspace = test_workspace("construction_marks_init_method_live");
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("main.py"),
            r#"
class ExampleEntity:
    def __init__(self):
        pass

class Other:
    def __init__(self):
        pass

def run():
    ExampleEntity()

run()
"#,
        )
        .unwrap();
        let config = loaded_config(
            &workspace,
            vec![root(&package, "pkg")],
            vec!["pkg/main.py".to_string()],
        );

        let index = index_project(&config).unwrap();
        let findings = find_unused_symbols(&index);
        let symbols = finding_symbols(&findings);

        assert!(!symbols.contains(&"pkg.main.ExampleEntity.__init__".to_string()));
        assert!(symbols.contains(&"pkg.main.Other.__init__".to_string()));
    }

    #[test]
    fn functions_can_use_module_level_typed_bindings() {
        let workspace = test_workspace("functions_can_use_module_level_typed_bindings");
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("main.py"),
            r#"
class Config:
    value: str

def get_config() -> Config:
    return Config()

config = get_config()

def run():
    return config.value

run()
"#,
        )
        .unwrap();
        let mut config = loaded_config(
            &workspace,
            vec![root(&package, "pkg")],
            vec!["pkg/main.py".to_string()],
        );
        config.rules = RuleConfig {
            constructors: vec![ConstructorRule {
                match_: "pkg.main.get_config".to_string(),
                produces_type: "pkg.main.Config".to_string(),
            }],
            ..RuleConfig::default()
        };

        let index = index_project(&config).unwrap();
        let findings = find_unused_symbols(&index);
        let symbols = finding_symbols(&findings);

        assert!(!symbols.contains(&"pkg.main.Config.value".to_string()));
    }

    #[test]
    fn self_attributes_assigned_from_typed_init_parameters_resolve_nested_calls() {
        let workspace = test_workspace(
            "self_attributes_assigned_from_typed_init_parameters_resolve_nested_calls",
        );
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("main.py"),
            r#"
class Client:
    def live(self):
        pass

    def dead(self):
        pass

class Service:
    def __init__(self, client: Client):
        self.client = client

    def run(self):
        self.client.live()

def main():
    service = Service(Client())
    service.run()

main()
"#,
        )
        .unwrap();
        let config = loaded_config(
            &workspace,
            vec![root(&package, "pkg")],
            vec!["pkg/main.py".to_string()],
        );

        let index = index_project(&config).unwrap();
        let findings = find_unused_symbols(&index);
        let symbols = finding_symbols(&findings);

        assert!(!symbols.contains(&"pkg.main.Client.live".to_string()));
        assert!(symbols.contains(&"pkg.main.Client.dead".to_string()));
    }

    #[test]
    fn resolves_method_call_from_parameter_annotation() {
        let workspace = test_workspace("resolves_method_call_from_parameter_annotation");
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("main.py"),
            r#"
class ExampleEntity:
    def save(self):
        pass

def process(entity: ExampleEntity):
    entity.save()

process(ExampleEntity())
"#,
        )
        .unwrap();
        let config = loaded_config(
            &workspace,
            vec![root(&package, "pkg")],
            vec!["pkg/main.py".to_string()],
        );

        let index = index_project(&config).unwrap();
        let findings = find_unused_symbols(&index);
        let symbols = finding_symbols(&findings);

        assert!(!symbols.contains(&"pkg.main.ExampleEntity.save".to_string()));
    }

    #[test]
    fn constructor_keywords_mark_only_matching_owner_field_used() {
        let workspace = test_workspace("constructor_keywords_mark_only_matching_owner_field_used");
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("main.py"),
            r#"
class ExampleEntity:
    name: str

class Other:
    name: str

def run():
    ExampleEntity(name="A")

run()
"#,
        )
        .unwrap();
        let config = loaded_config(
            &workspace,
            vec![root(&package, "pkg")],
            vec!["pkg/main.py".to_string()],
        );

        let index = index_project(&config).unwrap();
        let findings = find_unused_symbols(&index);
        let symbols = finding_symbols(&findings);

        assert!(!symbols.contains(&"pkg.main.ExampleEntity.name".to_string()));
        assert!(symbols.contains(&"pkg.main.Other.name".to_string()));
    }

    #[test]
    fn writes_mark_only_resolved_receiver_field_used() {
        let workspace = test_workspace("writes_mark_only_resolved_receiver_field_used");
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("main.py"),
            r#"
class ExampleEntity:
    name: str

class Other:
    name: str

def run(entity: ExampleEntity, other: Other):
    entity.name = "A"

run(ExampleEntity(), Other())
"#,
        )
        .unwrap();
        let config = loaded_config(
            &workspace,
            vec![root(&package, "pkg")],
            vec!["pkg/main.py".to_string()],
        );

        let index = index_project(&config).unwrap();
        let findings = find_unused_symbols(&index);
        let symbols = finding_symbols(&findings);

        assert!(!symbols.contains(&"pkg.main.ExampleEntity.name".to_string()));
        assert!(symbols.contains(&"pkg.main.Other.name".to_string()));
    }

    #[test]
    fn unsupported_constructor_expansion_warns_without_field_expansion() {
        let workspace =
            test_workspace("unsupported_constructor_expansion_warns_without_field_expansion");
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("main.py"),
            r#"
class ExampleEntity:
    name: str

def run(payload):
    ExampleEntity(**payload)

run({})
"#,
        )
        .unwrap();
        let config = loaded_config(
            &workspace,
            vec![root(&package, "pkg")],
            vec!["pkg/main.py".to_string()],
        );

        let index = index_project(&config).unwrap();
        let findings = find_unused_symbols(&index);
        let symbols = finding_symbols(&findings);
        let diagnostics = unsupported_expansion_diagnostics(&index);

        assert!(symbols.contains(&"pkg.main.ExampleEntity.name".to_string()));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "DCF103");
        assert!(diagnostics[0]
            .message
            .contains("cannot expand keyword payload for construction of pkg.main.ExampleEntity"));
    }

    #[test]
    fn inherited_method_is_live_when_called_through_subclass() {
        let workspace = test_workspace("inherited_method_is_live_when_called_through_subclass");
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("main.py"),
            r#"
class Repository:
    def save(self):
        pass

class SqlRepository(Repository):
    pass

def run():
    repo = SqlRepository()
    repo.save()

run()
"#,
        )
        .unwrap();
        let config = loaded_config(
            &workspace,
            vec![root(&package, "pkg")],
            vec!["pkg/main.py".to_string()],
        );

        let index = index_project(&config).unwrap();
        let findings = find_unused_symbols(&index);
        let symbols = finding_symbols(&findings);

        assert!(!symbols.contains(&"pkg.main.Repository.save".to_string()));
    }

    #[test]
    fn unused_overrides_remain_reportable() {
        let workspace = test_workspace("unused_overrides_remain_reportable");
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("main.py"),
            r#"
class Repository:
    def save(self):
        pass

class SqlRepository(Repository):
    def save(self):
        pass

class MemoryRepository(Repository):
    def save(self):
        pass

def run():
    repo = SqlRepository()
    repo.save()

run()
"#,
        )
        .unwrap();
        let config = loaded_config(
            &workspace,
            vec![root(&package, "pkg")],
            vec!["pkg/main.py".to_string()],
        );

        let index = index_project(&config).unwrap();
        let findings = find_unused_symbols(&index);
        let symbols = finding_symbols(&findings);

        assert!(!symbols.contains(&"pkg.main.SqlRepository.save".to_string()));
        assert!(symbols.contains(&"pkg.main.Repository.save".to_string()));
        assert!(symbols.contains(&"pkg.main.MemoryRepository.save".to_string()));
    }

    #[test]
    fn base_typed_slot_reaches_concrete_subtype_override_from_direct_argument_flow() {
        let workspace = test_workspace(
            "base_typed_slot_reaches_concrete_subtype_override_from_direct_argument_flow",
        );
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("main.py"),
            r#"
class Repository:
    def save(self):
        pass

class SqlRepository(Repository):
    def save(self):
        pass

class MemoryRepository(Repository):
    def save(self):
        pass

def process(repo: Repository):
    repo.save()

process(SqlRepository())
"#,
        )
        .unwrap();
        let config = loaded_config(
            &workspace,
            vec![root(&package, "pkg")],
            vec!["pkg/main.py".to_string()],
        );

        let index = index_project(&config).unwrap();
        let findings = find_unused_symbols(&index);
        let symbols = finding_symbols(&findings);

        assert!(!symbols.contains(&"pkg.main.Repository.save".to_string()));
        assert!(!symbols.contains(&"pkg.main.SqlRepository.save".to_string()));
        assert!(symbols.contains(&"pkg.main.MemoryRepository.save".to_string()));
    }

    #[test]
    fn generic_receiver_annotations_resolve_methods_on_erased_base() {
        let workspace =
            test_workspace("generic_receiver_annotations_resolve_methods_on_erased_base");
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("main.py"),
            r#"
class ExampleEntity:
    pass

class Box[T]:
    def unpack(self):
        pass

def process(box: Box[ExampleEntity]):
    box.unpack()

process(Box())
"#,
        )
        .unwrap();
        let config = loaded_config(
            &workspace,
            vec![root(&package, "pkg")],
            vec!["pkg/main.py".to_string()],
        );

        let index = index_project(&config).unwrap();
        let findings = find_unused_symbols(&index);
        let symbols = finding_symbols(&findings);

        assert!(!symbols.contains(&"pkg.main.Box.unpack".to_string()));
    }

    #[test]
    fn generic_field_type_parameter_substitution_resolves_followup_member_call() {
        let workspace = test_workspace(
            "generic_field_type_parameter_substitution_resolves_followup_member_call",
        );
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("main.py"),
            r#"
class ExampleEntity:
    def save(self):
        pass

class Other:
    def save(self):
        pass

class Box[T]:
    value: T

def process(box: Box[ExampleEntity]):
    entity = box.value
    entity.save()

process(Box())
"#,
        )
        .unwrap();
        let config = loaded_config(
            &workspace,
            vec![root(&package, "pkg")],
            vec!["pkg/main.py".to_string()],
        );

        let index = index_project(&config).unwrap();
        let findings = find_unused_symbols(&index);
        let symbols = finding_symbols(&findings);

        assert!(!symbols.contains(&"pkg.main.Box.value".to_string()));
        assert!(!symbols.contains(&"pkg.main.ExampleEntity.save".to_string()));
        assert!(symbols.contains(&"pkg.main.Other.save".to_string()));
    }

    #[test]
    fn emits_unresolved_receiver_diagnostic_for_reachable_code() {
        let workspace = test_workspace("emits_unresolved_receiver_diagnostic_for_reachable_code");
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("main.py"),
            r#"
def run(x):
    x.save()

run(None)
"#,
        )
        .unwrap();
        let config = loaded_config(
            &workspace,
            vec![root(&package, "pkg")],
            vec!["pkg/main.py".to_string()],
        );

        let index = index_project(&config).unwrap();
        let diagnostics = unresolved_receiver_diagnostics(&index);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "DCF101");
        assert!(diagnostics[0]
            .message
            .contains("cannot resolve receiver type for x.save"));
    }

    #[test]
    fn skips_unresolved_receiver_diagnostic_for_unreachable_code() {
        let workspace = test_workspace("skips_unresolved_receiver_diagnostic_for_unreachable_code");
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("main.py"),
            r#"
def live():
    pass

def dead(x):
    x.save()

live()
"#,
        )
        .unwrap();
        let config = loaded_config(
            &workspace,
            vec![root(&package, "pkg")],
            vec!["pkg/main.py".to_string()],
        );

        let index = index_project(&config).unwrap();

        assert!(unresolved_receiver_diagnostics(&index).is_empty());
    }

    fn finding_symbols(findings: &[Finding]) -> Vec<String> {
        findings
            .iter()
            .map(|finding| finding.symbol.clone())
            .collect()
    }

    fn loaded_config(
        workspace: &Path,
        roots: Vec<ResolvedRoot>,
        entrypoints: Vec<String>,
    ) -> LoadedProjectConfig {
        LoadedProjectConfig {
            config_path: workspace.join("dead-code-finder.json"),
            project_dir: workspace.to_path_buf(),
            roots,
            type_sources: Vec::new(),
            root_groups: vec![crate::config::LoadedRootGroup {
                name: "main".to_string(),
                entrypoints,
                counts_as_used: true,
            }],
            include_tests: false,
            test_patterns: Vec::new(),
            rules: RuleConfig::default(),
        }
    }

    fn root(path: &Path, module: &str) -> ResolvedRoot {
        ResolvedRoot {
            path: path.to_path_buf(),
            module: module.to_string(),
        }
    }

    fn test_workspace(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("deadcode_reachability_{name}_{unique}"));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
