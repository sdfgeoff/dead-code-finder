use std::collections::{HashMap, HashSet};

use deadcode_core::SymbolKind;

use crate::symbol_index::{ClassInfo, FunctionSignature, TypeBinding};

use super::reachability_concrete_flow::{concrete_flow_base, concrete_flow_candidates};

pub(super) fn propagate_concrete_return_flows(
    owner: &str,
    target: &str,
    signature_map: &HashMap<String, FunctionSignature>,
    symbol_kinds: &HashMap<String, SymbolKind>,
    class_map: &HashMap<String, ClassInfo>,
    concrete_flows: &mut HashMap<(String, String), HashSet<String>>,
) -> bool {
    let Some((receiver_type, member)) = target.rsplit_once('.') else {
        return false;
    };
    let Some(static_member) = super::lookup_member(receiver_type, member, symbol_kinds, class_map)
    else {
        return false;
    };
    let Some(static_return) = signature_return_type(signature_map.get(static_member.as_str()))
    else {
        return false;
    };
    let Some((return_base, requires_subclass_check)) = concrete_flow_base(static_return) else {
        return false;
    };
    let concrete_receivers = concrete_flow_candidates(owner, receiver_type, concrete_flows)
        .into_iter()
        .flat_map(|types| types.iter().cloned())
        .collect::<Vec<_>>();
    let mut changed = false;
    for concrete_receiver in concrete_receivers {
        let Some(concrete_member) =
            super::lookup_member(&concrete_receiver, member, symbol_kinds, class_map)
        else {
            continue;
        };
        if concrete_member == static_member {
            continue;
        }
        let Some(concrete_return) =
            signature_return_type(signature_map.get(concrete_member.as_str()))
        else {
            continue;
        };
        let Some((concrete_return_base, _)) = concrete_flow_base(concrete_return) else {
            continue;
        };
        if requires_subclass_check
            && !super::is_subclass_or_same(
                concrete_return_base,
                return_base,
                class_map,
                symbol_kinds,
            )
        {
            continue;
        }
        let concrete_types = concrete_flows
            .entry((owner.to_string(), return_base.clone()))
            .or_default();
        changed |= concrete_types.insert(concrete_return_base.clone());
    }
    changed
}

fn signature_return_type(signature: Option<&FunctionSignature>) -> Option<&TypeBinding> {
    let signature = signature?;
    signature
        .concrete_return_type
        .as_ref()
        .or(signature.return_type.as_ref())
}
