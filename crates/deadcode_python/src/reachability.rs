use std::collections::{HashMap, HashSet, VecDeque};

use deadcode_core::{Diagnostic, Finding, Severity, SymbolKind};

use crate::symbol_index::{ClassInfo, FunctionSignature, ImportTarget, ModuleIndex, SymbolIndex};

pub fn find_unused_symbols(index: &SymbolIndex) -> Vec<Finding> {
    let live = compute_live_symbols(index, RootSet::Main);
    let test_live = index
        .include_tests
        .then(|| compute_live_symbols(index, RootSet::Test))
        .unwrap_or_default();
    let weak_live = index
        .include_weak
        .then(|| compute_live_symbols(index, RootSet::Weak))
        .unwrap_or_default();
    let mut findings = Vec::new();
    for module in &index.modules {
        for symbol in &module.symbols {
            if symbol.kind == SymbolKind::Module {
                continue;
            }
            if module.is_test {
                if !index.include_tests || test_live.contains(&symbol.qualified_name) {
                    continue;
                }
                findings.push(Finding::unused(
                    code_for_kind(&symbol.kind),
                    symbol.qualified_name.clone(),
                    symbol.kind.clone(),
                    symbol.span.clone(),
                ));
            } else if !live.contains(&symbol.qualified_name) {
                let mut reachable_from = Vec::new();
                if test_live.contains(&symbol.qualified_name) {
                    reachable_from.push("test".to_string());
                }
                if weak_live.contains(&symbol.qualified_name) {
                    reachable_from.push("weak".to_string());
                }
                findings.push(
                    Finding::unused(
                        code_for_kind(&symbol.kind),
                        symbol.qualified_name.clone(),
                        symbol.kind.clone(),
                        symbol.span.clone(),
                    )
                    .with_reachable_from(reachable_from),
                );
            }
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
    let live = compute_live_symbols(index, RootSet::Main);
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
    let live = compute_live_symbols(index, RootSet::Main);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RootSet {
    Main,
    Test,
    Weak,
}

fn compute_live_symbols(index: &SymbolIndex, root_set: RootSet) -> HashSet<String> {
    let symbol_modules = symbol_module_map(index);
    let symbol_kinds = symbol_kind_map(index);
    let module_map = module_map(index);
    let class_map = class_map(index);
    let signature_map = function_signature_map(index);
    let mut concrete_flows: HashMap<(String, String), HashSet<String>> = HashMap::new();
    let mut live = root_symbols(index, root_set);
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

        for call_argument in module
            .call_argument_types
            .iter()
            .filter(|call_argument| call_argument.from == owner)
        {
            let Some(signature) = signature_map.get(call_argument.callee.as_str()) else {
                continue;
            };
            let Some(Some(base_type)) = signature.parameter_types.get(call_argument.position)
            else {
                continue;
            };
            if !is_subclass_or_same(&call_argument.concrete_type, base_type, &class_map) {
                continue;
            }
            let flow_key = (call_argument.callee.clone(), base_type.clone());
            let concrete_types = concrete_flows.entry(flow_key).or_default();
            if concrete_types.insert(call_argument.concrete_type.clone())
                && live.contains(&call_argument.callee)
            {
                queue.push_back(call_argument.callee.clone());
            }
        }

        for reference in module
            .references
            .iter()
            .filter(|reference| reference.from == owner)
        {
            if let Some(target) = resolve_reference(module, &reference.name, &symbol_kinds) {
                push_live(&target, &mut live, &mut queue);
                for route_glob in &index.route_globs {
                    if route_glob.when_function_called == target {
                        for module in &route_glob.modules {
                            push_live(module, &mut live, &mut queue);
                        }
                    }
                }
            }
        }

        for reference in module
            .member_references
            .iter()
            .filter(|reference| reference.from == owner)
        {
            for target in resolve_member_targets(
                &owner,
                &reference.target,
                &symbol_kinds,
                &class_map,
                &concrete_flows,
            ) {
                push_live(&target, &mut live, &mut queue);
            }
        }
    }

    mark_symbol_owners_live(&mut live, &symbol_kinds);
    live
}

fn root_symbols(index: &SymbolIndex, root_set: RootSet) -> HashSet<String> {
    match root_set {
        RootSet::Main => index
            .modules
            .iter()
            .filter(|module| module.is_entrypoint && !module.is_test)
            .map(|module| module.module.clone())
            .collect(),
        RootSet::Test => index
            .modules
            .iter()
            .flat_map(|module| module.test_roots.iter().cloned())
            .collect(),
        RootSet::Weak => index
            .modules
            .iter()
            .filter(|module| module.is_weak_entrypoint)
            .map(|module| module.module.clone())
            .collect(),
    }
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

fn class_map(index: &SymbolIndex) -> HashMap<String, ClassInfo> {
    let mut map = HashMap::new();
    for module in &index.modules {
        for class in &module.classes {
            map.insert(class.class.clone(), class.clone());
        }
    }
    map
}

fn function_signature_map(index: &SymbolIndex) -> HashMap<String, FunctionSignature> {
    let mut map = HashMap::new();
    for module in &index.modules {
        for signature in &module.function_signatures {
            map.insert(signature.function.clone(), signature.clone());
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

fn resolve_member_targets(
    owner: &str,
    target: &str,
    symbol_kinds: &HashMap<String, SymbolKind>,
    class_map: &HashMap<String, ClassInfo>,
    concrete_flows: &HashMap<(String, String), HashSet<String>>,
) -> Vec<String> {
    let Some((receiver_type, member)) = target.rsplit_once('.') else {
        return Vec::new();
    };
    let mut targets = Vec::new();
    if let Some(resolved) = lookup_member(receiver_type, member, symbol_kinds, class_map) {
        targets.push(resolved);
    }
    if let Some(concrete_types) =
        concrete_flows.get(&(owner.to_string(), receiver_type.to_string()))
    {
        for concrete_type in concrete_types {
            if let Some(resolved) = lookup_member(concrete_type, member, symbol_kinds, class_map) {
                if !targets.contains(&resolved) {
                    targets.push(resolved);
                }
            }
        }
    }
    targets
}

fn lookup_member(
    class_name: &str,
    member: &str,
    symbol_kinds: &HashMap<String, SymbolKind>,
    class_map: &HashMap<String, ClassInfo>,
) -> Option<String> {
    lookup_member_inner(
        class_name,
        member,
        symbol_kinds,
        class_map,
        &mut HashSet::new(),
    )
}

fn lookup_member_inner(
    class_name: &str,
    member: &str,
    symbol_kinds: &HashMap<String, SymbolKind>,
    class_map: &HashMap<String, ClassInfo>,
    visited: &mut HashSet<String>,
) -> Option<String> {
    if !visited.insert(class_name.to_string()) {
        return None;
    }
    let direct = format!("{class_name}.{member}");
    if symbol_kinds.contains_key(&direct) {
        return Some(direct);
    }
    let class_info = class_map.get(class_name)?;
    for base in &class_info.bases {
        if let Some(target) =
            lookup_member_inner(&base.base, member, symbol_kinds, class_map, visited)
        {
            return Some(target);
        }
    }
    None
}

fn is_subclass_or_same(
    concrete_type: &str,
    base_type: &str,
    class_map: &HashMap<String, ClassInfo>,
) -> bool {
    if concrete_type == base_type {
        return true;
    }
    is_subclass_inner(concrete_type, base_type, class_map, &mut HashSet::new())
}

fn is_subclass_inner(
    concrete_type: &str,
    base_type: &str,
    class_map: &HashMap<String, ClassInfo>,
    visited: &mut HashSet<String>,
) -> bool {
    if !visited.insert(concrete_type.to_string()) {
        return false;
    }
    let Some(class_info) = class_map.get(concrete_type) else {
        return false;
    };
    class_info.bases.iter().any(|base| {
        base.base == base_type || is_subclass_inner(&base.base, base_type, class_map, visited)
    })
}

fn mark_symbol_owners_live(live: &mut HashSet<String>, symbol_kinds: &HashMap<String, SymbolKind>) {
    let owners = live
        .iter()
        .filter_map(|symbol| {
            let kind = symbol_kinds.get(symbol)?;
            if !matches!(
                kind,
                SymbolKind::Method | SymbolKind::Attribute | SymbolKind::Field
            ) {
                return None;
            }
            let (owner, _) = symbol.rsplit_once('.')?;
            symbol_kinds
                .get(owner)
                .is_some_and(|kind| *kind == SymbolKind::Class)
                .then(|| owner.to_string())
        })
        .collect::<Vec<_>>();
    live.extend(owners);
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
#[path = "reachability_tests.rs"]
mod reachability_tests;

#[cfg(test)]
#[path = "reachability_entrypoint_tests.rs"]
mod reachability_entrypoint_tests;
