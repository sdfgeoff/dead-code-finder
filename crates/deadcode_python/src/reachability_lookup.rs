use std::collections::{HashMap, HashSet, VecDeque};

use deadcode_core::SymbolKind;

use crate::symbol_index::{ClassInfo, SymbolIndex};

use super::reachability_concrete_flow::concrete_flow_candidates;

pub(super) fn root_symbols(index: &SymbolIndex, root_group: &str) -> HashSet<String> {
    index
        .modules
        .iter()
        .flat_map(|module| &module.root_symbols)
        .filter(|root| root.group == root_group)
        .map(|root| root.symbol.clone())
        .collect()
}

pub(super) fn push_live(target: &str, live: &mut HashSet<String>, queue: &mut VecDeque<String>) {
    if live.insert(target.to_string()) {
        queue.push_back(target.to_string());
    }
}

pub(super) fn requeue_live_init_dependent_methods(
    callee: &str,
    symbol_kinds: &HashMap<String, SymbolKind>,
    live: &HashSet<String>,
    queue: &mut VecDeque<String>,
) {
    let Some(class_name) = callee.strip_suffix(".__init__") else {
        return;
    };
    let method_prefix = format!("{class_name}.");
    for (symbol, kind) in symbol_kinds {
        if *kind == SymbolKind::Method
            && symbol.starts_with(&method_prefix)
            && live.contains(symbol)
        {
            queue.push_back(symbol.clone());
        }
    }
}

pub(super) fn resolve_member_targets(
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

pub(crate) fn lookup_member(
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

pub(crate) fn is_subclass_or_same(
    concrete_type: &str,
    base_type: &str,
    class_map: &HashMap<String, ClassInfo>,
    symbol_kinds: &HashMap<String, SymbolKind>,
) -> bool {
    if concrete_type == base_type {
        return true;
    }
    is_subclass_inner(concrete_type, base_type, class_map, &mut HashSet::new())
        || super::reachability_protocol::structurally_implements_protocol(
            concrete_type,
            base_type,
            class_map,
            symbol_kinds,
        )
}

pub(crate) fn class_derives_from(
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
