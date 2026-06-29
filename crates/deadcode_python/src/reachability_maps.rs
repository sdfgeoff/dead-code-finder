use std::collections::{HashMap, HashSet};

use deadcode_core::SymbolKind;

use crate::symbol_index::{
    ClassInfo, DependencyOverride, FunctionDependency, FunctionSignature, ImportTarget,
    ModuleIndex, PytestFixture, SymbolIndex,
};

pub(super) fn module_map(index: &SymbolIndex) -> HashMap<&str, &ModuleIndex> {
    index
        .modules
        .iter()
        .map(|module| (module.module.as_str(), module))
        .collect()
}

pub(super) fn symbol_module_map(index: &SymbolIndex) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for module in &index.modules {
        for symbol in &module.symbols {
            map.insert(symbol.qualified_name.clone(), module.module.clone());
        }
        for value in &module.module_values {
            map.insert(value.qualified_name.clone(), module.module.clone());
        }
    }
    map
}

pub(super) fn symbol_kind_map(index: &SymbolIndex) -> HashMap<String, SymbolKind> {
    let mut map = HashMap::new();
    for module in &index.modules {
        for symbol in &module.symbols {
            map.insert(symbol.qualified_name.clone(), symbol.kind.clone());
        }
    }
    map
}

pub(super) fn class_map(index: &SymbolIndex) -> HashMap<String, ClassInfo> {
    let mut map = HashMap::new();
    for module in &index.modules {
        for class in &module.classes {
            map.insert(class.class.clone(), class.clone());
        }
    }
    map
}

pub(super) fn function_signature_map(index: &SymbolIndex) -> HashMap<String, FunctionSignature> {
    let mut map = HashMap::new();
    for module in &index.modules {
        for signature in &module.function_signatures {
            map.insert(signature.function.clone(), signature.clone());
        }
    }
    map
}

pub(super) fn module_value_set(index: &SymbolIndex) -> HashSet<String> {
    index
        .modules
        .iter()
        .flat_map(|module| &module.module_values)
        .map(|value| value.qualified_name.clone())
        .collect()
}

pub(super) fn pytest_fixture_map(index: &SymbolIndex) -> HashMap<String, PytestFixture> {
    let mut map = HashMap::new();
    for fixture in index
        .modules
        .iter()
        .filter(|module| module.is_test)
        .flat_map(|module| &module.pytest_fixtures)
    {
        map.insert(fixture.name.clone(), fixture.clone());
    }
    map
}

pub(super) fn function_dependencies(index: &SymbolIndex) -> Vec<FunctionDependency> {
    index
        .modules
        .iter()
        .flat_map(|module| module.function_dependencies.iter().cloned())
        .collect()
}

pub(super) fn dependency_overrides(index: &SymbolIndex) -> Vec<DependencyOverride> {
    index
        .modules
        .iter()
        .flat_map(|module| module.dependency_overrides.iter().cloned())
        .collect()
}

pub(super) fn imported_module_target(target: &ImportTarget) -> Option<&str> {
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

pub(super) fn owner_module(
    owner: &str,
    symbol_modules: &HashMap<String, String>,
    modules: &HashMap<&str, &ModuleIndex>,
) -> Option<String> {
    if modules.contains_key(owner) {
        return Some(owner.to_string());
    }
    symbol_modules.get(owner).cloned()
}

pub(super) fn resolve_reference(
    owner: &str,
    module: &ModuleIndex,
    name: &str,
    symbol_kinds: &HashMap<String, SymbolKind>,
    module_values: &HashSet<String>,
) -> Option<String> {
    if symbol_kinds.contains_key(name) {
        return Some(name.to_string());
    }

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

    let owner_local_symbol = format!("{owner}.{name}");
    if symbol_kinds.contains_key(&owner_local_symbol) {
        return Some(owner_local_symbol);
    }

    let mut scope = owner;
    while let Some((parent, _)) = scope.rsplit_once('.') {
        if parent == module.module {
            break;
        }
        let parent_local_symbol = format!("{parent}.{name}");
        if symbol_kinds.contains_key(&parent_local_symbol) {
            return Some(parent_local_symbol);
        }
        scope = parent;
    }

    let same_module_symbol = format!("{}.{}", module.module, name);
    if symbol_kinds.contains_key(&same_module_symbol) || module_values.contains(&same_module_symbol)
    {
        return Some(same_module_symbol);
    }
    None
}
