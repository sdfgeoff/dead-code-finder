use std::collections::HashMap;

use crate::symbol_index::{FunctionSignature, TypeBinding, ValueBinding};

pub(super) fn callable_alias_target(
    module: &str,
    name: &str,
    types: &HashMap<String, TypeBinding>,
    values: &[ValueBinding],
    signatures: &[FunctionSignature],
) -> Option<String> {
    let binding = types
        .get(name)
        .cloned()
        .or_else(|| module_value_binding(module, name, values))?;
    signatures
        .iter()
        .any(|signature| signature.function == binding.base)
        .then_some(binding.base)
}

pub(super) fn module_value_binding(
    module: &str,
    name: &str,
    values: &[ValueBinding],
) -> Option<TypeBinding> {
    let qualified_name = format!("{module}.{name}");
    values
        .iter()
        .find(|value| value.qualified_name == qualified_name)
        .map(|value| value.binding.clone())
}
