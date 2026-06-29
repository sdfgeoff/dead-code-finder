use std::collections::{HashMap, HashSet, VecDeque};

use deadcode_core::SymbolKind;

use crate::symbol_index::{
    CallableReturnMemberUse, CallableReturnOverride, ClassInfo, FunctionReturnCall,
};

pub(super) fn process_callable_return_flows(
    owner: &str,
    live: &HashSet<String>,
    queue: &mut VecDeque<String>,
    flows: &mut HashMap<String, HashSet<String>>,
    overrides: &[CallableReturnOverride],
    member_uses: &[CallableReturnMemberUse],
    return_calls: &[FunctionReturnCall],
    symbol_kinds: &HashMap<String, SymbolKind>,
    class_map: &HashMap<String, ClassInfo>,
) -> Vec<String> {
    for callable_override in overrides
        .iter()
        .filter(|callable_override| callable_override.from == owner)
    {
        let concrete_types = flows
            .entry(callable_override.target_callable.clone())
            .or_default();
        if concrete_types.insert(callable_override.concrete_type.clone()) {
            requeue_dependents(
                &callable_override.target_callable,
                live,
                queue,
                member_uses,
                return_calls,
            );
        }
    }

    for return_call in return_calls
        .iter()
        .filter(|return_call| return_call.function == owner)
    {
        let concrete_types = flows
            .get(&return_call.callable)
            .cloned()
            .unwrap_or_default();
        for concrete_type in concrete_types {
            let target_types = flows.entry(owner.to_string()).or_default();
            if target_types.insert(concrete_type) {
                requeue_dependents(owner, live, queue, member_uses, return_calls);
            }
        }
    }

    let mut targets = Vec::new();
    for member_use in member_uses
        .iter()
        .filter(|member_use| member_use.from == owner)
    {
        let Some(concrete_types) = flows.get(&member_use.callable) else {
            continue;
        };
        for concrete_type in concrete_types {
            if let Some(target) =
                super::lookup_member(concrete_type, &member_use.member, symbol_kinds, class_map)
            {
                targets.push(target);
            }
        }
    }
    targets
}

fn requeue_dependents(
    callable: &str,
    live: &HashSet<String>,
    queue: &mut VecDeque<String>,
    member_uses: &[CallableReturnMemberUse],
    return_calls: &[FunctionReturnCall],
) {
    for member_use in member_uses
        .iter()
        .filter(|member_use| member_use.callable == callable && live.contains(&member_use.from))
    {
        queue.push_back(member_use.from.clone());
    }
    for return_call in return_calls.iter().filter(|return_call| {
        return_call.callable == callable && live.contains(&return_call.function)
    }) {
        queue.push_back(return_call.function.clone());
    }
}
