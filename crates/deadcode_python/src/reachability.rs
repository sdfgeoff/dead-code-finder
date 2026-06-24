use std::collections::{HashMap, HashSet, VecDeque};

use deadcode_core::{Diagnostic, Finding, Severity, SymbolKind};

use crate::symbol_index::{ImportTarget, ModuleIndex, SymbolIndex};

pub fn find_unused_symbols(index: &SymbolIndex) -> Vec<Finding> {
    let live = compute_live_symbols(index);
    let mut findings = Vec::new();
    for module in &index.modules {
        for symbol in &module.symbols {
            if symbol.kind == SymbolKind::Module || live.contains(&symbol.qualified_name) {
                continue;
            }
            findings.push(Finding::unused(
                code_for_kind(&symbol.kind),
                symbol.qualified_name.clone(),
                symbol.kind.clone(),
                symbol.span.clone(),
            ));
        }
    }
    findings.sort_by(|left, right| {
        left.span
            .file
            .cmp(&right.span.file)
            .then_with(|| left.span.line.cmp(&right.span.line))
            .then_with(|| left.symbol.cmp(&right.symbol))
    });
    findings
}

pub fn unresolved_receiver_diagnostics(index: &SymbolIndex) -> Vec<Diagnostic> {
    let live = compute_live_symbols(index);
    let mut diagnostics = Vec::new();
    for module in &index.modules {
        for unresolved in &module.unresolved_receivers {
            if !live.contains(&unresolved.from) {
                continue;
            }
            diagnostics.push(Diagnostic {
                code: "DCF101".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "cannot resolve receiver type for {}.{}",
                    unresolved.receiver, unresolved.member
                ),
                span: unresolved.span.clone(),
            });
        }
    }
    diagnostics.sort_by(|left, right| {
        left.span
            .file
            .cmp(&right.span.file)
            .then_with(|| left.span.line.cmp(&right.span.line))
            .then_with(|| left.message.cmp(&right.message))
    });
    diagnostics
}

pub fn unsupported_expansion_diagnostics(index: &SymbolIndex) -> Vec<Diagnostic> {
    let live = compute_live_symbols(index);
    let mut diagnostics = Vec::new();
    for module in &index.modules {
        for expansion in &module.unsupported_expansions {
            if !live.contains(&expansion.from) {
                continue;
            }
            diagnostics.push(Diagnostic {
                code: "DCF103".to_string(),
                severity: Severity::Warning,
                message: format!(
                    "cannot expand keyword payload for construction of {}",
                    expansion.target
                ),
                span: expansion.span.clone(),
            });
        }
    }
    diagnostics.sort_by(|left, right| {
        left.span
            .file
            .cmp(&right.span.file)
            .then_with(|| left.span.line.cmp(&right.span.line))
            .then_with(|| left.message.cmp(&right.message))
    });
    diagnostics
}

fn compute_live_symbols(index: &SymbolIndex) -> HashSet<String> {
    let symbol_modules = symbol_module_map(index);
    let symbol_kinds = symbol_kind_map(index);
    let module_map = module_map(index);
    let mut live = root_modules(index);
    let mut queue = live.iter().cloned().collect::<VecDeque<_>>();

    while let Some(owner) = queue.pop_front() {
        if let Some(module) = module_map.get(owner.as_str()) {
            for import in &module.imports {
                if let Some(target) = imported_module_target(&import.target) {
                    push_live(target, &mut live, &mut queue);
                }
            }
        }

        let Some(module_name) = owner_module(&owner, &symbol_modules, &module_map) else {
            continue;
        };
        let Some(module) = module_map.get(module_name.as_str()) else {
            continue;
        };

        for reference in module
            .references
            .iter()
            .filter(|reference| reference.from == owner)
        {
            if let Some(target) = resolve_reference(module, &reference.name, &symbol_kinds) {
                push_live(&target, &mut live, &mut queue);
            }
        }

        for reference in module
            .member_references
            .iter()
            .filter(|reference| reference.from == owner)
        {
            if symbol_kinds.contains_key(&reference.target) {
                push_live(&reference.target, &mut live, &mut queue);
            }
        }
    }

    live
}

fn root_modules(index: &SymbolIndex) -> HashSet<String> {
    index
        .modules
        .iter()
        .filter(|module| module.is_entrypoint)
        .map(|module| module.module.clone())
        .collect()
}

fn module_map(index: &SymbolIndex) -> HashMap<&str, &ModuleIndex> {
    index
        .modules
        .iter()
        .map(|module| (module.module.as_str(), module))
        .collect()
}

fn symbol_module_map(index: &SymbolIndex) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for module in &index.modules {
        for symbol in &module.symbols {
            map.insert(symbol.qualified_name.clone(), module.module.clone());
        }
    }
    map
}

fn symbol_kind_map(index: &SymbolIndex) -> HashMap<String, SymbolKind> {
    let mut map = HashMap::new();
    for module in &index.modules {
        for symbol in &module.symbols {
            map.insert(symbol.qualified_name.clone(), symbol.kind.clone());
        }
    }
    map
}

fn imported_module_target(target: &ImportTarget) -> Option<&str> {
    match target {
        ImportTarget::Module {
            module,
            external: false,
        }
        | ImportTarget::Symbol {
            module,
            external: false,
            ..
        }
        | ImportTarget::Star {
            module,
            external: false,
        } => Some(module),
        _ => None,
    }
}

fn owner_module(
    owner: &str,
    symbol_modules: &HashMap<String, String>,
    modules: &HashMap<&str, &ModuleIndex>,
) -> Option<String> {
    if modules.contains_key(owner) {
        return Some(owner.to_string());
    }
    symbol_modules.get(owner).cloned()
}

fn resolve_reference(
    module: &ModuleIndex,
    name: &str,
    symbol_kinds: &HashMap<String, SymbolKind>,
) -> Option<String> {
    for import in &module.imports {
        if import.binding != name {
            continue;
        }
        return match &import.target {
            ImportTarget::Module {
                module,
                external: false,
            } => Some(module.clone()),
            ImportTarget::Symbol {
                module,
                name,
                external: false,
            } => Some(format!("{module}.{name}")),
            _ => None,
        };
    }

    let same_module_symbol = format!("{}.{}", module.module, name);
    symbol_kinds
        .contains_key(&same_module_symbol)
        .then_some(same_module_symbol)
}

fn push_live(target: &str, live: &mut HashSet<String>, queue: &mut VecDeque<String>) {
    if live.insert(target.to_string()) {
        queue.push_back(target.to_string());
    }
}

fn code_for_kind(kind: &SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Function => "DCF001",
        SymbolKind::Class => "DCF002",
        SymbolKind::Method => "DCF003",
        SymbolKind::Attribute | SymbolKind::Field => "DCF004",
        SymbolKind::Module => "DCF000",
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use crate::config::{LoadedProjectConfig, ResolvedRoot};
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
            entrypoints,
            include_tests: false,
            test_patterns: Vec::new(),
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
