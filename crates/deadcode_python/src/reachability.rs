use std::collections::{HashMap, HashSet, VecDeque};

use deadcode_core::{Finding, SymbolKind};

use crate::symbol_index::{ImportTarget, ModuleIndex, SymbolIndex};

pub fn find_unused_symbols(index: &SymbolIndex) -> Vec<Finding> {
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
    }

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
