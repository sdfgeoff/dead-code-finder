use std::collections::HashMap;

use crate::symbol_index::TypeBinding;

pub(super) fn type_var_from_type_argument(annotation: &TypeBinding) -> Option<&str> {
    if !matches!(
        annotation.base.as_str(),
        "typing.Type" | "typing_extensions.Type" | "Type"
    ) && !annotation.base.ends_with(".Type")
    {
        return None;
    }
    annotation.args.first().map(|arg| arg.base.as_str())
}

pub(super) fn type_object_arg(binding: &TypeBinding) -> Option<TypeBinding> {
    if !matches!(
        binding.base.as_str(),
        "typing.Type" | "typing_extensions.Type" | "Type"
    ) && !binding.base.ends_with(".Type")
    {
        return None;
    }
    binding.args.first().cloned()
}

pub(super) fn collect_type_var_substitutions(
    pattern: &TypeBinding,
    value: &TypeBinding,
    substitutions: &mut HashMap<String, TypeBinding>,
) {
    if pattern.args.is_empty() {
        if pattern.base != value.base && is_type_var_like(&pattern.base) {
            substitutions.insert(pattern.base.clone(), value.clone());
        }
        return;
    }
    if pattern.args.len() != value.args.len() {
        return;
    }
    for (pattern_arg, value_arg) in pattern.args.iter().zip(&value.args) {
        collect_type_var_substitutions(pattern_arg, value_arg, substitutions);
    }
}

fn is_type_var_like(type_name: &str) -> bool {
    let name = type_name.rsplit('.').next().unwrap_or(type_name);
    name.starts_with('T') || name.ends_with("Type")
}

pub(super) fn substitute_type_vars(
    binding: &TypeBinding,
    substitutions: &HashMap<String, TypeBinding>,
) -> TypeBinding {
    if let Some(substitution) = substitutions.get(&binding.base) {
        return substitution.clone();
    }
    TypeBinding {
        base: binding.base.clone(),
        args: binding
            .args
            .iter()
            .map(|arg| substitute_type_vars(arg, substitutions))
            .collect(),
        external: binding.external,
    }
}
