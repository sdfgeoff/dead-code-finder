use std::collections::{HashMap, HashSet};

use deadcode_core::SymbolKind;

use crate::symbol_index::ClassInfo;

pub(super) fn mark_symbol_owners_live(
    live: &mut HashSet<String>,
    symbol_kinds: &HashMap<String, SymbolKind>,
) {
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

pub(super) fn mark_live_class_creation_metadata(
    live: &mut HashSet<String>,
    symbol_kinds: &HashMap<String, SymbolKind>,
) {
    let metadata = live
        .iter()
        .filter(|symbol| {
            symbol_kinds
                .get(symbol.as_str())
                .is_some_and(|kind| *kind == SymbolKind::Class)
        })
        .filter_map(|class| {
            let slots = format!("{class}.__slots__");
            symbol_kinds.contains_key(&slots).then_some(slots)
        })
        .collect::<Vec<_>>();
    live.extend(metadata);
}

pub(super) fn abc_override_methods_for_live_classes(
    live: &HashSet<String>,
    symbol_kinds: &HashMap<String, SymbolKind>,
    class_map: &HashMap<String, ClassInfo>,
) -> Vec<String> {
    let mut methods = Vec::new();
    for class in live_classes(live, symbol_kinds) {
        if !class_derives_from_abc(&class, class_map) {
            continue;
        }
        let Some(class_info) = class_map.get(&class) else {
            continue;
        };
        for method in direct_methods(&class, symbol_kinds) {
            if base_member(&method, class_info, symbol_kinds, class_map).is_some() {
                methods.push(format!("{class}.{method}"));
                methods.extend(base_members(&method, class_info, symbol_kinds, class_map));
            }
        }
    }
    methods
}

pub(super) fn mark_configured_live_class_surfaces(
    live: &mut HashSet<String>,
    symbol_kinds: &HashMap<String, SymbolKind>,
    class_map: &HashMap<String, ClassInfo>,
    class_surfaces: &[String],
) {
    if class_surfaces.is_empty() {
        return;
    }
    let surfaces = live
        .iter()
        .filter(|symbol| {
            symbol_kinds
                .get(symbol.as_str())
                .is_some_and(|kind| *kind == SymbolKind::Class)
        })
        .filter(|class| {
            class_surfaces
                .iter()
                .any(|base| class_derives_from(class, base, class_map))
        })
        .flat_map(|class| {
            let prefix = format!("{class}.");
            symbol_kinds
                .iter()
                .filter(move |(symbol, kind)| {
                    matches!(kind, SymbolKind::Field | SymbolKind::Attribute)
                        && symbol.starts_with(&prefix)
                        && symbol[prefix.len()..].find('.').is_none()
                })
                .map(|(symbol, _)| symbol.clone())
        })
        .collect::<Vec<_>>();
    live.extend(surfaces);
}

fn live_classes(live: &HashSet<String>, symbol_kinds: &HashMap<String, SymbolKind>) -> Vec<String> {
    live.iter()
        .filter(|symbol| {
            symbol_kinds
                .get(symbol.as_str())
                .is_some_and(|kind| *kind == SymbolKind::Class)
        })
        .cloned()
        .collect()
}

fn direct_methods(class: &str, symbol_kinds: &HashMap<String, SymbolKind>) -> Vec<String> {
    let prefix = format!("{class}.");
    symbol_kinds
        .iter()
        .filter(|(symbol, kind)| {
            **kind == SymbolKind::Method
                && symbol.starts_with(&prefix)
                && symbol[prefix.len()..].find('.').is_none()
        })
        .map(|(symbol, _)| symbol[prefix.len()..].to_string())
        .collect()
}

fn base_member(
    member: &str,
    class_info: &ClassInfo,
    symbol_kinds: &HashMap<String, SymbolKind>,
    class_map: &HashMap<String, ClassInfo>,
) -> Option<String> {
    for base in &class_info.bases {
        let direct = format!("{}.{}", base.base, member);
        if symbol_kinds.contains_key(&direct) {
            return Some(direct);
        }
        let Some(base_info) = class_map.get(&base.base) else {
            continue;
        };
        if let Some(found) = base_member(member, base_info, symbol_kinds, class_map) {
            return Some(found);
        }
    }
    None
}

fn base_members(
    member: &str,
    class_info: &ClassInfo,
    symbol_kinds: &HashMap<String, SymbolKind>,
    class_map: &HashMap<String, ClassInfo>,
) -> Vec<String> {
    let mut members = Vec::new();
    for base in &class_info.bases {
        let direct = format!("{}.{}", base.base, member);
        if symbol_kinds.contains_key(&direct) {
            members.push(direct);
        }
        if let Some(base_info) = class_map.get(&base.base) {
            members.extend(base_members(member, base_info, symbol_kinds, class_map));
        }
    }
    members
}

fn class_derives_from_abc(class: &str, class_map: &HashMap<String, ClassInfo>) -> bool {
    class_derives_from_abc_inner(class, class_map, &mut HashSet::new())
}

fn class_derives_from_abc_inner(
    class: &str,
    class_map: &HashMap<String, ClassInfo>,
    visited: &mut HashSet<String>,
) -> bool {
    if !visited.insert(class.to_string()) {
        return false;
    }
    let Some(class_info) = class_map.get(class) else {
        return false;
    };
    class_info.bases.iter().any(|base| {
        is_abc_base(&base.base) || class_derives_from_abc_inner(&base.base, class_map, visited)
    })
}

fn is_abc_base(base: &str) -> bool {
    matches!(base, "abc.ABC" | "ABC" | "abc.ABCMeta" | "ABCMeta")
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
