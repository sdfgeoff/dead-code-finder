use std::collections::HashMap;

use deadcode_core::SymbolKind;

use crate::symbol_index::ClassInfo;

pub(super) fn structurally_implements_protocol(
    concrete_type: &str,
    protocol_type: &str,
    class_map: &HashMap<String, ClassInfo>,
    symbol_kinds: &HashMap<String, SymbolKind>,
) -> bool {
    let Some(protocol_info) = class_map.get(protocol_type) else {
        return false;
    };
    if !super::class_derives_from(protocol_type, "typing.Protocol", class_map) {
        return false;
    }
    protocol_members(protocol_info, symbol_kinds).all(|member| {
        super::lookup_member(concrete_type, &member, symbol_kinds, class_map).is_some()
    })
}

fn protocol_members<'a>(
    protocol_info: &'a ClassInfo,
    symbol_kinds: &'a HashMap<String, SymbolKind>,
) -> impl Iterator<Item = String> + 'a {
    let prefix = format!("{}.", protocol_info.class);
    symbol_kinds.iter().filter_map(move |(symbol, kind)| {
        if !matches!(
            kind,
            SymbolKind::Method | SymbolKind::Attribute | SymbolKind::Field
        ) || !symbol.starts_with(&prefix)
        {
            return None;
        }
        let member = symbol.strip_prefix(&prefix)?;
        (!member.contains('.')).then(|| member.to_string())
    })
}
