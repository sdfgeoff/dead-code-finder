use std::collections::{HashMap, HashSet};

use deadcode_core::SymbolKind;

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
