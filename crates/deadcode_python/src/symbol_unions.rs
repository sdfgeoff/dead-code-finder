use crate::symbol_index::TypeBinding;

pub(super) fn flattened_union_members(binding: &TypeBinding) -> Vec<TypeBinding> {
    if !is_union_type(&binding.base) {
        return vec![binding.clone()];
    }
    binding
        .args
        .iter()
        .flat_map(flattened_union_members)
        .collect()
}

pub(super) fn is_union_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "typing.Union" | "types.UnionType" | "typing.Optional" | "Optional"
    ) || type_name.ends_with(".Union")
        || type_name.ends_with(".Optional")
}

pub(super) fn is_none_type(type_name: &str) -> bool {
    matches!(type_name, "None" | "builtins.None")
        || type_name.ends_with(".None")
        || type_name.ends_with(".NoneType")
}
