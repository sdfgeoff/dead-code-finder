use std::collections::{HashMap, HashSet, VecDeque};

use deadcode_core::{Diagnostic, Finding, Severity, SymbolKind};

use crate::symbol_index::{AccessKind, ClassInfo, SymbolIndex};

use self::reachability_class_metadata::{
    mark_configured_live_class_surfaces, mark_live_class_creation_metadata, mark_symbol_owners_live,
};
use self::reachability_concrete_flow::{concrete_flow_base, concrete_flow_candidates};
use self::reachability_concrete_return::propagate_concrete_return_flows;
use self::reachability_maps::{
    class_map, function_signature_map, imported_module_target, module_map, module_value_set,
    owner_module, pytest_fixture_map, resolve_reference, symbol_kind_map, symbol_module_map,
};

#[path = "reachability_class_metadata.rs"]
mod reachability_class_metadata;
#[path = "reachability_concrete_flow.rs"]
mod reachability_concrete_flow;
#[path = "reachability_concrete_return.rs"]
mod reachability_concrete_return;
#[path = "reachability_maps.rs"]
mod reachability_maps;
#[path = "reachability_protocol.rs"]
mod reachability_protocol;

pub fn find_unused_symbols(index: &SymbolIndex) -> Vec<Finding> {
    let live_by_group = compute_live_symbols_by_group(index);
    let counted_live = counted_live_symbols(index, &live_by_group);
    let mut findings = Vec::new();
    for module in &index.modules {
        for symbol in &module.symbols {
            if symbol.kind == SymbolKind::Module {
                continue;
            }
            if module.is_test {
                if !index.include_tests || symbol_reachable_from_any_group(&live_by_group, symbol) {
                    continue;
                }
                findings.push(Finding::unused(
                    code_for_kind(&symbol.kind),
                    symbol.qualified_name.clone(),
                    symbol.kind.clone(),
                    symbol.span.clone(),
                ));
            } else if !counted_live.contains(&symbol.qualified_name) {
                let reachable_from = reachable_from_groups(index, &live_by_group, symbol);
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
    let mut seen_symbols = HashSet::new();
    findings.retain(|finding| seen_symbols.insert(finding.symbol.clone()));
    findings
}

fn counted_live_symbols(
    index: &SymbolIndex,
    live_by_group: &HashMap<String, HashSet<String>>,
) -> HashSet<String> {
    let mut live = HashSet::new();
    for group in &index.counts_as_used_root_groups {
        if let Some(group_live) = live_by_group.get(group) {
            live.extend(group_live.iter().cloned());
        }
    }
    live
}

pub fn unresolved_receiver_diagnostics(index: &SymbolIndex) -> Vec<Diagnostic> {
    let live = compute_live_symbols(index, &index.primary_root_group);
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
    let live = compute_live_symbols(index, &index.primary_root_group);
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

fn compute_live_symbols_by_group(index: &SymbolIndex) -> HashMap<String, HashSet<String>> {
    root_group_names(index)
        .into_iter()
        .map(|group| {
            let live = compute_live_symbols(index, &group);
            (group, live)
        })
        .collect()
}

fn root_group_names(index: &SymbolIndex) -> Vec<String> {
    let mut names = index.root_groups.clone();
    if !names.contains(&index.primary_root_group) {
        names.insert(0, index.primary_root_group.clone());
    }
    names
}

fn symbol_reachable_from_any_group(
    live_by_group: &HashMap<String, HashSet<String>>,
    symbol: &crate::symbol_index::IndexedSymbol,
) -> bool {
    live_by_group
        .values()
        .any(|live| live.contains(&symbol.qualified_name))
}

fn reachable_from_groups(
    index: &SymbolIndex,
    live_by_group: &HashMap<String, HashSet<String>>,
    symbol: &crate::symbol_index::IndexedSymbol,
) -> Vec<String> {
    root_group_names(index)
        .into_iter()
        .filter(|group| group != &index.primary_root_group)
        .filter(|group| {
            live_by_group
                .get(group)
                .is_some_and(|live| live.contains(&symbol.qualified_name))
        })
        .collect()
}

fn compute_live_symbols(index: &SymbolIndex, root_group: &str) -> HashSet<String> {
    let symbol_modules = symbol_module_map(index);
    let symbol_kinds = symbol_kind_map(index);
    let module_values = module_value_set(index);
    let module_map = module_map(index);
    let class_map = class_map(index);
    let signature_map = function_signature_map(index);
    let fixture_map = pytest_fixture_map(index);
    let mut concrete_flows: HashMap<(String, String), HashSet<String>> = HashMap::new();
    let mut live = root_symbols(index, root_group);
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

        if class_derives_from(&owner, "pydantic.BaseModel", &class_map) {
            let model_config = format!("{owner}.model_config");
            if symbol_kinds.contains_key(&model_config) {
                push_live(&model_config, &mut live, &mut queue);
            }
        }

        if let Some(signature) = signature_map.get(owner.as_str()) {
            for parameter in &signature.parameters {
                if matches!(parameter.name.as_str(), "self" | "cls") {
                    continue;
                }
                if let Some(fixture) = fixture_map.get(parameter.name.as_str()) {
                    push_live(&fixture.function, &mut live, &mut queue);
                }
            }
        }

        for call_argument in module
            .call_argument_types
            .iter()
            .filter(|call_argument| call_argument.from == owner)
        {
            let Some(signature) = signature_map.get(call_argument.callee.as_str()) else {
                continue;
            };
            let Some(Some((base_type, requires_subclass_check))) = signature
                .parameters
                .get(call_argument.position)
                .map(|parameter| parameter.annotation.as_ref().and_then(concrete_flow_base))
            else {
                continue;
            };
            if requires_subclass_check
                && !is_subclass_or_same(
                    &call_argument.concrete_type,
                    base_type,
                    &class_map,
                    &symbol_kinds,
                )
            {
                continue;
            }
            let flow_key = (call_argument.callee.clone(), base_type.clone());
            let forwarded =
                concrete_flow_candidates(&owner, &call_argument.concrete_type, &concrete_flows)
                    .into_iter()
                    .flat_map(|types| types.iter().cloned())
                    .collect::<Vec<_>>();
            let concrete_candidates = if forwarded.is_empty() {
                vec![call_argument.concrete_type.clone()]
            } else {
                forwarded
            };
            for concrete_type in concrete_candidates {
                if requires_subclass_check
                    && !is_subclass_or_same(&concrete_type, base_type, &class_map, &symbol_kinds)
                {
                    continue;
                }
                let concrete_types = concrete_flows.entry(flow_key.clone()).or_default();
                if concrete_types.insert(concrete_type) && live.contains(&call_argument.callee) {
                    queue.push_back(call_argument.callee.clone());
                }
            }
        }

        for reference in module
            .references
            .iter()
            .filter(|reference| reference.from == owner)
        {
            if let Some(target) = resolve_reference(
                &owner,
                module,
                &reference.name,
                &symbol_kinds,
                &module_values,
            ) {
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
            if reference.access == AccessKind::Call
                && propagate_concrete_return_flows(
                    &owner,
                    &reference.target,
                    &signature_map,
                    &symbol_kinds,
                    &class_map,
                    &mut concrete_flows,
                )
            {
                queue.push_back(owner.clone());
            }
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
    mark_live_class_creation_metadata(&mut live, &symbol_kinds);
    mark_configured_live_class_surfaces(
        &mut live,
        &symbol_kinds,
        &class_map,
        &index.class_surfaces,
    );
    live
}

fn root_symbols(index: &SymbolIndex, root_group: &str) -> HashSet<String> {
    index
        .modules
        .iter()
        .flat_map(|module| &module.root_symbols)
        .filter(|root| root.group == root_group)
        .map(|root| root.symbol.clone())
        .collect()
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
    for concrete_types in concrete_flow_candidates(owner, receiver_type, concrete_flows) {
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

pub(super) fn lookup_member(
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

pub(super) fn is_subclass_or_same(
    concrete_type: &str,
    base_type: &str,
    class_map: &HashMap<String, ClassInfo>,
    symbol_kinds: &HashMap<String, SymbolKind>,
) -> bool {
    if concrete_type == base_type {
        return true;
    }
    is_subclass_inner(concrete_type, base_type, class_map, &mut HashSet::new())
        || reachability_protocol::structurally_implements_protocol(
            concrete_type,
            base_type,
            class_map,
            symbol_kinds,
        )
}

fn class_derives_from(
    concrete_type: &str,
    base_type: &str,
    class_map: &HashMap<String, ClassInfo>,
) -> bool {
    class_derives_from_inner(concrete_type, base_type, class_map, &mut HashSet::new())
}

fn class_derives_from_inner(
    concrete_type: &str,
    base_type: &str,
    class_map: &HashMap<String, ClassInfo>,
    visited: &mut HashSet<String>,
) -> bool {
    if concrete_type == base_type {
        return true;
    }
    if !visited.insert(concrete_type.to_string()) {
        return false;
    }
    let Some(class_info) = class_map.get(concrete_type) else {
        return false;
    };
    class_info
        .bases
        .iter()
        .any(|base| class_derives_from_inner(&base.base, base_type, class_map, visited))
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
