use std::collections::{HashMap, HashSet};

use crate::symbol_index::TypeBinding;

pub(super) fn concrete_flow_base(annotation: &TypeBinding) -> Option<(&String, bool)> {
    if is_union_type(&annotation.base) {
        return annotation
            .args
            .iter()
            .filter(|arg| !is_none_type(&arg.base))
            .find_map(concrete_flow_base);
    }
    if is_type_object(&annotation.base) {
        return annotation.args.first().map(|arg| (&arg.base, false));
    }
    if is_callable_type(&annotation.base) {
        return annotation.args.last().map(|arg| (&arg.base, true));
    }
    if is_collection_type(&annotation.base) {
        return annotation.args.first().map(|arg| (&arg.base, true));
    }
    Some((&annotation.base, true))
}

pub(super) fn concrete_flow_candidates<'a>(
    owner: &str,
    receiver_type: &str,
    concrete_flows: &'a HashMap<(String, String), HashSet<String>>,
) -> Vec<&'a HashSet<String>> {
    let mut candidates = Vec::new();
    let mut scope = Some(owner);
    while let Some(current) = scope {
        if let Some(concrete_types) =
            concrete_flows.get(&(current.to_string(), receiver_type.to_string()))
        {
            candidates.push(concrete_types);
        }
        if let Some(init_owner) = owner_init_method(current) {
            if let Some(concrete_types) =
                concrete_flows.get(&(init_owner, receiver_type.to_string()))
            {
                candidates.push(concrete_types);
            }
        }
        scope = current.rsplit_once('.').map(|(parent, _)| parent);
    }
    candidates
}

fn owner_init_method(owner: &str) -> Option<String> {
    let (class_name, method_name) = owner.rsplit_once('.')?;
    (method_name != "__init__").then(|| format!("{class_name}.__init__"))
}

fn is_union_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "typing.Union" | "typing.Optional" | "Union" | "Optional"
    ) || type_name.ends_with(".Union")
        || type_name.ends_with(".Optional")
}

fn is_none_type(type_name: &str) -> bool {
    matches!(type_name, "None" | "NoneType" | "types.NoneType")
}

fn is_collection_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "list"
            | "set"
            | "tuple"
            | "typing.List"
            | "typing.Set"
            | "typing.Sequence"
            | "collections.abc.Sequence"
    )
}

fn is_type_object(type_name: &str) -> bool {
    matches!(
        type_name,
        "type" | "typing.Type" | "typing_extensions.Type" | "Type"
    ) || type_name.ends_with(".Type")
}

fn is_callable_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "typing.Callable" | "collections.abc.Callable" | "Callable"
    )
}
